//! the riscv Platform Level Interrupt Controller (PLIC).

use crate::{
    memlayout::{plic_sclaim, plic_senable, plic_spriority, PLIC, UART0_IRQ, VIRTIO0_IRQ},
    proc::cpuid,
};

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

/// the riscv Platform Level Interrupt Controller (PLIC).
pub fn plic_init() {
    unsafe {
        // set desired IRQ priorities non-zero (otherwise disabled).
        core::ptr::write_volatile((PLIC + UART0_IRQ as u64 * 4) as *mut u32, 1);
        core::ptr::write_volatile((PLIC + VIRTIO0_IRQ as u64 * 4) as *mut u32, 1);
    }
}

pub fn plic_init_hart() {
    let hart = cpuid();

    unsafe {
        // set uart's enable bit for this hart's S-mode.
        core::ptr::write_volatile(
            plic_senable(hart) as *mut u32,
            (1 << UART0_IRQ) | (1 << VIRTIO0_IRQ),
        );
        // set this hart's S-mode priority threshold to 0.
        core::ptr::write_volatile(plic_spriority(hart) as *mut u32, 0);
    }
}
