use core::ptr;

use crate::{memlayout::PHYSTOP, spinlock::SpinMutex};
use alloc::alloc::{GlobalAlloc, Layout};

#[global_allocator]
static ALLOCATOR: SpinMutex<BumpAllocator> = SpinMutex::new("kmem", BumpAllocator::new());

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout);
}

pub fn kinit() {
    unsafe {
        ALLOCATOR
            .lock()
            .init(end.as_ptr() as usize, PHYSTOP as usize);
    }
}

extern "C" {
    // first address after kernel.
    // defined by kernel.ld.
    static end: [u8; 0];
}

pub struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    /// Creates a new empty allocator.
    pub const fn new() -> Self {
        BumpAllocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the
    /// given heap bounds are valid and that the heap is unused. This method
    /// must be called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for SpinMutex<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        let size = layout.size();
        let align = layout.align();

        // `Layout` contract forbids making a `Layout` with align=0, or align not power of 2.
        // So we can safely use a mask to ensure alignment without worrying about UB.
        let align_mask_to_round_down = !(align - 1);

        let alloc_start = (allocator.next + align - 1) & align_mask_to_round_down;
        let alloc_end = match alloc_start.checked_add(size) {
            Some(end_addr) => end_addr,
            None => return ptr::null_mut(),
        };

        if alloc_end > allocator.heap_end {
            ptr::null_mut()
        } else {
            allocator.next = alloc_end;
            allocator.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut allocator = self.lock();

        allocator.allocations -= 1;
        if allocator.allocations == 0 {
            allocator.next = allocator.heap_start;
        }
    }
}
