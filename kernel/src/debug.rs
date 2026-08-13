use core::fmt::{self, Write};
use x86_64::instructions::port::Port;

struct QemuDebug;

impl Write for QemuDebug {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            unsafe {
                Port::<u8>::new(0xE9).write(byte);
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
pub fn print(args: fmt::Arguments) {
    QemuDebug.write_fmt(args).unwrap();
}
pub fn println(args: fmt::Arguments) {
    QemuDebug.write_fmt(format_args!("{}\n", args)).unwrap();
}

#[macro_export]
macro_rules! dbg {
    ($($arg:tt)*) => {
        $crate::debug::println(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! dbgn {
    ($($arg:tt)*) => {
        $crate::debug::print(format_args!($($arg)*));
    };
}
