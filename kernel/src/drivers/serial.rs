use core::fmt::{Arguments, Write};
use spin::Mutex;
use uart_16550::{Config, Uart16550Tty, backend::PioBackend};

const COM1: u16 = 0x3F8;

static SERIAL: Mutex<Option<Uart16550Tty<PioBackend>>> = Mutex::new(None);

macro_rules! s_print {
    ($($arg:tt)*) => {
        $crate::drivers::serial::_print(format_args!($($arg)*));
    };
}

macro_rules! s_println {
    () => (s_print!("\n"));
    ($($arg:tt)*) => (s_print!("{}\n",format_args!($($arg)*)));
}

macro_rules! info {
    () => (s_print!("\n"));
    ($($arg:tt)*) => {
        s_println!("[INFO] {}", format_args!($($arg)*));
    };
}

macro_rules! error {
    () => (s_print!("\n"));
    ($($arg:tt)*) => {
        s_println!("[ERROR] {}", format_args!($($arg)*));
    };
}

pub fn init() {
    let port = unsafe { Uart16550Tty::new_port(COM1, Config::default()).expect("Failed to initialize COM1") };
    *SERIAL.lock() = Some(port);
}

pub(crate) fn _print(args: Arguments) {
    if let Some(ref mut serial) = *SERIAL.lock() {
        serial.write_fmt(args).expect("Failed to write to serial port");
    }
}
