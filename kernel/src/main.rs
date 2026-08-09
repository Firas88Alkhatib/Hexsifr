#![no_std]
#![no_main]
#![feature(custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::test_runner)]

extern crate alloc;

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use core::panic::PanicInfo;

use crate::cpu::halt_loop;

#[macro_use]
mod drivers;
mod acpi;
mod cpu;
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
    let acpi_info = acpi::get_acpi_boot_info(rsdp_addr);
    info!("Detected {} usable CPUs ", acpi_info.usable_cpu_count);

    cpu::lapic::init_lapic(acpi_info.lapic_addresss);

    memory::memory_init(&mut boot_info.memory_regions, acpi_info.usable_cpu_count);

    cpu::gdt::PerCpuGdt::new().load();
    cpu::idt::init_idt();

    info!("Init completed, entering main loop");
    halt_loop();
}

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    s_println!("[PANIC] {}", info);
    halt_loop();
}

pub fn test_runner(tests: &[&dyn Fn()]) {
    for test in tests {
        test();
    }
}
