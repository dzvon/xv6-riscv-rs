//! the riscv Platform Level Interrupt Controller (PLIC).

use crate::{memlayout::plic_sclaim, proc::cpuid};

/// ask the PLIC what interrupt we should serve.
pub fn plic_claim() -> u32 {
    let hart = cpuid();
    let irq = plic_sclaim(hart) as *const u32;
    unsafe { *irq }
}

/// tell the PLIC we've served this IRQ.
pub fn plic_complete(irq: u32) {
    let hart = cpuid();
    unsafe { (plic_sclaim(hart) as *mut u32).write(irq) }
}
