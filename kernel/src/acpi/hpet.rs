use core::{
    hint::spin_loop,
    ptr::{read_volatile, write_volatile},
};

use acpi_crate::HpetInfo;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{acpi::get_acpi, memory::phys_to_virt, time::read_tsc_counter};

#[repr(usize)]
pub enum HpetRegister {
    Capabilities = 0x00,
    Config = 0x10,
    Counter = 0xF0,
}

#[derive(Debug, Clone)]
pub struct HPET {
    base_virt: *mut u8,
}

impl HPET {
    pub fn new() -> Result<Self, &'static str> {
        let acpi = get_acpi();
        let hpet_info = HpetInfo::new(acpi).map_err(|_| "Cannot create HPET info")?;
        let base_virt = phys_to_virt::<u8>(hpet_info.base_address as u64) as *mut u8;
        Ok(Self { base_virt })
    }
    fn read_register(&self, reg: HpetRegister) -> u64 {
        unsafe {
            let ptr = self.base_virt.add(reg as usize) as *const u64;
            read_volatile::<u64>(ptr)
        }
    }
    fn write_register(&self, reg: HpetRegister, value: u64) {
        unsafe {
            let ptr = self.base_virt.add(reg as usize) as *mut u64;
            write_volatile(ptr, value)
        }
    }
    pub fn capabilities(&self) -> u64 {
        self.read_register(HpetRegister::Capabilities)
    }
    pub fn main_counter(&self) -> u64 {
        self.read_register(HpetRegister::Counter)
    }

    pub fn frequency(&self) -> Option<u64> {
        self.enable();
        let caps = self.capabilities();
        let period_fs = (caps >> 32) & 0xFFFF_FFFF;

        if period_fs == 0 {
            return None;
        }
        Some(1_000_000_000_000_000 / period_fs)
    }

    pub fn enable(&self) {
        let config = self.read_register(HpetRegister::Config) | 1;
        self.write_register(HpetRegister::Config, config);
    }

    // pub fn disable(&self) {
    //     let config = self.read_register(HpetRegister::Config) & !1;
    //     self.write_register(HpetRegister::Config, config);
    // }

    pub fn calibrate_tsc(&self) -> Option<u64> {
        let hpet_freq = self.frequency()?;
        if hpet_freq == 0 {
            return None;
        }
        let target_ticks = hpet_freq / 20; // 50 ms

        without_interrupts(|| {
            let hpet_start = self.main_counter();
            let tsc_start = read_tsc_counter();

            let mut hpet_now = hpet_start;
            while hpet_now.wrapping_sub(hpet_start) < target_ticks {
                hpet_now = self.main_counter();
                spin_loop();
            }

            let tsc_end = read_tsc_counter();
            let elapsed_hpet = hpet_now.wrapping_sub(hpet_start);
            if elapsed_hpet == 0 {
                return None;
            }

            let elapsed_tsc = tsc_end - tsc_start;
            Some((elapsed_tsc * hpet_freq) / elapsed_hpet)
        })
    }
}
