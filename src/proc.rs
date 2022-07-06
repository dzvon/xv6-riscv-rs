use crate::{
    file::{File, Inode},
    param::{NCPU, NOFILE, NPROC},
    println,
    spinlock::{guard_lock, pop_off, push_off, SpinMutex, SpinMutexGuard},
};
use core::{
    cell::{Ref, UnsafeCell},
    ptr,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::riscv::*;
use alloc::{
    boxed::Box,
    string::String,
    sync::{Arc, Weak},
};

pub static CPUS: Cpus = {
    const CPU: UnsafeCell<Cpu> = UnsafeCell::new(Cpu::new());
    Cpus([CPU; NCPU])
};
pub static PROCS: ProcList = {
    const PROC: Proc = Proc::default();
    ProcList {
        list: [PROC; NPROC],
        wait_lock: SpinMutex::new("wait_lock", ()),
    }
};

extern "C" {
    fn swtch(old: *const Context, new: *mut Context);
}

pub struct Cpus([UnsafeCell<Cpu>; NCPU]);
unsafe impl Sync for Cpus {}

pub struct ProcList {
    list: [Proc; NPROC],

    // helps ensure that wakeups of wait()ing
    // parents are not lost. helps obey the
    // memory model when using p->parent.
    // must be acquired before any p->lock.
    wait_lock: SpinMutex<()>,
}
// unsafe impl Sync for ProcList {}

impl ProcList {
    pub fn alloc_pid() -> usize {
        static NEXT_PID: AtomicUsize = AtomicUsize::new(1);
        NEXT_PID.fetch_add(1, Ordering::Relaxed)
    }

    /// Wake up all processes sleeping on chan.
    /// Must be called without any p->lock.
    pub fn wakeup(&self, chan: usize) {
        for proc in &self.list {
            let myproc = CPUS.myproc();

            match myproc {
                Some(myproc) => {
                    if !ptr::eq(Arc::as_ptr(myproc), proc) {
                        let mut p = proc.control.lock();
                        if p.state == ProcState::Sleeping {
                            if let Some(proc_chan) = p.chan {
                                if proc_chan == chan {
                                    p.state = ProcState::Runnable;
                                }
                            }
                        }
                    }
                }
                None => {}
            }
        }
    }

    /// Atomically release lock and sleep on chan.
    /// Reacquires lock when awakened.
    pub fn sleep<T>(&self, chan: usize, lock: &SpinMutexGuard<T>) {
        match CPUS.myproc() {
            Some(p) => {
                // Must acquire p->lock in order to
                // change p->state and then call sched.
                // Once we hold p->lock, we can be
                // guaranteed that we won't miss any wakeup
                // (wakeup locks p->lock),
                // so it's okay to release lk.
                let mut proc_ctrl = p.control.lock();
                let mutex = guard_lock(lock);
                unsafe {
                    mutex.force_unlock();
                }

                // Go to sleep.
                proc_ctrl.chan = Some(chan);
                proc_ctrl.state = ProcState::Sleeping;

                ProcList::sched(&proc_ctrl);

                // Tidy up.
                proc_ctrl.chan = None;

                // Reacquire original lock.
                drop(proc_ctrl);
                core::mem::forget(mutex.lock());
            }
            None => {}
        }
    }

    /// Switch to scheduler.  Must hold only p->lock
    /// and have changed proc->state. Saves and restores
    /// intena because intena is a property of this
    /// kernel thread, not this CPU. It should
    /// be proc->intena and proc->noff, but that would
    /// break in the few places where a lock is held but
    /// there's no process.
    pub fn sched(proc: &SpinMutexGuard<ProcControl>) {
        let myproc = CPUS.myproc();

        if let Some(myproc) = myproc {
            if !myproc.control.holding() {
                panic!("sched: not holding p->lock");
            }
            if CPUS.mycpu().noff != 1 {
                panic!("sched: cpus.mycpu.noff != 1");
            }
            if proc.state == ProcState::Running {
                panic!("sched: proc is running");
            }
            if intr_get() {
                panic!("sched: interrupts are enabled");
            }

            let intena = CPUS.mycpu().intena;
            unsafe {
                swtch(
                    ptr::addr_of!(myproc.context),
                    ptr::addr_of_mut!(CPUS.mycpu().context),
                );
            }
            CPUS.mycpu().intena = intena;
        }
    }

    /// Print a process listing to console.  For debugging.
    /// Runs when user types ^P on console.
    /// No lock to avoid wedging a stuck machine further.
    pub fn proc_dump(&self) {
        for p in &self.list {
            let (pid, state) = {
                let proc = p.control.lock();
                (proc.pid, proc.state)
            };
            if matches!(state, ProcState::Unused) {
                continue;
            }
            println!("{} {:8?} {}", pid, state, p.name);
        }
    }
}

impl Cpus {
    /// Return this CPU's mutable reference.
    /// Interrupts must be disabled.
    ///
    /// # Safety
    ///
    /// Interrupts must be disabled when calling this function.
    /// We should make sure that only one CPU can access the corresponding
    /// CPUS[CPUS] element at a time, so there is no data race.
    pub fn mycpu(&self) -> &mut Cpu {
        let id = cpuid();
        unsafe { &mut *self.0[id].get() }
    }

    pub fn myproc(&self) -> Option<&Arc<Proc>> {
        push_off();
        let p = self.mycpu().proc.as_ref();
        pop_off();
        p
    }
}

/// Saved registers for kernel context switches.
#[repr(C)]
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
    pub noff: usize,             // Depth of push_off() nesting.
    pub intena: bool,            // Were interrupts enabled before push_off()?
}

impl Cpu {
    const fn new() -> Cpu {
        Cpu {
            proc: None,
            context: Context::default(),
            noff: 0,
            intena: false,
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

pub struct ProcControl {
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

#[derive(Clone, Copy, Debug, PartialEq)]
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
