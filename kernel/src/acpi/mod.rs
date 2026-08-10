use crate::{
    acpi::{
        hpet::HPET,
        madt::{MADTInfo, madt_tables::Madt},
        sdt::XSDTInfo,
    },
    memory::phys_to_virt_unaligned,
};

// https://wiki.osdev.org/ACPI
pub(crate) mod dmar;
pub(crate) mod fadt;
pub(crate) mod hpet;
pub(crate) mod madt;
pub(crate) mod mcfg;
pub(crate) mod sdt;

// Some of ACPI signatures
pub(crate) mod acpi_sig {
    pub const RSDP: [u8; 8] = *b"RSD PTR ";
    pub const MADT: [u8; 4] = *b"APIC";
    pub const DMAR: [u8; 4] = *b"DMAR";
    pub const IVRS: [u8; 4] = *b"IVRS";
    pub const FADT: [u8; 4] = *b"FADT";
    pub const HPET: [u8; 4] = *b"HPET";
    pub const MCFG: [u8; 4] = *b"MCFG";
    pub const BGRT: [u8; 4] = *b"BGRT";
    pub const WAET: [u8; 4] = *b"WAET";
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct AcpiHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AcpiGenericAddress {
    pub address_space: u8,
    pub bit_width: u8,
    pub bit_offset: u8,
    pub access_size: u8,
    pub address: u64,
}

pub struct ACPIBootInfo {
    pub usable_cpu_count: usize,
    pub lapic_addresss: u64,
}

pub(crate) fn get_acpi_boot_info(rsdp_addr: u64) -> ACPIBootInfo {
    let xsdt_info = XSDTInfo::new(rsdp_addr).expect("Failed to parse XSDT");
    let madt_address = xsdt_info.madt.expect("MADT address not found");
    let madt = phys_to_virt_unaligned::<Madt>(madt_address);

    ACPIBootInfo { usable_cpu_count: madt.usable_cpus(madt_address), lapic_addresss: madt.lapic_address(madt_address) }
}

pub struct ACPI {
    pub hpet: Option<HPET>,
    pub madt: Option<MADTInfo>,
}
impl ACPI {
    pub fn new(rsdp_addr: u64) -> Self {
        let xsdt_info = XSDTInfo::new(rsdp_addr).expect("Failed to parse XSDT");

        let hpet = xsdt_info.hpet.map(|addr| HPET::new(addr));
        let madt = xsdt_info.madt.map(|addr| MADTInfo::new(addr));

        Self { madt, hpet }
    }
}
