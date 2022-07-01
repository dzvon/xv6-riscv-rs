use crate::{spinlock::SpinMutex, uart::uart_putc_sync};
use core::fmt;

pub static CONS: SpinMutex<Console> = SpinMutex::new("cons", Console::default());

const INPUT_BUF: usize = 128;

pub struct Console {
    pub r: usize,             // Read index
    pub w: usize,             // Write index
    pub e: usize,             // Edit index
    pub buf: [u8; INPUT_BUF], // Buffer
}

impl const Default for Console {
    fn default() -> Self {
        Console {
            r: 0,
            w: 0,
            e: 0,
            buf: [0; INPUT_BUF],
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            uart_putc_sync(b);
        }
        Ok(())
    }
}

pub fn consoleinit() {}
