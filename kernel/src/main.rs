#![no_std]
#![no_main]
#![feature(custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::test_runner)]

extern crate alloc;

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use core::panic::PanicInfo;

#[macro_use]
mod debug;
#[macro_use]
mod drivers;
mod acpi;
mod cpu;
mod memory;
mod time;

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
    acpi::acpi_init(rsdp_addr as usize);
    cpu::cpu_init();
    memory::memory_init(&mut boot_info.memory_regions);
    time::time_init();

    info!("System uptime: {:?}", time::uptime_duration());
    info!("System Date Time: {:?}", time::current_date_time());

    cpu::per_cpu::init_per_cpu_data();
    cpu::ap::start_ap_processors();

    info!("Init completed, entering main loop");
    crate::cpu::halt_loop()
}

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    s_println!("[PANIC] {}", info);
    crate::cpu::halt_loop()
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
