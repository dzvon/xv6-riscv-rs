use core::arch::asm;

// Which hart (core) is this?
#[inline(always)]
pub(crate) fn r_mhartid() -> usize {
    let mut hartid: usize;
    unsafe {
        asm!("csrr {}, mhartid", out(reg) hartid);
    }
    hartid
}

// Machine Status Register, mstatus
pub(crate) const MSTATUS_MPP_MASK: usize = 3 << 11; // previous mode.
pub(crate) const MSTATUS_MPP_M: usize = 3 << 11;
pub(crate) const MSTATUS_MPP_S: usize = 1 << 11;
pub(crate) const MSTATUS_MPP_U: usize = 0 << 11;
pub(crate) const MSTATUS_MIE: usize = 1 << 3; // machine-mode interrupt enable.

#[inline(always)]
pub(crate) fn r_mstatus() -> usize {
    let mut mstatus: usize;
    unsafe {
        asm!("csrr {}, mstatus", out(reg) mstatus);
    }
    mstatus
}

#[inline(always)]
pub(crate) fn w_mstatus(mstatus: usize) {
    unsafe {
        asm!("csrw mstatus, {}", in(reg) mstatus);
    }
}

// machine exception program counter, holds the
// instruction address to which a return from
// exception will go.
#[inline(always)]
pub(crate) fn w_mepc(mepc: usize) {
    unsafe {
        asm!("csrw mepc, {}", in(reg) mepc);
    }
}

// Supervisor Status Register, sstatus
pub(crate) const SSTATUS_SPP: usize = 1 << 8; // Previous mode, 1 = Supervisor, 0 = User.
pub(crate) const SSTATUS_SPIE: usize = 1 << 5; // Supervisor Previous Interrupt Enable
pub(crate) const SSTATUS_UPIE: usize = 1 << 4; // User Previous Interrupt Enable
pub(crate) const SSTATUS_SIE: usize = 1 << 1; // Supervisor Interrupt Enable
pub(crate) const SSTATUS_UIE: usize = 1 << 0; // User Interrupt Enable

#[inline(always)]
pub(crate) fn r_sstatus() -> usize {
    let mut sstatus: usize;
    unsafe {
        asm!("csrr {}, sstatus", out(reg) sstatus);
    }
    sstatus
}

#[inline(always)]
pub(crate) fn w_sstatus(sstatus: usize) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) sstatus);
    }
}

/// Supervisor Interrupt Pending
#[inline(always)]
pub(crate) fn r_sip() -> usize {
    let mut sip: usize;
    unsafe {
        asm!("csrr {}, sip", out(reg) sip);
    }
    sip
}

#[inline(always)]
pub(crate) fn w_sip(sip: usize) {
    unsafe {
        asm!("csrw sip, {}", in(reg) sip);
    }
}

// Supervisor Interrupt Enable
pub(crate) const SIP_SEIE: usize = 1 << 9; // external
pub(crate) const SIP_STIE: usize = 1 << 5; // timer
pub(crate) const SIP_SSIE: usize = 1 << 1; // software

#[inline(always)]
pub(crate) fn r_sie() -> usize {
    let mut sie: usize;
    unsafe {
        asm!("csrr {}, sie", out(reg) sie);
    }
    sie
}

#[inline(always)]
pub(crate) fn w_sie(sie: usize) {
    unsafe {
        asm!("csrw sie, {}", in(reg) sie);
    }
}

// Machine-mode Interrupt Enable
pub(crate) const MIE_MEIE: usize = 1 << 11; // external
pub(crate) const MIE_MTIE: usize = 1 << 7; // timer
pub(crate) const MIE_MSIE: usize = 1 << 3; // software

#[inline(always)]
pub(crate) fn r_mie() -> usize {
    let mut mie: usize;
    unsafe {
        asm!("csrr {}, mie", out(reg) mie);
    }
    mie
}

#[inline(always)]
pub(crate) fn w_mie(mie: usize) {
    unsafe {
        asm!("csrw mie, {}", in(reg) mie);
    }
}

// supervisor exception program counter, holds the
// instruction address to which a return from
// exception will go.
#[inline(always)]
pub(crate) fn w_sepc(sepc: usize) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) sepc);
    }
}

#[inline(always)]
pub(crate) fn r_sepc() -> usize {
    let mut sepc: usize;
    unsafe {
        asm!("csrr {}, sepc", out(reg) sepc);
    }
    sepc
}

// Machine Exception Delegation
#[inline(always)]
pub(crate) fn r_medeleg() -> usize {
    let mut medeleg: usize;
    unsafe {
        asm!("csrr {}, medeleg", out(reg) medeleg);
    }
    medeleg
}

#[inline(always)]
pub(crate) fn w_medeleg(medeleg: usize) {
    unsafe {
        asm!("csrw medeleg, {}", in(reg) medeleg);
    }
}

// Machine Interrupt Delegation
#[inline(always)]
pub(crate) fn r_mideleg() -> usize {
    let mut mideleg: usize;
    unsafe {
        asm!("csrr {}, mideleg", out(reg) mideleg);
    }
    mideleg
}

#[inline(always)]
pub(crate) fn w_mideleg(mideleg: usize) {
    unsafe {
        asm!("csrw mideleg, {}", in(reg) mideleg);
    }
}

// Supervisor Trap-Vector Base Address
// low two bits are mode.
#[inline(always)]
pub(crate) fn w_stvec(stvec: usize) {
    unsafe {
        asm!("csrw stvec, {}", in(reg) stvec);
    }
}

#[inline(always)]
pub(crate) fn r_stvec() -> usize {
    let mut stvec: usize;
    unsafe {
        asm!("csrr {}, stvec", out(reg) stvec);
    }
    stvec
}

// Machine-mode interrupt vector
#[inline(always)]
pub(crate) fn w_mtvec(mtvec: usize) {
    unsafe {
        asm!("csrw mtvec, {}", in(reg) mtvec);
    }
}

#[inline(always)]
pub(crate) fn w_pmpcfg0(pmpcfg0: usize) {
    unsafe {
        asm!("csrw pmpcfg0, {}", in(reg) pmpcfg0);
    }
}

#[inline(always)]
pub(crate) fn w_pmpaddr0(pmpaddr0: usize) {
    unsafe {
        asm!("csrw pmpaddr0, {}", in(reg) pmpaddr0);
    }
}

// use riscv's sv39 page table scheme.
pub(crate) const SATP_SV39: usize = 8 << 60;

pub(crate) const fn make_satp(pagetable: usize) -> usize {
    SATP_SV39 | pagetable >> 12
}

// supervisor address translation and protection;
// holds the address of the page table.
#[inline(always)]
pub(crate) fn w_satp(satp: usize) {
    unsafe {
        asm!("csrw satp, {}", in(reg) satp);
    }
}

#[inline(always)]
pub(crate) fn r_satp() -> usize {
    let mut satp: usize;
    unsafe {
        asm!("csrr {}, satp", out(reg) satp);
    }
    satp
}

// Supervisor Scratch register, for early trap handler in trampoline.S.
#[inline(always)]
pub(crate) fn w_sscratch(sscratch: usize) {
    unsafe {
        asm!("csrw sscratch, {}", in(reg) sscratch);
    }
}

#[inline(always)]
pub(crate) fn w_mscratch(mscratch: usize) {
    unsafe {
        asm!("csrw mscratch, {}", in(reg) mscratch);
    }
}

// Supervisor Trap Cause
#[inline(always)]
pub(crate) fn r_scause() -> usize {
    let mut scause: usize;
    unsafe {
        asm!("csrr {}, scause", out(reg) scause);
    }
    scause
}

// Supervisor Trap Value
#[inline(always)]
pub(crate) fn r_stval() -> usize {
    let mut stval: usize;
    unsafe {
        asm!("csrr {}, stval", out(reg) stval);
    }
    stval
}

// Machine-mode Counter-Enable
#[inline(always)]
pub(crate) fn w_mcounteren(mcounteren: usize) {
    unsafe {
        asm!("csrw mcounteren, {}", in(reg) mcounteren);
    }
}

#[inline(always)]
pub(crate) fn r_mcounteren() -> usize {
    let mut mcounteren: usize;
    unsafe {
        asm!("csrr {}, mcounteren", out(reg) mcounteren);
    }
    mcounteren
}

// Machine-mode cycle counter
#[inline(always)]
pub(crate) fn r_time() -> usize {
    let mut time: usize;
    unsafe {
        asm!("csrr {}, time", out(reg) time);
    }
    time
}

// Enable device interrupts
#[inline(always)]
pub(crate) fn intr_on() {
    w_sstatus(r_sstatus() | SSTATUS_SIE);
}

// Disable device interrupts
#[inline(always)]
pub(crate) fn intr_off() {
    w_sstatus(r_sstatus() & !SSTATUS_SIE);
}

// Are device interrupts enabled?
#[inline(always)]
pub(crate) fn intr_get() -> bool {
    r_sstatus() & SSTATUS_SIE != 0
}

#[inline(always)]
pub(crate) fn r_sp() -> usize {
    let mut sp: usize;
    unsafe {
        asm!("mv {}, sp", out(reg) sp);
    }
    sp
}

// Read and write tp, the thread pointer, which holds
// this core's hartid (core number), the index into cpus[].
#[inline(always)]
pub(crate) fn r_tp() -> usize {
    let mut tp: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) tp);
    }
    tp
}

#[inline(always)]
pub(crate) fn w_tp(tp: usize) {
    unsafe {
        asm!("mv tp, {}", in(reg) tp);
    }
}

#[inline(always)]
pub(crate) fn r_ra() -> usize {
    let mut ra: usize;
    unsafe {
        asm!("mv {}, ra", out(reg) ra);
    }
    ra
}

// Flush the TLB.
#[inline(always)]
pub(crate) fn sfence_vma() {
    // the zero, zero means flush all TLB entries.
    unsafe {
        asm!("sfence.vma zero, zero");
    }
}
