use core::ops::Add;

use x86_64::{PhysAddr, VirtAddr};

use crate::{
    cpu::{idt::XHCI_MSIX_VECTOR, per_cpu::get_cpu_info},
    drivers::pci::pci_class::CapabilityHeader,
    memory::{phys_to_virt, phys_to_virt_mut},
};

fn bir_offset(value: u32) -> (u8, u64) {
    let bir = (value & 0x7) as u8;
    let offset = (value & !0x7) as u64;
    (bir, offset)
}
#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct MsiXEntry {
    pub msg_addr_low: u32,
    pub msg_addr_high: u32,
    pub msg_data: u32,
    pub vector_control: u32,
}

#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct MsiXCapHeader {
    header: CapabilityHeader,
    pub msg_control: u16,
    pub table_bir_offset: u32,
    pub pba_bir_offset: u32,
}

pub struct MsixCap {
    header: MsiXCapHeader,
    phys: PhysAddr,
    bar_virt: VirtAddr,
}
impl MsixCap {
    pub fn new(phys: PhysAddr, bar_virt: VirtAddr) -> Self {
        let header = unsafe { phys_to_virt::<MsiXCapHeader>(phys.as_u64()).read_volatile() };
        Self { header, phys, bar_virt }
    }
    pub fn write_entry(&self) {
        let (_table_bir, table_offset) = bir_offset(self.header.table_bir_offset);

        let table_virt = self.bar_virt.add(table_offset);

        let lapic_id = unsafe { get_cpu_info().lapic.id() };

        let entry = MsiXEntry {
            msg_addr_low: 0xFEE00000 | (lapic_id << 12),
            msg_addr_high: 0,
            msg_data: XHCI_MSIX_VECTOR as u32,
            vector_control: 0,
        };

        unsafe { table_virt.as_mut_ptr::<MsiXEntry>().write_volatile(entry) };
    }

    pub fn enable(&self) {
        // Enable MSI-X by writing to msg_control in MsiXCapHeader
        let msg_control_virt = phys_to_virt_mut::<u16>(self.phys.as_u64() + 2);
        unsafe { msg_control_virt.write_volatile(self.header.msg_control | 0x8000) };
    }
}
