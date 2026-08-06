use crate::acpi::{madt::get_usable_cpus_count, sdt::XSDTInfo};

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

pub(crate) fn get_usable_cpu_count(rsdp_addr: u64) -> usize {
    // https://wiki.osdev.org/RSDP

    let xsdt_info = XSDTInfo::new(rsdp_addr);
    let madt_addr = xsdt_info.expect("Failed to parse XSDT").madt.expect("MADT address not found");
    get_usable_cpus_count(madt_addr)
}
