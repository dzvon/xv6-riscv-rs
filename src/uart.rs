//! low-level driver routines for 16550a UART.
//!
//! The UART control registers.
/// some have different meanings for
/// read vs write.
/// see http://byterunner.com/16550.html
use core::sync::atomic::Ordering;

use crate::{
    memlayout::UART0,
    printf::PANICKED,
    proc::PROCS,
    spinlock::{pop_off, push_off, SpinMutex},
};

pub static UART: SpinMutex<Uart> = SpinMutex::new("uart", Uart::default());

const UART_TX_BUF_SIZE: usize = 32;

/// receive holding register (for input bytes)
const RHR: usize = 0;
/// transmit holding register (for output bytes)
const THR: usize = 0;
/// interrupt enable register
const IER: usize = 1;
/// enable receive interrupts
const IER_RX_ENABLE: u8 = 1 << 0;
/// enable transmit interrupts
const IER_TX_ENABLE: u8 = 1 << 1;
/// FIFO control register
const FCR: usize = 2;
/// enable FIFO
const FCR_FIFO_ENABLE: u8 = 1 << 0;
/// clear the content of the two FIFOs
const FCR_FIFO_CLEAR: u8 = 3 << 1;
/// interrupt status register
const ISR: usize = 2;
/// line control register
const LCR: usize = 3;
/// 8 bits per byte
const LCR_EIGHT_BITS: u8 = 3 << 0;
/// special mode to set baud rate
const LCR_BAUD_LATCH: u8 = 1 << 7;
/// line status register
const LSR: usize = 5;
/// input is waiting to be read from RHR
const LSR_RX_READY: usize = 1 << 0;
/// THR can accept another character to send
const LSR_TX_IDLE: u8 = 1 << 5;

pub struct Uart {
    buf: [u8; UART_TX_BUF_SIZE],
    // write next to uart_tx_buf[uart_tx_w % UART_TX_BUF_SIZE]
    tx_w: usize,
    // read next from uart_tx_buf[uart_tx_r % UART_TX_BUF_SIZE]
    tx_r: usize,
}

impl const Default for Uart {
    fn default() -> Self {
        Uart {
            buf: [0; UART_TX_BUF_SIZE],
            tx_w: 0,
            tx_r: 0,
        }
    }
}

impl Uart {
    /// add a character to the output buffer and tell the
    /// UART to start sending if it isn't already.
    /// blocks if the output buffer is full.
    /// because it may block, it can't be called
    /// from interrupts; it's only suitable for use
    /// by write().
    pub fn uart_putc(&mut self, c: u8) {
        if PANICKED.load(Ordering::Relaxed) {
            loop {}
        }

        loop {
            if self.tx_w - self.tx_r == UART_TX_BUF_SIZE {
                // buffer is full.
                // wait for uart_start() to open up space in the buffer.

                // PROCS::current().sleep();
            } else {
                self.buf[self.tx_w % UART_TX_BUF_SIZE] = c;
                self.tx_w += 1;
                self.uart_start();
                break;
            }
        }
    }

    /// if the UART is idle, and a character is waiting
    /// in the transmit buffer, send it.
    /// caller must hold uart_tx_lock.
    /// called from both the top- and bottom-half.
    pub fn uart_start(&mut self) {
        loop {
            if self.tx_w - self.tx_r == 0 {
                // transmit buffer is empty.
                return;
            }

            if (read_reg(LSR) & LSR_TX_IDLE) == 0 {
                // the UART transmit holding register is full,
                // so we cannot give it another byte.
                // it will interrupt when it's ready for a new byte.
                return;
            }

            let c = self.buf[self.tx_r % UART_TX_BUF_SIZE];
            self.tx_r += 1;

            // maybe uartputc() is waiting for space in the buffer.
            // TODO: wakeup(&uart_tx_r);

            write_reg(THR, c);
        }
    }

    // read one input character from the UART.
    // return -1 if none is waiting.
    pub fn uart_getc() -> Option<u8> {
        if (read_reg(LSR) & 0x01) != 0 {
            // input data is ready.
            Some(read_reg(RHR))
        } else {
            None
        }
    }
}

/// alternate version of uartputc() that doesn't
/// use interrupts, for use by kernel printf() and
/// to echo characters. it spins waiting for the uart's
/// output register to be empty.
pub fn uart_putc_sync(c: u8) {
    push_off();

    if PANICKED.load(Ordering::Relaxed) {
        loop {}
    }

    while read_reg(LSR) & LSR_TX_IDLE == 0 {
        // spin until the UART is ready for another byte.
        core::hint::spin_loop();
    }
    write_reg(THR, c);

    pop_off();
}

/// handle a uart interrupt, raised because input has
/// arrived, or the uart is ready for more output, or
/// both. called from trap.c.
pub fn uart_intr() {
    // read and process incoming characters.
    loop {
        let c = Uart::uart_getc();
        if c.is_none() {
            break;
        }
        // TODO: console_intr(c);
    }

    // send buffered characters.
    UART.lock().uart_start();
}

pub fn uart_init() {
    // disable interrupts.
    write_reg(IER, 0x00);

    // special mode to set baud rate.
    write_reg(LCR, LCR_BAUD_LATCH);

    // LSB for baud rate of 38.4K.
    write_reg(0, 0x03);

    // MSB for baud rate of 38.4K.
    write_reg(1, 0x00);

    // leave set-baud mode,
    // and set word length to 8 bits, no parity.
    write_reg(LCR, LCR_EIGHT_BITS);

    // reset and enable FIFOs.
    write_reg(FCR, FCR_FIFO_ENABLE | FCR_FIFO_CLEAR);

    // enable transmit and receive interrupts.
    write_reg(IER, IER_RX_ENABLE | IER_TX_ENABLE);
}

fn write_reg(reg: usize, val: u8) {
    // the UART control registers are memory-mapped
    // at address UART0. this macro returns the
    // address of one of the registers.
    let base_pointer = UART0 as *mut u8;
    unsafe { base_pointer.add(reg).write(val) }
}

fn read_reg(reg: usize) -> u8 {
    let base_pointer = UART0 as *mut u8;
    unsafe { base_pointer.add(reg).read() }
}
