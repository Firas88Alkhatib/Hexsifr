use alloc::vec::Vec;
use x86_64::VirtAddr;

use crate::drivers::usb::xhci::mmio::{Mmio, Reg};

#[repr(C)]
#[derive(Clone)]
pub struct Doorbell {
    pub reg: Reg<u32>,
}
impl Mmio<Doorbell> {
    pub fn ring(&self, target: u8) {
        self.reg.write(target as u32);
    }
}

pub struct DoorbellsManager {
    doorbells: Vec<Mmio<Doorbell>>,
    max_slots: u8,
}
impl DoorbellsManager {
    pub fn new(base_addr: VirtAddr, max_slots: u8) -> Self {
        let count = max_slots as usize + 1; // max slots + command doorbell
        let doorbells =
            (base_addr..).step_by(size_of::<Doorbell>()).take(count).map(|addr| Mmio::<Doorbell>::new(addr)).collect();

        Self { doorbells, max_slots }
    }
    pub fn ring_command_doorbell(&self) {
        self.doorbells[0].reg.write(0);
    }
    pub fn get_doorbell(&self, index: u8) -> &Mmio<Doorbell> {
        &self.doorbells[index as usize]
    }
}
