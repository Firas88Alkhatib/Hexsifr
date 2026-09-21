pub mod pci_class;

use crate::{acpi::get_acpi, drivers::pci::pci_class::PciDevice, memory::phys_to_virt};
use acpi_crate::platform::PciConfigRegions;

pub use pci_class::{PciConfigHeader, PciDevices};
use spin::Once;
use x86_64::PhysAddr;

#[repr(C, packed)]
struct PciCapabilityHeader {
    pub id: u8,
    pub next: u8,
}

static PCI_DEVICES: Once<PciDevices> = Once::new();

pub fn get_pci_devices() -> &'static PciDevices {
    PCI_DEVICES.call_once(|| read_pci_devices())
}

fn read_pci_devices() -> PciDevices {
    let mut pci_devices = PciDevices::default();

    let pci_regions = PciConfigRegions::new(get_acpi()).expect("Cannot find pci regions");

    for region in &pci_regions.regions {
        for bus in region.bus_number_start..=region.bus_number_end {
            for device in 0_u8..=31 {
                let dev_phys =
                    region.base_address + (u64::from(bus - region.bus_number_start) << 20) + (u64::from(device) << 15);
                let vendor_id = unsafe { phys_to_virt::<u16>(dev_phys).read_volatile() };
                if vendor_id == u16::MAX {
                    continue;
                }

                // Device exists at function 0 – read its full header.
                let header = unsafe { phys_to_virt::<PciConfigHeader>(dev_phys).read_volatile() };
                pci_devices.add(PciDevice { header, config_base: PhysAddr::new(dev_phys) });

                // If single‑function, skip functions 1..7.
                if (header.header_type & 0x80) == 0 {
                    continue;
                }

                for function in 1_u8..=7 {
                    let phys = dev_phys + (u64::from(function) << 12);
                    let vendor_fn = unsafe { phys_to_virt::<u16>(phys).read_volatile() };
                    if vendor_fn == u16::MAX {
                        continue;
                    }
                    let fn_header = unsafe { phys_to_virt::<PciConfigHeader>(phys).read_volatile() };

                    pci_devices.add(PciDevice { header: fn_header, config_base: PhysAddr::new(phys) });
                }
            }
        }
    }
    pci_devices
}
