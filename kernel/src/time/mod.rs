mod calibration;
mod cmos;

use core::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{acpi::hpet::HPET, time::calibration::calibrate_tsc};

static TSC_FREQ: AtomicU64 = AtomicU64::new(0);
static BOOT_TSC: AtomicU64 = AtomicU64::new(0);

pub fn time_init(hpet: Option<HPET>) {
    let tsc_freq = calibrate_tsc(hpet).expect("Failed to calibrate TSC frequency");
    let boot_tsc = read_tsc();
    TSC_FREQ.store(tsc_freq, Ordering::Relaxed);
    BOOT_TSC.store(boot_tsc, Ordering::Relaxed);
}

pub fn tsc_freq() -> u64 {
    TSC_FREQ.load(Ordering::Relaxed)
}
pub fn read_tsc() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}
pub fn boot_tsc() -> u64 {
    BOOT_TSC.load(Ordering::Relaxed)
}

pub fn uptime_ns() -> u64 {
    let frequency = tsc_freq() as u128;
    let ticks = (read_tsc() - boot_tsc()) as u128;

    ((ticks * 1_000_000_000) / frequency) as u64
}

pub fn uptime_duration() -> Duration {
    Duration::from_nanos(uptime_ns())
}

pub fn sleep_ms(ms: u64) {
    let ticks = (tsc_freq() * ms) / 1000;
    let start = read_tsc();
    while read_tsc() - start < ticks {
        core::hint::spin_loop();
    }
}
