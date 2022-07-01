use core::{
    fmt,
    panic::PanicInfo,
    sync::atomic::{AtomicBool, Ordering},
};

pub static PANICKED: AtomicBool = AtomicBool::new(false);

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // freeze uart output from other CPUs
    PANICKED.store(true, Ordering::Relaxed);
    loop {}
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

pub fn _print(args: fmt::Arguments) {
    use crate::console::CONS;
    use core::fmt::Write;
    CONS.lock().write_fmt(args).unwrap();
}
