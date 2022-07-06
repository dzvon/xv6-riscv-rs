use bitflags::bitflags;
use core::{
    cell::UnsafeCell,
    ops::{Add, AddAssign, Index, IndexMut},
    ptr::{self, NonNull},
};

use crate::{
    memlayout::{KERNBASE, PHYSTOP, PLIC, TRAMPOLINE, UART0, VIRTIO0},
    riscv::{pa2pte, pg_index, pg_round_down, pte2pa, MAXVA, PGSIZE},
};

/// The kernel's page table.
static KERNEL_PAGE_TABLE: PageTablePtr = PageTablePtr::dangling();

extern "C" {
    static etext: [u8; 0]; // kernel.ld sets this to end of kernel code.
    static trampoline: [u8; 0]; // trampoline.S
}

// Initialize the one kernel_pagetable
pub fn kvminit() {
    unsafe {
        KERNEL_PAGE_TABLE.init(kvmmake());
    }
}

/// Make a direct-map page table for the kernel.
pub unsafe fn kvmmake() -> NonNull<PageTable> {
    let page_table = allocate_page_table()
        .expect("failed to allocate page table")
        .as_mut()
        .unwrap();

    // uart registers are mapped at 0x1000_0000.
    kvmmap(
        page_table,
        VirtAddr::new(UART0),
        PhysAddr::new(UART0),
        PGSIZE,
        PageTableEntryFlags::READABLE | PageTableEntryFlags::WRITABLE,
    );

    // virtio mmio disk interface
    kvmmap(
        page_table,
        VirtAddr::new(VIRTIO0),
        PhysAddr::new(VIRTIO0),
        PGSIZE,
        PageTableEntryFlags::READABLE | PageTableEntryFlags::WRITABLE,
    );

    // PLIC
    kvmmap(
        page_table,
        VirtAddr::new(PLIC),
        PhysAddr::new(PLIC),
        0x400000,
        PageTableEntryFlags::READABLE | PageTableEntryFlags::WRITABLE,
    );

    // map kernel text executable and read-only.
    kvmmap(
        page_table,
        VirtAddr::new(KERNBASE),
        PhysAddr::new(KERNBASE),
        ptr::addr_of!(etext) as u64 - KERNBASE,
        PageTableEntryFlags::READABLE | PageTableEntryFlags::EXECUTABLE,
    );

    // map kernel data and the physical RAM we'll make use of.
    kvmmap(
        page_table,
        VirtAddr::new(ptr::addr_of!(etext) as u64),
        PhysAddr::new(ptr::addr_of!(etext) as u64),
        PHYSTOP - ptr::addr_of!(etext) as u64,
        PageTableEntryFlags::READABLE | PageTableEntryFlags::WRITABLE,
    );

    // map the trampoline for trap entry/exit to
    // the highest virtual address in the kernel.
    kvmmap(
        page_table,
        VirtAddr::new(TRAMPOLINE),
        PhysAddr::new(ptr::addr_of!(trampoline) as u64),
        PGSIZE,
        PageTableEntryFlags::READABLE | PageTableEntryFlags::WRITABLE,
    );

    NonNull::new(page_table).unwrap()
}

/// add a mapping to the kernel page table.
/// only used when booting.
/// does not flush TLB or enable paging.
fn kvmmap(
    page_table: &mut PageTable,
    va: VirtAddr,
    pa: PhysAddr,
    size: u64,
    flags: PageTableEntryFlags,
) {
    map_pages(page_table, va, pa, size, flags).expect("kvmmap failed");
}

/// Create PTEs for virtual addresses starting at va that refer to
/// physical addresses starting at pa. va and size might not
/// be page-aligned. Returns 0 on success, -1 if walk() couldn't
/// allocate a needed page-table page.
fn map_pages(
    page_table: &mut PageTable,
    va: VirtAddr,
    mut pa: PhysAddr,
    size: u64,
    flags: PageTableEntryFlags,
) -> Result<(), MapToError> {
    assert!(size > 0, "map_pages: size must be > 0");

    let mut a = pg_round_down(va.as_u64());
    let last = pg_round_down(va.as_u64() + size - 1);

    loop {
        let pte = unsafe {
            walk(page_table, VirtAddr::new(a), true).ok_or(MapToError::FrameAllocationFailed)?
        };
        if pte.flags().contains(PageTableEntryFlags::VALID) {
            panic!("map_pages: page table entry already exists");
        }
        pte.set_addr(
            PhysAddr::new(pa2pte(pa.as_u64())),
            flags | PageTableEntryFlags::VALID,
        );
        if a == last {
            break;
        }
        a += PGSIZE;
        pa += PGSIZE;
    }

    Ok(())
}

/// Return the address of the PTE in page table pagetable
/// that corresponds to virtual address va.  If alloc!=0,
/// create any required page-table pages.
///
/// The risc-v Sv39 scheme has three levels of page-table
/// pages. A page-table page contains 512 64-bit PTEs.
/// A 64-bit virtual address is split into five fields:
///   39..63 -- must be zero.
///   30..38 -- 9 bits of level-2 index.
///   21..29 -- 9 bits of level-1 index.
///   12..20 -- 9 bits of level-0 index.
///    0..11 -- 12 bits of byte offset within the page.
unsafe fn walk(
    mut page_table: &mut PageTable,
    va: VirtAddr,
    alloc: bool,
) -> Option<&mut PageTableEntry> {
    assert!(va.as_u64() < MAXVA, "walk: va out of range");

    for level in (0..3).rev() {
        let pte = &mut page_table[pg_index(level, va.as_u64()) as usize];
        if pte.flags().contains(PageTableEntryFlags::VALID) {
            page_table = (pte2pa(pte.as_u64()) as *mut PageTable).as_mut().unwrap();
        } else {
            if !alloc {
                return None;
            }
            page_table = allocate_page_table()?.as_mut().unwrap();

            pte.set_addr(
                PhysAddr::new(pa2pte(page_table as *mut PageTable as u64)),
                PageTableEntryFlags::VALID,
            );
        }
    }

    Some(&mut page_table[pg_index(0, va.as_u64()) as usize])
}

fn allocate_page_table() -> Option<*mut PageTable> {
    let ptr = unsafe { alloc::alloc::alloc_zeroed(alloc::alloc::Layout::new::<PageTable>()) };

    if ptr.is_null() {
        None
    } else {
        Some(ptr as *mut PageTable)
    }
}

#[derive(Debug)]
pub enum MapToError {
    FrameAllocationFailed,
}

const ENTRY_COUNT: usize = 512;

#[repr(C)]
#[repr(align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    fn new() -> PageTable {
        PageTable {
            entries: [PageTableEntry::new(); ENTRY_COUNT],
        }
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    #[inline]
    fn index(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}

/// A 64-bit page table entry.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    fn new() -> PageTableEntry {
        PageTableEntry { entry: 0 }
    }

    pub const fn flags(&self) -> PageTableEntryFlags {
        PageTableEntryFlags::from_bits_truncate(self.entry)
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.entry
    }

    #[inline]
    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableEntryFlags) {
        self.entry = addr.as_u64() | flags.bits();
    }

    #[inline]
    pub fn set_flags(&mut self, flags: PageTableEntryFlags) {
        self.entry |= flags.bits();
    }
}

struct PageTablePtr(UnsafeCell<NonNull<PageTable>>);
unsafe impl Sync for PageTablePtr {}

impl PageTablePtr {
    const fn dangling() -> Self {
        PageTablePtr(UnsafeCell::new(NonNull::dangling()))
    }

    /// The init method should be called only once at boot time.
    unsafe fn init(&self, page_table: NonNull<PageTable>) {
        ptr::write(self.0.get(), page_table);
    }
}

bitflags! {
    pub struct PageTableEntryFlags: u64 {
        const VALID = 1 << 0;
        const READABLE = 1 << 1;
        const WRITABLE = 1 << 2;
        const EXECUTABLE = 1 << 3;
        /// User can access the page.
        const USER = 1 << 4;
    }
}

#[repr(transparent)]
pub struct VirtAddr(u64);

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct PhysAddr(u64);

impl Add<u64> for PhysAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        PhysAddr(self.0 + rhs)
    }
}

impl AddAssign<u64> for PhysAddr {
    #[inline]
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl PhysAddr {
    #[inline]
    pub const fn new(addr: u64) -> PhysAddr {
        PhysAddr(addr)
    }

    #[inline]
    pub const fn is_aligned(&self) -> bool {
        pg_round_down(self.0) == self.0
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl VirtAddr {
    #[inline]
    pub fn new(addr: u64) -> Self {
        VirtAddr(addr)
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}
