// Mutual exclusion spin locks.

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::{
    proc::{Cpu, CPUS},
    riscv::*,
};

pub struct SpinMutex<T: ?Sized> {
    locked: AtomicPtr<Cpu>, // Is the lock held?

    name: &'static str,  // Name of the lock.
    data: UnsafeCell<T>, // The data protected by the lock.
}

unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}

pub struct SpinMutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a SpinMutex<T>,
}

impl<T: ?Sized> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for SpinMutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if !self.lock.holding() {
            panic!("{} is not held", self.lock.name);
        }

        self.lock.locked.store(ptr::null_mut(), Ordering::Release);

        pop_off();
    }
}

impl<T> SpinMutex<T> {
    pub const fn new(name: &'static str, t: T) -> SpinMutex<T> {
        SpinMutex {
            locked: AtomicPtr::new(ptr::null_mut()),
            data: UnsafeCell::new(t),
            name,
        }
    }
}

impl<T: ?Sized> SpinMutex<T> {
    // Acquire the lock.
    // Loops (spins) until the lock is acquired.
    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        // disable interrupts to avoid deadlock.
        push_off();
        if self.holding() {
            panic!("{} is already locked", self.name);
        }

        while self
            .locked
            .compare_exchange(
                ptr::null_mut(),
                ptr::addr_of!(*CPUS.mycpu()) as *mut _,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            // The spin loop is a hint to the CPU that we're waiting, but probably
            // not for very long
            core::hint::spin_loop();
        }

        SpinMutexGuard { lock: self }
    }

    pub fn holding(&self) -> bool {
        let a = self.locked.load(Ordering::Relaxed) as usize;
        let b = CPUS.mycpu() as *const _ as usize;
        a == b
    }
}

// push_off/pop_off are like intr_off()/intr_on() except that they are matched:
// it takes two pop_off()s to undo two push_off()s.  Also, if interrupts
// are initially off, then push_off, pop_off leaves them off.
pub fn push_off() {
    let old = intr_get();

    intr_off();

    let mycpu = CPUS.mycpu();
    if mycpu.noff.fetch_add(1, Ordering::Relaxed) == 0 {
        mycpu.intena.store(old, Ordering::Relaxed);
    }
}

pub fn pop_off() {
    let mycpu = CPUS.mycpu();
    if intr_get() {
        panic!("pop_off() called with interrupts enabled");
    }

    if mycpu.noff.fetch_sub(1, Ordering::Relaxed) < 1 {
        panic!("pop_off() called too many times");
    }

    if mycpu.noff.load(Ordering::Relaxed) == 0 && mycpu.intena.load(Ordering::Relaxed) {
        intr_on();
    }
}
