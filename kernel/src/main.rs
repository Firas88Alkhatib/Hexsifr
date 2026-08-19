#![no_std]
#![no_main]
#![feature(custom_test_frameworks, abi_x86_interrupt)]
#![test_runner(crate::test_runner)]

extern crate alloc;

use bootloader_api::{BootInfo, BootloaderConfig, config::Mapping, entry_point};
use core::panic::PanicInfo;

use crate::{
    cpu::{halt_loop, per_cpu::get_cpu_info},
    time::sleep_ms,
};

#[macro_use]
mod debug;
#[macro_use]
mod drivers;
mod acpi;
mod cpu;
mod memory;
mod scheduler;
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
    scheduler::init();

    info!("System uptime: {:?}", time::uptime_duration());
    info!("System Date Time: {:?}", time::current_date_time());

    cpu::per_cpu::init_per_cpu_data();
    cpu::ap::start_ap_processors();

    info!("Init completed, entering scheduler");
    scheduler::spawn(task_a, scheduler::Priority::Normal);
    scheduler::spawn(task_b, scheduler::Priority::Normal);
    scheduler::spawn(task_c, scheduler::Priority::Low);
    scheduler::spawn(task_d, scheduler::Priority::High);
    scheduler::start()
}

fn idle_task() {
    halt_loop()
}

pub fn task_a() {
    let cpu_id = get_cpu_info().shared.id;
    loop {
        info!("Task A on CPU {cpu_id}",);
        sleep_ms(200);
    }
}
pub fn task_b() {
    let cpu_id = get_cpu_info().shared.id;
    loop {
        info!("Task B on CPU {cpu_id}",);
        sleep_ms(1000);
    }
}
pub fn task_c() {
    let cpu_id = get_cpu_info().shared.id;
    for i in 0..5 {
        info!("Task B on CPU {} ... loop {}", cpu_id, i);
        sleep_ms(500);
    }
}
pub fn task_d() {
    let cpu_id = get_cpu_info().shared.id;
    for i in 0..5 {
        info!("Task D on CPU {} ... loop {}", cpu_id, i);
        sleep_ms(700);
    }
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
