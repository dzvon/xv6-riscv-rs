use core::{
    fmt,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{console::cons_putc, spinlock::SpinMutex};

pub static PANICKED: AtomicBool = AtomicBool::new(false);
pub static PR: SpinMutex<Writer> = SpinMutex::new("pr", Writer);

static NEED_LOCKING: AtomicBool = AtomicBool::new(true);

pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            cons_putc(b);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::printf::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*));
    };
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    NEED_LOCKING.store(false, Ordering::Release);
    // freeze uart output from other CPUs
    println!("panic: {}", info);
    PANICKED.store(true, Ordering::Relaxed);
    loop {}
}
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    if NEED_LOCKING.load(Ordering::Acquire) {
        PR.lock().write_fmt(args).unwrap();
    } else {
        unsafe {
            (*PR.as_mut_ptr()).write_fmt(args).unwrap();
        }
    }
}
