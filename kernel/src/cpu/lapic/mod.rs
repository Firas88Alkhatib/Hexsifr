use crate::{
    acpi::get_acpi,
    cpu::idt::{APIC_ERROR_VECTOR, SPURIOUS_VECTOR, TIMER_VECTOR},
    memory::phys_to_virt,
};

use acpi_crate::sdt::madt::{Madt, MadtEntry};
use core::ptr::read_volatile;
use raw_cpuid::CpuId;

use spin::Once;
use x2apic::lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode};
use x86_64::registers::model_specific::Msr;

// https://wiki.osdev.org/APIC

static LAPIC_VIRT_ADDR: Once<u64> = Once::new();
static IS_X2APIC: Once<bool> = Once::new();

fn get_lapic_virt() -> u64 {
    *LAPIC_VIRT_ADDR.call_once(|| {
        let binding = get_acpi().find_table::<Madt>().expect("Cannot read MADT table");
        let madt = binding.get();

        let lapic_phys_addr = madt
            .entries()
            .find_map(|e| match e {
                MadtEntry::LocalApicAddressOverride(o) => Some(o.local_apic_address),
                _ => None,
            })
            .unwrap_or(madt.local_apic_address as u64);
        phys_to_virt::<u8>(lapic_phys_addr) as u64
    })
}

pub fn new_lapic() -> LocalApic {
    LocalApicBuilder::new()
        .timer_vector(TIMER_VECTOR as usize)
        .error_vector(APIC_ERROR_VECTOR as usize)
        .spurious_vector(SPURIOUS_VECTOR as usize)
        .set_xapic_base(get_lapic_virt())
        .timer_initial(u32::MAX)
        .timer_divide(TimerDivide::Div16)
        .timer_mode(TimerMode::Periodic)
        .build()
        .expect("Failed to create LAPIC instance")
}

pub fn is_x2apic() -> bool {
    *IS_X2APIC.call_once(|| CpuId::new().get_feature_info().map(|f| f.has_x2apic()).unwrap_or(false))
}

pub fn read_lapic_error_status_register() -> u32 {
    if is_x2apic() {
        unsafe { Msr::new(0x828).read() as u32 }
    } else {
        let addr = (get_lapic_virt() + 0x28) as *const u32;
        unsafe { read_volatile(addr) }
    }
}
