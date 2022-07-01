use crate::{
    file::{File, Inode},
    param::{NCPU, NOFILE, NPROC},
    spinlock::SpinMutex,
};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::riscv::*;
use alloc::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
};

pub static CPUS: Cpus = {
    const CPU: Cpu = Cpu::new();
    Cpus([CPU; NCPU])
};
pub static PROCS: ProcList = {
    const PROC: Proc = Proc::default();
    ProcList {
        list: [PROC; NPROC],
        wait_lock: SpinMutex::new("wait_lock", ()),
    }
};

pub struct Cpus([Cpu; NCPU]);
unsafe impl Sync for Cpus {}

pub struct ProcList {
    list: [Proc; NPROC],

    // helps ensure that wakeups of wait()ing
    // parents are not lost. helps obey the
    // memory model when using p->parent.
    // must be acquired before any p->lock.
    wait_lock: SpinMutex<()>,
}
unsafe impl Sync for ProcList {}

impl ProcList {
    pub fn alloc_pid() -> usize {
        static NEXT_PID: AtomicUsize = AtomicUsize::new(1);
        NEXT_PID.fetch_add(1, Ordering::Relaxed)
    }
}

impl Cpus {
    pub fn mycpu(&self) -> &Cpu {
        let id = cpuid();
        &self.0[id]
    }

    pub fn a(&self) -> i32 {
        let b = 123;
        return b;
    }
}

/// Saved registers for kernel context switches.
pub struct Context {
    pub ra: usize,
    pub sp: usize,

    // callee-saved registers
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
}

impl Context {
    const fn default() -> Context {
        Context {
            ra: 0,
            sp: 0,

            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }
}

// Per-CPU state.
pub struct Cpu {
    pub proc: Option<Arc<Proc>>, // The process running on this cpu, or null.
    pub context: Context,        // swtch() here to enter scheduler().
    pub noff: AtomicUsize,       // Depth of push_off() nesting.
    pub intena: AtomicBool,      // Were interrupts enabled before push_off()?
}

impl Cpu {
    const fn new() -> Cpu {
        Cpu {
            proc: None,
            context: Context::default(),
            noff: AtomicUsize::new(0),
            intena: AtomicBool::new(false),
        }
    }
}

// Must be called with interrupts disabled,
// to prevent race with process being moved
// to a different CPU.
pub(crate) fn cpuid() -> usize {
    r_tp()
}

// Per-process state
pub struct Proc {
    control: SpinMutex<ProcControl>,

    // wait_lock must be held when using this:
    parent: Weak<Proc>, // The parent process

    // these are private to the process, so lock need not be held.
    kstack: u64,                       // Virtual address of kernel stack
    sz: u64,                           // Size of process memory (bytes)
    pagetable: usize,                  // User-level page table
    trapframe: Option<Box<TrapFrame>>, // data page for trampoline.S
    context: Context,                  // swtch() here to run process.
    file: [Option<Arc<File>>; NOFILE], // open files
    cwd: Option<Arc<Inode>>,           // current working directory
    name: String,                      // Process name.
}

impl const Default for Proc {
    fn default() -> Self {
        const FILE: Option<Arc<File>> = None;
        Proc {
            control: SpinMutex::new("proc", ProcControl::default()),
            parent: Weak::new(),
            kstack: 0,
            sz: 0,
            pagetable: 0,
            trapframe: None,
            context: Context::default(),
            file: [FILE; NOFILE],
            cwd: None,
            name: String::new(),
        }
    }
}

struct ProcControl {
    state: ProcState,    // Process state
    chan: Option<usize>, // If non-none, sleeping on channel chan.
    killed: bool,        // Has the process been killed?
    xstate: i32,         // Process exit status to be returned to parent's wait.
    pid: usize,          // Process ID.
}

impl const Default for ProcControl {
    fn default() -> Self {
        ProcControl {
            state: ProcState::Unused,
            chan: None,
            killed: false,
            xstate: 0,
            pid: 0,
        }
    }
}

enum ProcState {
    Unused,
    Used,
    Sleeping,
    Runnable,
    Running,
    Zombie,
}

// per-process data for the trap handling code in trampoline.S.
// sits in a page by itself just under the trampoline page in the
// user page table. not specially mapped in the kernel page table.
// the sscratch register points here.
// uservec in trampoline.S saves user registers in the trapframe,
// then initializes registers from the trapframe's
// kernel_sp, kernel_hartid, kernel_satp, and jumps to kernel_trap.
// usertrapret() and userret in trampoline.S set up
// the trapframe's kernel_*, restore user registers from the
// trapframe, switch to the user page table, and enter user space.
// the trapframe includes callee-saved user registers like s0-s11 because the
// return-to-user path via usertrapret() doesn't return through
// the entire kernel call stack.
#[repr(C)]
struct TrapFrame {
    kernel_satp: u64,   // kernel page table
    kernel_sp: u64,     // top of process's kernel stack
    kernel_trap: u64,   // usertrap()
    epc: u64,           // saved user program counter
    kernel_hartid: u64, // saved kernel tp
    ra: u64,            // saved user return address
    sp: u64,            // saved user stack pointer
    gp: u64,            // saved user global pointer
    tp: u64,            // saved user trap pointer
    t0: u64,
    t1: u64,
    t2: u64,
    s0: u64,
    s1: u64,
    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
    t3: u64,
    t4: u64,
    t5: u64,
    t6: u64,
}
