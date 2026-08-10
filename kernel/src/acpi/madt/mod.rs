//https://wiki.osdev.org/MADT

pub mod madt_tables;
use crate::memory::phys_to_virt_unaligned;
use alloc::vec::Vec;
use madt_tables::*;

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub lapic_id: u32,
    pub logical_id: usize,
}
#[derive(Debug, Clone)]
pub struct IoApicInfo {
    pub id: u8,
    pub address: u32,
    pub global_irq_base: u32,
}
#[derive(Debug, Clone)]
pub struct InterruptOverride {
    pub bus: u8,
    pub source: u8,
    pub global_irq: u32,
    pub flags: u16,
}
#[derive(Debug, Clone)]
pub struct LocalApicNmi {
    pub acpi_processor_uid: u8,
    pub flags: u16,
    pub local_apic_lint: u8,
}
#[derive(Debug, Clone)]
pub(crate) struct MADTInfo {
    pub lapic_addr: u64,
    pub cpus: Vec<CpuInfo>,
    pub io_apics: Vec<IoApicInfo>,
    pub overrides: Vec<InterruptOverride>,
    pub nmi_sources: Vec<LocalApicNmi>,
}

impl MADTInfo {
    pub fn new(madt_address: u64) -> Self {
        let madt = phys_to_virt_unaligned::<Madt>(madt_address);

        let mut result = Self {
            lapic_addr: madt.local_apic_address as u64,
            cpus: Vec::new(),
            io_apics: Vec::new(),
            overrides: Vec::new(),
            nmi_sources: Vec::new(),
        };

        for (header, entry_address) in madt.entries(madt_address) {
            match header.entry_type {
                madt_entry_type::PROCESSOR_LOCAL_APIC => {
                    let lapic = unsafe { entry_address.cast::<MadtProcessorLocalApic>().read_unaligned() };
                    if !lapic.is_usable() {
                        continue;
                    }
                    result.cpus.push(CpuInfo { lapic_id: lapic.apic_id as u32, logical_id: result.cpus.len() });
                }
                madt_entry_type::PROCESSOR_LOCAL_X2APIC => {
                    let x2apic = unsafe { entry_address.cast::<MadtProcessorLocalX2Apic>().read_unaligned() };
                    if !x2apic.is_usable() {
                        continue;
                    }
                    result.cpus.push(CpuInfo { lapic_id: x2apic.x2apic_id, logical_id: result.cpus.len() });
                }
                madt_entry_type::IO_APIC => {
                    let io_apic = unsafe { entry_address.cast::<MadtIoApic>().read_unaligned() };
                    result.io_apics.push(IoApicInfo {
                        address: io_apic.io_apic_address,
                        global_irq_base: io_apic.global_system_interrupt_base,
                        id: io_apic.io_apic_id,
                    });
                }
                madt_entry_type::INTERRUPT_SOURCE_OVERRIDE => {
                    let iso = unsafe { entry_address.cast::<MadtInterruptSourceOverride>().read_unaligned() };
                    result.overrides.push(InterruptOverride {
                        bus: iso.bus,
                        flags: iso.flags,
                        global_irq: iso.global_system_interrupt,
                        source: iso.source,
                    });
                }
                madt_entry_type::LOCAL_APIC_NMI => {
                    let lapic_nmi = unsafe { entry_address.cast::<MadtLocalApicNmi>().read_unaligned() };
                    result.nmi_sources.push(LocalApicNmi {
                        acpi_processor_uid: lapic_nmi.acpi_processor_uid,
                        flags: lapic_nmi.flags,
                        local_apic_lint: lapic_nmi.local_apic_lint,
                    });
                }
                madt_entry_type::LOCAL_APIC_ADDRESS_OVERRIDE => {
                    let lapic_addr_override = unsafe { entry_address.cast::<MadtLocalApicAddressOverride>().read_unaligned() };
                    result.lapic_addr = lapic_addr_override.local_apic_address;
                }
                _ => {
                    info!("Unhandled MADT entry of type {}", header.entry_type);
                }
            }
        }

        result
    }
}
