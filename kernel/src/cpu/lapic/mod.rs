use crate::memory::phys_to_virt;
use core::{
    ptr::read_volatile,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use raw_cpuid::CpuId;

use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::registers::model_specific::Msr;

// https://wiki.osdev.org/APIC

static LAPIC_VIRT_ADDR: AtomicU64 = AtomicU64::new(0);
static IS_X2APIC: AtomicBool = AtomicBool::new(false);

pub const TIMER_VECTOR: u8 = 32;
pub const APIC_ERROR_VECTOR: u8 = 33;
pub const SPURIOUS_VECTOR: u8 = 255;

pub fn eoi() {
    unsafe { get_lapic().end_of_interrupt() };
}

pub fn get_current_lapic_id() -> u32 {
    unsafe { get_lapic().id() }
}

pub fn init_lapic(lapic_phys_addr: u64) {
    let lapic_virt_addr = phys_to_virt::<u8>(lapic_phys_addr) as u64;
    LAPIC_VIRT_ADDR.store(lapic_virt_addr, Ordering::Relaxed);

    unsafe { get_lapic().enable() };

    if let Some(info) = CpuId::new().get_feature_info() {
        IS_X2APIC.store(info.has_x2apic(), Ordering::Relaxed);
    };
}

fn get_lapic() -> LocalApic {
    let virt = LAPIC_VIRT_ADDR.load(Ordering::Relaxed);
    LocalApicBuilder::new()
        .timer_vector(TIMER_VECTOR as usize)
        .error_vector(APIC_ERROR_VECTOR as usize)
        .spurious_vector(SPURIOUS_VECTOR as usize)
        .set_xapic_base(virt)
        .build()
        .expect("Failed to create LAPIC instance")
}

pub fn is_x2apic() -> bool {
    IS_X2APIC.load(Ordering::Relaxed)
}

pub fn read_lapic_error_statu_register() -> u32 {
    if is_x2apic() {
        unsafe { Msr::new(0x828).read() as u32 }
    } else {
        let addr = (LAPIC_VIRT_ADDR.load(Ordering::Relaxed) + 0x28) as *const u32;
        unsafe { read_volatile(addr) }
    }
}
