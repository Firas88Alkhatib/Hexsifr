use acpi_crate::{
    AcpiTables,
    sdt::madt::{Madt, MadtEntry},
};
use spin::Once;

use crate::acpi::acpi_handler::AcpiHandler;

pub(crate) mod acpi_handler;
pub(crate) mod hpet;

static ACPI_TABLES: Once<AcpiTables<AcpiHandler>> = Once::new();

pub fn acpi_init(rsdp_address: usize) {
    ACPI_TABLES.call_once(|| unsafe { AcpiTables::from_rsdp(AcpiHandler, rsdp_address).expect("Failed to parse ACPI tables") });
}

pub fn get_acpi() -> &'static AcpiTables<AcpiHandler> {
    ACPI_TABLES.get().expect("Failed to get ACPI tables")
}

pub fn get_cpu_count() -> usize {
    let binding = get_acpi().find_table::<Madt>().expect("Cannot read MADT table");
    let madt = binding.get();

    madt.entries()
        .filter(|entry| match entry {
            MadtEntry::LocalApic(apic) => is_lapic_enabled(apic.flags),
            MadtEntry::LocalX2Apic(apic) => is_lapic_enabled(apic.flags),
            _ => false,
        })
        .count()
}
fn is_lapic_enabled(flags: u32) -> bool {
    (flags & 0x01) != 0
}
