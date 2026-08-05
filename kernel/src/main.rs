#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use core::panic::PanicInfo;

use crate::acpi::{
    madt::{MADT, get_usable_cpus_count},
    sdt::XSDTInfo,
};

#[macro_use]
mod drivers;
mod acpi;
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

    let physical_memory_offset = boot_info
        .physical_memory_offset
        .into_option()
        .expect("Physical memory offset is not set");

    memory::set_physical_memory_offset(physical_memory_offset);

    // https://wiki.osdev.org/RSDP
    let rsdp_addr = boot_info.rsdp_addr.into_option().expect("RSDP address is not set");
    let xsdt_info = XSDTInfo::new(rsdp_addr);
    let madt_addr = xsdt_info.expect("Failed to parse XSDT").madt.expect("MADT address not found");

    let cpus_count = get_usable_cpus_count(madt_addr);
    info!("Detected {} usable CPUs ", cpus_count);
    if cpus_count < 1 {
        panic!("Invalid cpus count: {}", cpus_count)
    }

    info!("Initializing frame allocator");
    let allocator = memory::frame_allocator::FrameAllocator::new(&mut boot_info.memory_regions, cpus_count)
        .expect("Cannot init frame allocator LLFree");

    let order = 1;
    let allocated_phys = allocator.allocate(order).unwrap();
    info!("allocated pointer {}", allocated_phys);

    allocator.deallocate(allocated_phys, order);

    let madt = MADT::new(madt_addr).expect("Failed to parse MADT");
    info!("Parsed MADT: {:?}", madt);

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
