//! Console input and output, to the uart.
//!
//! Reads are line at a time.
//! Implements special input characters:
//!   newline -- end of line
//!   control-h -- backspace
//!   control-u -- kill line
//!   control-d -- end of file
//!   control-p -- print process list

use crate::{proc::PROCS, spinlock::SpinMutex, uart::uart_putc_sync};
use core::{fmt, ptr};

pub static CONS: SpinMutex<Console> = SpinMutex::new("cons", Console::default());

const INPUT_BUF: usize = 128;

const BACKSPACE: u8 = b'\x08';

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

pub fn console_init() {}

const fn ctrl(b: u8) -> u8 {
    b - b'@'
}

/// the console input interrupt handler.
/// uartintr() calls this for input character.
/// do erase/kill processing, append to cons.buf,
/// wake up consoleread() if a whole line has arrived.
pub fn console_intr(c: u8) {
    let mut cons = CONS.lock();

    match c {
        c if c == ctrl(b'P') => {
            // print process list
            PROCS.proc_dump();
        }
        c if c == ctrl(b'U') => {
            // erase line
            while cons.e != cons.w && cons.buf[(cons.e - 1) % INPUT_BUF] != b'\n' {
                cons.e -= 1;
                cons_putc(BACKSPACE);
            }
        }
        c if c == ctrl(b'H') || c == b'\x7f' => {
            // backspace
            if cons.e != cons.w {
                cons.e -= 1;
                cons_putc(BACKSPACE);
            }
        }
        c => {
            if c != 0 && (cons.e - cons.r) < INPUT_BUF {
                let c = if c == b'\r' { b'\n' } else { c };

                // echo back to the user.
                cons_putc(c);

                // store for consumption by consoleread().
                let i = cons.e % INPUT_BUF;
                cons.buf[i] = c;
                cons.e += 1;

                if c == b'\n' || c == ctrl(b'D') || cons.e == cons.r + INPUT_BUF {
                    // wake up consoleread() if a whole line (or end-of-file) has arrived.
                    cons.w = cons.e;
                    PROCS.wakeup(ptr::addr_of!(cons.r) as usize);
                }
            }
        }
    }
}

/// send one character to the uart.
/// called by printf, and to echo input characters,
/// but not from write().
fn cons_putc(c: u8) {
    if c == BACKSPACE {
        // if the user typed backspace, overwrite with a space.
        uart_putc_sync(b'\x08');
        uart_putc_sync(b' ');
        uart_putc_sync(b'\x08');
    } else {
        uart_putc_sync(c);
    }
}

/// user write()s to the console go here.
pub fn console_write(user_src: i32, src: u64, n: i32) -> i32 {
    let mut i = 0;

    i
}
