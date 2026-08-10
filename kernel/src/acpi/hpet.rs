use core::{
    hint::spin_loop,
    ptr::{read_volatile, write_volatile},
};

use crate::{
    acpi::{AcpiGenericAddress, AcpiHeader},
    memory::{phys_to_virt, phys_to_virt_unaligned},
};

#[repr(usize)]
pub enum HpetRegister {
    Capabilities = 0x00,
    Config = 0x10,
    Counter = 0xF0,
}
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HpetTable {
    pub header: AcpiHeader,
    pub hardware_rev_id: u8,
    pub comparator_count: u8,
    pub pci_vendor_id: u16,
    pub address: AcpiGenericAddress,
    pub hpet_number: u8,
    pub minimum_tick: u16,
    pub page_protection: u8,
}

pub struct HPET {
    base_virt: *mut u8,
}

impl HPET {
    pub fn new(hpet_phys_addr: u64) -> Self {
        let hpet_table = phys_to_virt_unaligned::<HpetTable>(hpet_phys_addr);
        let base_virt = phys_to_virt::<u8>(hpet_table.address.address) as *mut u8;
        Self { base_virt }
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

    pub fn wait_ticks(&self, ticks: u64) {
        let start = self.main_counter();
        while self.main_counter() - start < ticks {
            spin_loop();
        }
    }

    pub fn calibrate_tsc(&self) -> Option<u64> {
        let hpet_freq = self.frequency()?;
        if hpet_freq == 0 {
            return None;
        }
        // Wait for a known duration: e.g., 10 ms.
        let wait_ticks = hpet_freq / 100; // 10 ms
        let tsc_start = unsafe { core::arch::x86_64::_rdtsc() };
        self.wait_ticks(wait_ticks);
        let tsc_end = unsafe { core::arch::x86_64::_rdtsc() };
        let elapsed_tsc = tsc_end - tsc_start;
        // TSC frequency = (TSC ticks * HPET frequency) / HPET ticks
        let tsc_freq = (elapsed_tsc * hpet_freq) / wait_ticks;
        Some(tsc_freq)
    }
}
