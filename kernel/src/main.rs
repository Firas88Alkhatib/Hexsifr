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
    let acpi_boot_info = acpi::get_acpi_boot_info(rsdp_addr);
    info!("Detected {} usable CPUs ", acpi_boot_info.usable_cpu_count);

    cpu::lapic::set_lapic_base_addr(acpi_boot_info.lapic_addresss);
    cpu::init();

    memory::memory_init(&mut boot_info.memory_regions, acpi_boot_info.usable_cpu_count);

    let acpi = acpi::ACPI::new(rsdp_addr);
    time::time_init(acpi.hpet, acpi.fadt);

    info!("System uptime: {:?}", time::uptime_duration());
    info!("System Date Time: {:?}", time::current_date_time());
    info!("System Date Time in nanos: {:?}", time::current_time_ns());
    info!("Sleeping for 10 seconds...");
    time::sleep_ms(10_000);
    info!("System uptime: {:?}", time::uptime_duration());
    info!("System Date Time: {:?}", time::current_date_time());
    info!("System Date Time in nanos: {:?}", time::current_time_ns());

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
