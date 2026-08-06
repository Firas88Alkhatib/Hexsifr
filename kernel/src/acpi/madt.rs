//https://wiki.osdev.org/MADT

use crate::memory::{phys_to_virt, phys_to_virt_unaligned};

use super::AcpiHeader;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Madt {
    pub header: AcpiHeader,
    pub local_apic_address: u32,
    pub flags: u32,
}
impl Madt {
    pub fn entries(&self, madt_address: u64) -> MadtEntryIterator {
        let entries_start = unsafe { phys_to_virt::<u8>(madt_address).add(size_of::<Madt>()) };
        let entries_end = unsafe { phys_to_virt::<u8>(madt_address).add(self.header.length as usize) };
        MadtEntryIterator { current: entries_start, end: entries_end }
    }
}

pub struct MadtEntryIterator {
    current: *const u8,
    end: *const u8,
}

impl Iterator for MadtEntryIterator {
    type Item = (MadtEntryHeader, *const u8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }
        let header = unsafe { self.current.cast::<MadtEntryHeader>().read_unaligned() };
        let data_ptr = self.current;
        self.current = unsafe { self.current.add(header.length as usize) };
        Some((header, data_ptr))
    }
}

pub mod madt_entry_type {
    pub const PROCESSOR_LOCAL_APIC: u8 = 0;
    pub const IO_APIC: u8 = 1;
    pub const INTERRUPT_SOURCE_OVERRIDE: u8 = 2;
    pub const LOCAL_APIC_NMI: u8 = 4;
    pub const LOCAL_APIC_ADDRESS_OVERRIDE: u8 = 5;
    // pub const IO_SAPIC: u8 = 6;
    // pub const LOCAL_SAPIC: u8 = 7;
    // pub const PLATFORM_INTERRUPT_SOURCES: u8 = 8;
    pub const PROCESSOR_LOCAL_X2APIC: u8 = 9;
    // pub const LOCAL_X2APIC_NMI: u8 = 10;
    // pub const GIC: u8 = 11;
    // pub const GIC_DISTRIBUTOR: u8 = 12;
    // pub const GIC_MSI_FRAME: u8 = 13;
    // pub const GIC_REDISTRIBUTOR: u8 = 14;
    // pub const GIC_INTERRUPT_TRANSLATION_SERVICE: u8 = 15;
    // pub const MULTIPROCESSOR_WAKEUP: u8 = 16;
    // pub const CORE_PIC: u8 = 17;
    // pub const LEGACY_IO_PIC: u8 = 18;
    // pub const HYPERTRANSPORT_PIC: u8 = 19;
    // pub const EXTEND_IO_PIC: u8 = 20;
    // pub const MSI_PIC: u8 = 21;
    // pub const BRIDGE_IO_PIC: u8 = 22;
    // pub const LOW_PIN_COUNT_PIC: u8 = 23;
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtEntryHeader {
    pub entry_type: u8,
    pub length: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtProcessorLocalApic {
    pub header: MadtEntryHeader,
    pub acpi_processor_uid: u8,
    pub apic_id: u8,
    pub flags: u32,
}
impl MadtProcessorLocalApic {
    pub(crate) fn is_enabled(&self) -> bool {
        self.flags & 1 != 0
    }
    pub(crate) fn is_online_capable(&self) -> bool {
        self.flags & 2 != 0
    }
    pub(crate) fn is_usable(&self) -> bool {
        self.is_enabled() || self.is_online_capable()
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtIoApic {
    pub header: MadtEntryHeader,
    pub io_apic_id: u8,
    pub reserved: u8,
    pub io_apic_address: u32,
    pub global_system_interrupt_base: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtInterruptSourceOverride {
    pub header: MadtEntryHeader,
    pub bus: u8,
    pub source: u8,
    pub global_system_interrupt: u32,
    pub flags: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalApicNmi {
    pub header: MadtEntryHeader,
    pub acpi_processor_uid: u8,
    pub flags: u16,
    pub local_apic_lint: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtLocalApicAddressOverride {
    pub header: MadtEntryHeader,
    pub reserved: u16,
    pub local_apic_address: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MadtProcessorLocalX2Apic {
    pub header: MadtEntryHeader,
    pub reserved: u16,
    pub x2apic_id: u32,
    pub flags: u32,
    pub acpi_processor_uid: u32,
}
impl MadtProcessorLocalX2Apic {
    pub(crate) fn is_enabled(&self) -> bool {
        self.flags & 1 != 0
    }
    pub(crate) fn is_online_capable(&self) -> bool {
        self.flags & 2 != 0
    }
    pub(crate) fn is_usable(&self) -> bool {
        self.is_enabled() || self.is_online_capable()
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct MADT {
    lapic_addr: u64,
}

impl MADT {
    pub fn new(madt_address: u64) -> Result<Self, &'static str> {
        let madt = phys_to_virt_unaligned::<Madt>(madt_address);

        let mut lapic_addr = madt.local_apic_address as u64;

        for (header, entry_address) in madt.entries(madt_address) {
            match header.entry_type {
                madt_entry_type::PROCESSOR_LOCAL_APIC => {
                    let lapic = unsafe { entry_address.cast::<MadtProcessorLocalApic>().read_unaligned() };
                    info!("Found LAPIC: {:?}", lapic);
                }
                madt_entry_type::IO_APIC => {
                    let io_apic = unsafe { entry_address.cast::<MadtIoApic>().read_unaligned() };
                    info!("Found I/O APIC: {:?}", io_apic);
                }
                madt_entry_type::INTERRUPT_SOURCE_OVERRIDE => {
                    let iso = unsafe { entry_address.cast::<MadtInterruptSourceOverride>().read_unaligned() };
                    info!("Found Interrupt Source Override: {:?}", iso);
                }
                madt_entry_type::LOCAL_APIC_NMI => {
                    let lapic_nmi = unsafe { entry_address.cast::<MadtLocalApicNmi>().read_unaligned() };
                    info!("Found Local APIC NMI: {:?}", lapic_nmi);
                }
                madt_entry_type::LOCAL_APIC_ADDRESS_OVERRIDE => {
                    let lapic_addr_override = unsafe { entry_address.cast::<MadtLocalApicAddressOverride>().read_unaligned() };
                    info!("Found Local APIC Address Override: {:?}", lapic_addr_override);
                    lapic_addr = lapic_addr_override.local_apic_address;
                }
                madt_entry_type::PROCESSOR_LOCAL_X2APIC => {
                    let x2apic = unsafe { entry_address.cast::<MadtProcessorLocalX2Apic>().read_unaligned() };
                    info!("Found Processor Local X2APIC: {:?}", x2apic);
                }
                _ => {}
            }
        }

        Ok(Self { lapic_addr })
    }
}

pub fn get_usable_cpus_count(madt_address: u64) -> usize {
    let mut cpus_count: usize = 0;
    let madt = phys_to_virt_unaligned::<Madt>(madt_address);

    for (header, entry_address) in madt.entries(madt_address) {
        match header.entry_type {
            madt_entry_type::PROCESSOR_LOCAL_APIC => {
                cpus_count += unsafe { entry_address.cast::<MadtProcessorLocalApic>().read_unaligned() }.is_usable() as usize;
            }

            madt_entry_type::PROCESSOR_LOCAL_X2APIC => {
                cpus_count += unsafe { entry_address.cast::<MadtProcessorLocalX2Apic>().read_unaligned() }.is_usable() as usize;
            }
            _ => {}
        }
    }

    if cpus_count < 1 {
        panic!("Invalid cpus count: {}", cpus_count)
    }
    cpus_count
}
