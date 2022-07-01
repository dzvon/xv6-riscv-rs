use core::arch::asm;

use crate::{main, memlayout::*, param::NCPU, riscv::*};

#[repr(C, align(16))]
struct Stack([u8; 4096 * NCPU]);

// entry.S needs one stack per CPU.
#[export_name = "stack0"]
static mut STACK0: Stack = Stack([0; 4096 * NCPU]);

// a scratch area per CPU for machine-mode timer interrupts.
static mut TIMER_SCRATCH: [[u64; 5]; NCPU] = [[0; 5]; NCPU];

extern "C" {
    // assembly code in kernelvec.S for machine-mode timer interrupt.
    fn timervec();
}

// entry.S jumps here in machine mode on stack0.
#[no_mangle]
pub extern "C" fn start() {
    // set M Previous Privilege mode to Supervisor, for mret.
    let mut x = r_mstatus();
    x &= !MSTATUS_MPP_MASK;
    x |= MSTATUS_MPP_S;
    w_mstatus(x);

    // set M Exception Program Counter to main, for mret.
    // requires gcc -mcmodel=medany
    w_mepc(main as usize);

    // disable paging for now.
    w_satp(0);

    // delegate all interrupts and exceptions to supervisor mode.
    w_medeleg(0xffff);
    w_mideleg(0xffff);
    w_sie(r_sie() | SIP_SEIE | SIP_STIE | SIP_SSIE);

    // configure Physical Memory Protection to give supervisor mode
    // access to all of physical memory.
    w_pmpaddr0(0x3fffffffffffff);
    w_pmpcfg0(0xf);

    // ask for clock interrupts.
    timerinit();

    // keep each CPU's hartid in its tp register, for cpuid().
    let id = r_mhartid();
    w_tp(id);

    // switch to supervisor mode and jump to main().
    unsafe {
        asm!("mret");
    }
}

/// set up to receive timer interrupts in machine mode,
/// which arrive at timervec in kernelvec.S,
/// which turns them into software interrupts for
/// devintr() in trap.rs.
fn timerinit() {
    // each CPU has a separate source of timer interrupts.
    let id = r_mhartid();

    // ask the CLINT for a timer interrupt.
    let interval = 100_0000; // cycles; about 1/10th second in qemu.
    unsafe {
        *(clint_mtimecmp(id) as *mut u64) = *(CLINT_MTIME as *const u64) + interval;
    }

    // prepare information in scratch[] for timervec.
    // scratch[0..2] : space for timervec to save registers.
    // scratch[3] : address of CLINT MTIMECMP register.
    // scratch[4] : desired interval (in cycles) between timer interrupts.
    unsafe {
        let scratch = &mut TIMER_SCRATCH[id];
        scratch[3] = clint_mtimecmp(id) as u64;
        scratch[4] = interval;
        w_mscratch(scratch.as_ptr() as usize);
    }

    // set the machine-mode trap handler
    w_mtvec(timervec as usize);

    // enable machine-mode interrupts.
    w_mstatus(r_mstatus() | MSTATUS_MIE);

    // enable machine-mode timer interrupts.
    w_mie(r_mie() | MIE_MTIE);
}
