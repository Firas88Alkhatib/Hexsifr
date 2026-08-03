use super::AcpiHeader;
use crate::{
    acpi::acpi_sig,
    memory::{phys_to_virt, phys_to_virt_unaligned},
};

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub(crate) struct RSDPExtended {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
    pub length: u32,
    pub xsdt_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

// https://wiki.osdev.org/XSDT
#[derive(Default, Debug)]
pub struct XSDTInfo {
    pub madt: Option<u64>,
    pub dmar: Option<u64>,
    pub ivrs: Option<u64>,
    pub fadt: Option<u64>,
    pub hpet: Option<u64>,
    pub mcfg: Option<u64>,
    pub bgrt: Option<u64>,
    pub waet: Option<u64>,
}

impl XSDTInfo {
    pub fn new(rsdp_address: u64) -> Result<Self, &'static str> {
        let mut result = Self::default();

        let rsdp = phys_to_virt_unaligned::<RSDPExtended>(rsdp_address);

        if rsdp.signature != acpi_sig::RSDP {
            return Err("Invalid RSDP");
        }
        if rsdp.revision < 2 {
            return Err("RSDP revision is less than 2, XSDT is not available");
        }

        let xsdt = phys_to_virt_unaligned::<AcpiHeader>(rsdp.xsdt_address);
        if xsdt.signature != *b"XSDT" {
            return Err("Invalid XSDT signature");
        }
        let entries_count = (xsdt.length as usize - size_of::<AcpiHeader>()) / size_of::<u64>();
        let entries_address = rsdp.xsdt_address + size_of::<AcpiHeader>() as u64;
        let entries_ptr = phys_to_virt::<u64>(entries_address);

        for i in 0..entries_count {
            let entry_address = unsafe { entries_ptr.add(i).read_unaligned() };
            let entry_header = phys_to_virt_unaligned::<AcpiHeader>(entry_address);

            match entry_header.signature {
                acpi_sig::MADT => result.madt = Some(entry_address),
                acpi_sig::DMAR => result.dmar = Some(entry_address),
                acpi_sig::IVRS => result.ivrs = Some(entry_address),
                acpi_sig::FADT => result.fadt = Some(entry_address),
                acpi_sig::HPET => result.hpet = Some(entry_address),
                acpi_sig::MCFG => result.mcfg = Some(entry_address),
                acpi_sig::BGRT => result.bgrt = Some(entry_address),
                acpi_sig::WAET => result.waet = Some(entry_address),
                _ => {}
            }
        }

        Ok(result)
    }
}
