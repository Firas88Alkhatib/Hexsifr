use acpi_crate::{Handle, Handler, PciAddress, PhysicalMapping, aml::AmlError};
use core::ptr::NonNull;
use x86_64::{
    instructions::port::Port,
    structures::port::{PortRead, PortWrite},
};

use crate::{
    memory::phys_to_virt,
    time::{boot_ns, sleep_us},
};

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;
const PCI_CONFIG_ENABLE: u32 = 1 << 31;

fn read_mem<T>(address: usize) -> T {
    unsafe { (address as *const T).read_volatile() }
}
fn write_mem<T>(address: usize, value: T) {
    unsafe { (address as *mut T).write_volatile(value) }
}

fn read_io<T: PortRead>(port: u16) -> T {
    unsafe { Port::<T>::new(port).read() }
}
fn write_io<T: PortWrite>(port: u16, value: T) {
    unsafe { Port::<T>::new(port).write(value) }
}

/// Reads 32‑bit PCI configuration space.
/// # Safety
/// Caller must ensure `bus`, `device`, `function`, `offset` are valid.
pub fn read_pci_config<T: PortRead>(bus: u8, device: u8, function: u8, offset: u8) -> T {
    // TODO use MCFG  if available

    // Build the address:
    // bit 31: enable (must be 1)
    // bits 23-16: bus number
    // bits 15-11: device number
    // bits 10-8: function number
    // bits 7-2: register offset (aligned to 4 bytes)
    let address: u32 =
        PCI_CONFIG_ENABLE | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | ((offset as u32) & 0xFC);

    unsafe {
        Port::<u32>::new(PCI_CONFIG_ADDRESS).write(address);
        Port::<T>::new(PCI_CONFIG_DATA).read()
    }
}

pub fn write_pci_config<T: PortWrite>(bus: u8, device: u8, function: u8, offset: u8, value: T) {
    // Build the 32‑bit address (always a 32‑bit write).
    let address: u32 =
        PCI_CONFIG_ENABLE | ((bus as u32) << 16) | ((device as u32) << 11) | ((function as u32) << 8) | ((offset as u32) & 0xFC);
    unsafe {
        Port::<u32>::new(PCI_CONFIG_ADDRESS).write(address);
        Port::<T>::new(PCI_CONFIG_DATA).write(value);
    }
}

#[derive(Clone, Copy)]
pub struct AcpiHandler;

impl Handler for AcpiHandler {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> PhysicalMapping<Self, T> {
        let virt = phys_to_virt::<T>(physical_address as u64) as *mut T;

        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(virt).expect("Failed to map physical region in AcpiHandler"),
            region_length: size,
            mapped_length: size,
            handler: self.clone(),
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}

    fn read_u8(&self, address: usize) -> u8 {
        read_mem(address)
    }

    fn read_u16(&self, address: usize) -> u16 {
        read_mem(address)
    }

    fn read_u32(&self, address: usize) -> u32 {
        read_mem(address)
    }

    fn read_u64(&self, address: usize) -> u64 {
        read_mem(address)
    }

    fn write_u8(&self, address: usize, value: u8) {
        write_mem(address, value);
    }

    fn write_u16(&self, address: usize, value: u16) {
        write_mem(address, value);
    }

    fn write_u32(&self, address: usize, value: u32) {
        write_mem(address, value);
    }

    fn write_u64(&self, address: usize, value: u64) {
        write_mem(address, value);
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        read_io(port)
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        read_io(port)
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        read_io(port)
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        write_io(port, value);
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        write_io(port, value);
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        write_io(port, value);
    }

    fn read_pci_u8(&self, address: PciAddress, offset: u16) -> u8 {
        read_pci_config(address.bus(), address.device(), address.function(), offset as u8)
    }

    fn read_pci_u16(&self, address: PciAddress, offset: u16) -> u16 {
        read_pci_config(address.bus(), address.device(), address.function(), offset as u8)
    }

    fn read_pci_u32(&self, address: PciAddress, offset: u16) -> u32 {
        read_pci_config(address.bus(), address.device(), address.function(), offset as u8)
    }

    fn write_pci_u8(&self, address: PciAddress, offset: u16, value: u8) {
        write_pci_config(address.bus(), address.device(), address.function(), offset as u8, value)
    }

    fn write_pci_u16(&self, address: PciAddress, offset: u16, value: u16) {
        write_pci_config(address.bus(), address.device(), address.function(), offset as u8, value)
    }

    fn write_pci_u32(&self, address: PciAddress, offset: u16, value: u32) {
        write_pci_config(address.bus(), address.device(), address.function(), offset as u8, value)
    }

    fn nanos_since_boot(&self) -> u64 {
        boot_ns() as u64
    }

    fn stall(&self, microseconds: u64) {
        sleep_us(microseconds);
    }

    fn sleep(&self, milliseconds: u64) {
        // TODO better approach when we have the scheduler done
        self.stall(milliseconds * 1000);
    }

    fn create_mutex(&self) -> Handle {
        todo!()
    }

    fn acquire(&self, _mutex: Handle, _timeout: u16) -> Result<(), AmlError> {
        todo!()
    }

    fn release(&self, _mutex: Handle) {
        todo!()
    }
}
