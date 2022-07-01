use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    memlayout::{UART0_IRQ, VIRTIO0_IRQ},
    plic::{plic_claim, plic_complete},
    proc::cpuid,
    riscv::*,
    uart::uart_intr,
};

static TICKS: AtomicUsize = AtomicUsize::new(0);

extern "C" {
    fn kernelvec();
}

// interrupts and exceptions from kernel code go here via kernelvec,
// on whatever the current kernel stack is.
#[no_mangle]
pub extern "C" fn kerneltrap() {
    let sepc = r_sepc();
    let sstatus = r_sstatus();
    let scause = r_scause();

    if (sstatus & SSTATUS_SPP) == 0 {
        panic!("kerneltrap: not from supervisor mode");
    }
    if intr_get() {
        panic!("kerneltrap: interrupts enabled");
    }

    match devintr() {
        Trap::Unknown => panic!("kerneltrap: unknown trap"),
        Trap::SoftwareInterrupt => {}
        Trap::ExternalInterrupt => {}
    }

    // the yield() may have caused some traps to occur,
    // so restore trap registers for use by kernelvec.S's sepc instruction.
    w_sepc(sepc);
    w_sstatus(sstatus);
}

// check if it's an external interrupt or software interrupt,
// and handle it.
// returns 2 if timer interrupt,
// 1 if other device,
// 0 if not recognized.
fn devintr() -> Trap {
    let scause = r_scause();

    if (scause & 0x8000000000000000) != 0 && ((scause & 0xff) == 9) {
        // this is a supervisor external interrupt, via PLIC.

        // irq indicates which device interrupted.
        let irq = plic_claim();

        match irq {
            UART0_IRQ => {
                // this is a UART interrupt.
                uart_intr();
            }
            VIRTIO0_IRQ => {
                // this is a virtio interrupt.
                // virtio_disk_intr();
            }
            _ => {
                // unknown interrupt
                if irq != 0 {
                    // println!("unknown interrupt irq: {}", irp);

                    // the PLIC allows each device to raise at most one
                    // interrupt at a time; tell the PLIC the device is
                    // now allowed to interrupt again.
                    plic_complete(irq);
                }
            }
        }

        Trap::ExternalInterrupt
    } else if scause == 0x8000000000000001 {
        // software interrupt from a machine-mode timer interrupt,
        // forwarded by timervec in kernelvec.S.

        if cpuid() == 0 {
            // this is the boot CPU.
            clock_intr();
        }

        // acknowledge the software interrupt by clearing
        // the SSIP bit in sip.
        w_sip(r_sip() & !2);

        Trap::SoftwareInterrupt
    } else {
        // not an interrupt we recognize.
        Trap::Unknown
    }
}

fn clock_intr() {
    // increment the number of ticks.
    TICKS.fetch_add(1, Ordering::Relaxed);
}

pub enum Trap {
    ExternalInterrupt,
    SoftwareInterrupt,
    Unknown,
}

pub fn trap_init_hart() {
    w_stvec(kernelvec as usize);
}
