#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use core::panic::PanicInfo;

#[macro_use]
mod drivers;
mod acpi;
mod arch;
mod memory;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);

    config
};

entry_point!(kernal_start, config = &BOOTLOADER_CONFIG);
fn kernal_start(boot_info: &'static mut BootInfo) -> ! {
    drivers::serial::init();
    info!("Booting Hexsifr kernel...");

    let physical_memory_offset = boot_info.physical_memory_offset.into_option().expect("Physical memory offset is not set");
    memory::set_physical_memory_offset(physical_memory_offset);

    let rsdp_addr = boot_info.rsdp_addr.into_option().expect("RSDP address is not set");
    let cpus_count = crate::acpi::get_usable_cpu_count(rsdp_addr);
    info!("Detected {} usable CPUs ", cpus_count);

    crate::memory::memory_init(&mut boot_info.memory_regions, cpus_count);

    info!("Init completed, entering main loop");
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
