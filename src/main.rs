#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(const_for)]
#![feature(const_default_impls)]
#![feature(const_trait_impl)]
#![feature(const_weak_new)]
#![feature(sync_unsafe_cell)]

#[cfg(not(target_os = "none"))]
compile_error!("You are not using a cross-compiler, you will most certainly run into trouble");

use core::{
    arch::global_asm,
    sync::atomic::{AtomicBool, Ordering},
};

use proc::cpuid;

use crate::trap::trap_init_hart;

extern crate alloc;

mod console;
mod file;
mod kalloc;
mod memlayout;
mod param;
mod plic;
mod printf;
mod proc;
mod riscv;
mod spinlock;
mod start;
mod trap;
mod uart;
mod vm;

global_asm!(include_str!("asm/entry.S"));
global_asm!(include_str!("asm/kernelvec.S"));
global_asm!(include_str!("asm/trampoline.S"));
global_asm!(include_str!("asm/swtch.S"));

static STARTED: AtomicBool = AtomicBool::new(false);

// start() jumps here in supervisor mode on all CPUs.
#[no_mangle]
pub extern "C" fn main() -> ! {
    if cpuid() == 0 {
        uart::uart_init();
        println!("xv6-rs kernel is booting");
        kalloc::kinit(); // physical page allocator
        vm::kvminit(); // create kernel page table
        trap_init_hart();
        STARTED.store(true, Ordering::Release);
    } else {
        while !STARTED.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
        println!("Hart {} starting!", cpuid());
    }

    riscv::intr_on();

    loop {
        core::hint::spin_loop();
    }
}
