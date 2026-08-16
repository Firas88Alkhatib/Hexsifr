use core::{mem::MaybeUninit, ptr::addr_of_mut};

use acpi_crate::sdt::madt::{Madt, MadtEntry};
use alloc::vec::Vec;
use raw_cpuid::CpuId;
use spin::Once;
use x2apic::lapic::LocalApic;
use x86_64::{
    VirtAddr,
    registers::{
        control::{Cr4, Cr4Flags},
        model_specific::GsBase,
        segmentation::{GS, Segment64},
    },
};

use crate::{
    acpi::{get_acpi, is_lapic_enabled},
    cpu::{
        gdt::{PerCpuGdt, get_bsp_per_cpu_gdt},
        idt::get_idt,
        lapic::new_lapic,
    },
};

static FSGSBASE_SUPPORTED: Once<bool> = Once::new();

static mut BSP_PER_CPU: MaybeUninit<PerCPU> = MaybeUninit::uninit();

static PER_CPU_DATA: Once<Vec<PerCPUShared>> = Once::new();

#[derive(Debug, Clone)]
pub struct PerCPUShared {
    pub id: u32,
    pub lapic_id: u32,
    pub is_bsp: bool,
}

#[derive(Debug)]
pub struct PerCPULocal {
    pub gdt: &'static PerCpuGdt,
    pub lapic: LocalApic,
}
#[derive(Debug)]
pub struct PerCPU {
    pub local: Option<PerCPULocal>,
    pub shared: PerCPUShared,
}

pub fn init_per_cpu_data() {
    PER_CPU_DATA.call_once(|| {
        let binding = get_acpi().find_table::<Madt>().expect("Failed to get MADT table");
        let madt = binding.get();

        let bsp_lapic_id = get_cpu_info().shared.lapic_id;

        let mut cpus_data = Vec::<PerCPUShared>::new();
        let mut cpu_id = 1;

        let mut handle_lapic = |lapic_id: u32, flags: u32| {
            if !is_lapic_enabled(flags) {
                return;
            }
            let is_bsp = lapic_id == bsp_lapic_id;
            let id = if is_bsp { 0 } else { cpu_id };
            cpus_data.push(PerCPUShared { id, lapic_id, is_bsp });

            if !is_bsp {
                cpu_id += 1;
            }
        };

        for entry in madt.entries() {
            match entry {
                MadtEntry::LocalApic(lapic) => handle_lapic(lapic.apic_id as u32, lapic.flags),
                MadtEntry::LocalX2Apic(x2apic) => handle_lapic(x2apic.x2apic_id, x2apic.flags),
                _ => {}
            }
        }
        cpus_data
    });
}

pub fn get_per_cpu_data() -> &'static Vec<PerCPUShared> {
    PER_CPU_DATA.get().expect("Failed to get per cpu data")
}
pub fn get_per_cpu_data_by_lapic_id(lapic_id: u32) -> Option<&'static PerCPUShared> {
    get_per_cpu_data().iter().find(|cpu| cpu.lapic_id == lapic_id)
}

pub fn init_bsp_per_cpu() -> &'static mut PerCPU {
    let mut lapic = new_lapic();

    unsafe {
        lapic.enable();
        let lapic_id = lapic.id();
        let is_bsp = lapic.is_bsp();
        if !is_bsp {
            panic!("trying to initialize BPS PerCpu for AP");
        }

        let gdt = get_bsp_per_cpu_gdt();
        gdt.load();
        get_idt().load();
        let local = Some(PerCPULocal { gdt, lapic });
        let shared = PerCPUShared { id: 0, is_bsp, lapic_id };
        let per_cpu = PerCPU { local, shared };
        let ptr: &'static mut PerCPU = (*addr_of_mut!(BSP_PER_CPU)).write(per_cpu);
        set_gs_base(VirtAddr::from_ptr(ptr));
        return ptr;
    }
}

fn get_gs_base() -> VirtAddr {
    if fsgsbase_supported() { GS::read_base() } else { GsBase::read() }
}
pub fn set_gs_base(value: VirtAddr) {
    if fsgsbase_supported() {
        unsafe { Cr4::update(|f| f.insert(Cr4Flags::FSGSBASE)) };
        unsafe { GS::write_base(value) }
    } else {
        GsBase::write(value)
    };
}

fn fsgsbase_supported() -> bool {
    *FSGSBASE_SUPPORTED.call_once(|| {
        let is_supported = CpuId::new().get_extended_feature_info().map(|ext| ext.has_fsgsbase()).unwrap_or(false);
        unsafe { Cr4::update(|f| f.insert(Cr4Flags::FSGSBASE)) };
        is_supported
    })
}

pub fn get_cpu_info() -> &'static mut PerCPU {
    let base = get_gs_base();
    let cpu_data = unsafe { &mut *base.as_mut_ptr::<PerCPU>() };
    cpu_data
}
