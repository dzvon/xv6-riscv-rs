// Physical memory layout

// qemu -machine virt is set up like this,
// based on qemu's hw/riscv/virt.c:
//
// 00001000 -- boot ROM, provided by qemu
// 02000000 -- CLINT
// 0C000000 -- PLIC
// 10000000 -- uart0
// 10001000 -- virtio disk
// 80000000 -- boot ROM jumps here in machine mode
//             -kernel loads the kernel here
// unused RAM after 80000000.

// the kernel uses physical memory thus:
// 80000000 -- entry.S, then kernel text and data
// end -- start of kernel page allocation area
// PHYSTOP -- end RAM used by the kernel

use crate::riscv::{MAXVA, PGSIZE};

// qemu puts UART registers here in physical memory.
pub const UART0: u64 = 0x10000000;
pub const UART0_IRQ: u32 = 10;

// virtio mmio interface
pub const VIRTIO0: u64 = 0x10001000;
pub const VIRTIO0_IRQ: u32 = 1;

// core local interruptor (CLINT), which contains the timer.
pub const CLINT: usize = 0x200_0000;
pub const fn clint_mtimecmp(id: usize) -> usize {
    CLINT + 0x4000 + 8 * id
}
/// cycles since boot.
pub const CLINT_MTIME: usize = CLINT + 0xBFF8;

// qemu puts platform-level interrupt controller (PLIC) here.
pub const PLIC: u64 = 0x0C00_0000;
pub const PLIC_PRIORITY: u64 = PLIC + 0x0; // priority bits for each interrupt.
pub const PLIC_PENDING: u64 = PLIC + 0x1000; // pending bits for each interrupt.
pub const fn plic_menable(hart: usize) -> u64 {
    PLIC + 0x2000 + hart as u64 * 0x100
}
pub const fn plic_senable(hart: usize) -> u64 {
    PLIC + 0x2080 + hart as u64 * 0x100
}
pub const fn plic_mpriority(hart: usize) -> u64 {
    PLIC + 0x20_0000 + hart as u64 * 0x2000
}
pub const fn plic_spriority(hart: usize) -> u64 {
    PLIC + 0x20_1000 + hart as u64 * 0x2000
}
pub const fn plic_mclaim(hart: usize) -> u64 {
    PLIC + 0x20_0004 + hart as u64 * 0x2000
}
pub const fn plic_sclaim(hart: usize) -> u64 {
    PLIC + 0x20_1004 + hart as u64 * 0x2000
}

// the kernel expects there to be RAM
// for use by the kernel and user pages
// from physical address 0x80000000 to PHYSTOP.
pub(crate) const KERNBASE: u64 = 0x80000000;
pub(crate) const PHYSTOP: u64 = KERNBASE + 128 * 1024 * 1024;

// map the trampoline page to the highest address,
// in both user and kernel space.
pub(crate) const TRAMPOLINE: u64 = MAXVA - PGSIZE;
