use core::{mem::MaybeUninit, ptr::addr_of_mut};

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

use crate::cpu::{
    gdt::{PerCpuGdt, get_bsp_per_cpu_gdt},
    idt::get_idt,
    lapic::new_lapic,
};

static FSGSBASE_SUPPORTED: Once<bool> = Once::new();

static mut BSP_PER_CPU: MaybeUninit<PerCPU> = MaybeUninit::uninit();

#[derive(Debug)]
pub struct PerCPUShared {
    pub id: u32,
    pub lapic_id: u32,
}

#[derive(Debug)]
pub struct PerCPULocal {
    pub gdt: &'static PerCpuGdt,
    pub lapic: LocalApic,
}
#[derive(Debug)]
pub struct PerCPU {
    pub local: PerCPULocal,
    pub shared: PerCPUShared,
}

pub fn new_per_cpu() -> &'static mut PerCPU {
    let mut lapic = new_lapic();

    if unsafe { lapic.is_bsp() } {
        let gdt = get_bsp_per_cpu_gdt();

        gdt.load();
        get_idt().load();
        unsafe {
            lapic.enable();
            let lapic_id = lapic.id();
            let bsp_per_cpu = PerCPU { local: PerCPULocal { gdt, lapic }, shared: PerCPUShared { id: 0, lapic_id } };
            let ptr: &'static mut PerCPU = (*addr_of_mut!(BSP_PER_CPU)).write(bsp_per_cpu);
            set_gs_base(VirtAddr::from_ptr(ptr));
            return ptr;
        };
    } else {
        panic!("Initializing PerCPU for AP is not implemented yet")
    };
}

fn get_gs_base() -> VirtAddr {
    if fsgsbase_supported() { GS::read_base() } else { GsBase::read() }
}
fn set_gs_base(value: VirtAddr) {
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

pub fn get_per_cpu_info() -> &'static mut PerCPU {
    let base = get_gs_base();
    let cpu_data = unsafe { &mut *base.as_mut_ptr::<PerCPU>() };
    cpu_data
}
