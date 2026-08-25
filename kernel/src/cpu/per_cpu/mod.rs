use alloc::boxed::Box;
use raw_cpuid::CpuId;
use spin::Once;
use x2apic::lapic::LocalApic;

use x86_64::{
    VirtAddr,
    instructions::interrupts,
    registers::{
        control::{Cr4, Cr4Flags},
        model_specific::GsBase,
        segmentation::{GS, Segment64},
    },
};

use crate::{
    cpu::{
        gdt::{PerCpuGdt, create_gdt},
        get_cpu_id_by_lapic_id,
        idt::get_idt,
        lapic::new_lapic,
    },
    scheduler::Scheduler,
};

pub fn get_cpu_id() -> usize {
    unsafe { *get_gs_base().as_ptr::<CpuIndex>() }.as_usize()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuIndex(usize);
impl CpuIndex {
    pub const fn new(cpu_id: usize) -> Self {
        Self(cpu_id)
    }
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[repr(C)]
pub struct PerCPU {
    pub id: CpuIndex,
    pub gdt: &'static PerCpuGdt,
    pub lapic: LocalApic,
    pub scheduler: Scheduler,
}

pub fn per_cpu_init() {
    let mut lapic = new_lapic();
    unsafe {
        lapic.enable();
    }
    let lapic_id = unsafe { lapic.id() };
    let cpu_id = CpuIndex::new(get_cpu_id_by_lapic_id(lapic_id));
    set_gs_base(VirtAddr::from_ptr(&cpu_id));

    let gdt = create_gdt();
    gdt.load();

    set_gs_base(VirtAddr::from_ptr(&cpu_id));

    let scheduler = Scheduler::new();
    let per_cpu = PerCPU { id: cpu_id, lapic, gdt, scheduler };
    let static_per_cpu = Box::leak(Box::new(per_cpu));
    set_gs_base(VirtAddr::from_ptr(static_per_cpu));

    get_idt().load();
    interrupts::enable();
}

pub fn get_cpu_info() -> &'static mut PerCPU {
    let base = get_gs_base();
    let cpu_data = unsafe { &mut *base.as_mut_ptr::<PerCPU>() };
    cpu_data
}

struct GsBaseAccess {
    read: fn() -> VirtAddr,
    write: fn(virt_addr: VirtAddr),
}

static GS_BASE_ACCESS: Once<GsBaseAccess> = Once::new();

pub fn get_gs_base() -> VirtAddr {
    (GS_BASE_ACCESS.get().expect("failed to get gs base access").read)()
}
pub fn set_gs_base(value: VirtAddr) {
    let has_fsgsbase = CpuId::new().get_extended_feature_info().map(|ext| ext.has_fsgsbase()).unwrap_or(false);

    if has_fsgsbase {
        unsafe { Cr4::update(|f| f.insert(Cr4Flags::FSGSBASE)) }
    }

    let gs_base_access = GS_BASE_ACCESS.call_once(|| {
        if has_fsgsbase {
            GsBaseAccess { read: || GS::read_base(), write: |v| unsafe { GS::write_base(v) } }
        } else {
            GsBaseAccess { read: || GsBase::read(), write: |v| GsBase::write(v) }
        }
    });

    (gs_base_access.write)(value)
}
