#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use core::panic::PanicInfo;

#[macro_use]
mod drivers;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);

    config
};

entry_point!(kernal_start, config = &BOOTLOADER_CONFIG);
fn kernal_start(_boot_info: &'static mut BootInfo) -> ! {
    drivers::serial::init();
    info!("Booting Hexsifr kernel...");
    loop {}
}

pub(crate) fn halt_forever() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    s_println!("[PANIC] {}", info);
    halt_forever();
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
