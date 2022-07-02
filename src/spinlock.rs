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
                // SAFETY: push_off() disables interrupts, so this is safe.
                unsafe { CPUS.mycpu() as *mut Cpu },
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
        // SAFETY: This function is only called from the lock() function, which
        // disables interrupts.
        self.locked.load(Ordering::Relaxed) == unsafe { CPUS.mycpu() as *mut Cpu }
    }

    /// Force unlock this [`SpinMutex`].
    ///
    /// # Safety
    ///
    /// This is *extremely* unsafe if the lock is not held by the current
    /// thread. However, this can be useful in some instances for exposing the
    /// lock to FFI that doesn't know how to deal with RAII.
    pub unsafe fn force_unlock(&self) {
        self.locked.store(ptr::null_mut(), Ordering::Release)
    }
}

pub fn guard_lock<'a, T: ?Sized>(guard: &SpinMutexGuard<'a, T>) -> &'a SpinMutex<T> {
    guard.lock
}

// push_off/pop_off are like intr_off()/intr_on() except that they are matched:
// it takes two pop_off()s to undo two push_off()s.  Also, if interrupts
// are initially off, then push_off, pop_off leaves them off.
pub fn push_off() {
    let old = intr_get();

    intr_off();

    // SAFETY: We are disabling interrupts, so we can't be holding the lock.
    unsafe {
        let mycpu = CPUS.mycpu();

        if mycpu.noff == 0 {
            mycpu.intena = old;
        }

        mycpu.noff += 1;
    }
}

pub fn pop_off() {
    if intr_get() {
        panic!("pop_off() called with interrupts enabled");
    }

    // SAFETY: We are ensuring that the interrupts are disabled here.
    unsafe {
        let mycpu = CPUS.mycpu();

        if mycpu.noff < 1 {
            panic!("pop_off() called too many times");
        }

        mycpu.noff -= 1;

        if mycpu.noff == 0 && mycpu.intena {
            intr_on();
        }
    }
}
