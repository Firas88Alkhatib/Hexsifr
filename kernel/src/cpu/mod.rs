use core::slice::from_raw_parts_mut;

use acpi_crate::sdt::madt::{Madt, MadtEntry};
use bootloader_api::info::MemoryRegions;
use x86_64::VirtAddr;

use crate::{
    acpi::{get_acpi, get_cpu_count, is_lapic_enabled},
    cpu::{
        lapic::new_lapic,
        per_cpu::{CpuIndex, set_gs_base},
    },
    memory::{phys_to_virt_mut, physical::reserve_memory},
};

pub(crate) mod ap;
pub(crate) mod gdt;
pub(crate) mod idt;
pub(crate) mod lapic;
pub(crate) mod per_cpu;

static CPU_MAP: spin::Once<&'static mut [u32]> = spin::Once::new();

pub fn get_cpu_id_by_lapic_id(lapic_id: u32) -> usize {
    CPU_MAP.get().expect("CPU_MAP is not initialized").iter().position(|&i| i == lapic_id).expect("Cannot find cpu id by lapic id")
}
pub fn init_cpu_map(memory_regions: &mut MemoryRegions) {
    let mut lapic = new_lapic();
    unsafe {
        lapic.enable();
        if !lapic.is_bsp() {
            panic!("Cannot initialize cpu map on AP")
        }
    };

    let cpu_count = get_cpu_count();
    let size = cpu_count * size_of::<u32>();

    let address = reserve_memory(memory_regions, size).expect("failed to reserve CPU map memory");
    let ptr = phys_to_virt_mut::<u32>(address);
    let cpu_map = unsafe { from_raw_parts_mut(ptr, cpu_count) };

    let binding = get_acpi().find_table::<Madt>().expect("Failed to get MADT table");
    let madt = binding.get();

    let bsp_lapic_id = unsafe { lapic.id() };

    let mut current_index = 1;

    for entry in madt.entries() {
        let (lapic_id, flags) = match entry {
            MadtEntry::LocalApic(lapic) => (lapic.apic_id as u32, lapic.flags),
            MadtEntry::LocalX2Apic(x2apic) => (x2apic.x2apic_id, x2apic.flags),
            _ => continue,
        };
        if is_lapic_enabled(flags) {
            if lapic_id == bsp_lapic_id {
                cpu_map[0] = lapic_id
            } else {
                cpu_map[current_index] = lapic_id;
                current_index += 1;
            }
        }
    }

    CPU_MAP.call_once(|| cpu_map);
}

pub(crate) fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
static BSP_CPU_ID: CpuIndex = CpuIndex::new(0);

pub fn cpu_init(memory_regions: &mut MemoryRegions) {
    init_cpu_map(memory_regions);
    set_gs_base(VirtAddr::from_ptr(&BSP_CPU_ID));
}
