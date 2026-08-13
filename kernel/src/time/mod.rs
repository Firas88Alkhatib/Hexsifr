mod calibration;
mod cmos;
pub mod date_time;

use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use crate::acpi::{fadt::FADT, hpet::HPET};
use crate::time::calibration::{calibrate_tsc_frequency, calibrate_lapic_timer};
use crate::time::date_time::DateTime;

static TSC_FREQ: AtomicU64 = AtomicU64::new(0); // ticks per second
static BOOT_TSC: AtomicU64 = AtomicU64::new(0);
static BOOT_TIME_SECS: AtomicU64 = AtomicU64::new(0);
static BOOT_TIME_NANOS: AtomicU64 = AtomicU64::new(0);

pub fn time_init(hpet: Option<HPET>, fadt: Option<FADT>) {
    let tsc_freq = calibrate_tsc_frequency(hpet).expect("Failed to calibrate TSC frequency");
    let boot_tsc = read_tsc_counter();
    TSC_FREQ.store(tsc_freq, Ordering::Relaxed);
    BOOT_TSC.store(boot_tsc, Ordering::Relaxed);
    let bios_date_time_nanos = cmos::read_rtc(fadt.map(|f| f.table.century)).to_nanos();

    BOOT_TIME_SECS.store(bios_date_time_nanos / 1_000_000_000, Ordering::Relaxed);
    BOOT_TIME_NANOS.store(bios_date_time_nanos % 1_000_000_000, Ordering::Relaxed);
    calibrate_lapic_timer();
}

pub fn read_tsc_counter() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub fn tsc_freq() -> u64 {
    TSC_FREQ.load(Ordering::Relaxed)
}
pub fn boot_tsc() -> u64 {
    BOOT_TSC.load(Ordering::Relaxed)
}
pub fn boot_ns() -> u128 {
    let boot_secs = BOOT_TIME_SECS.load(Ordering::Relaxed) as u128;
    let boot_ns = BOOT_TIME_NANOS.load(Ordering::Relaxed) as u128;
    (boot_secs * 1_000_000_000u128) + boot_ns
}
pub fn uptime_ns() -> u128 {
    let frequency = tsc_freq() as u128;
    let ticks = (read_tsc_counter() - boot_tsc()) as u128;
    (ticks * 1_000_000_000u128) / frequency
}

pub fn uptime_duration() -> Duration {
    Duration::from_nanos(uptime_ns() as u64)
}
pub fn current_time_ns() -> u128 {
    boot_ns() + uptime_ns()
}

pub fn current_date_time() -> DateTime {
    DateTime::from_nanos(current_time_ns() as u64)
}

pub fn sleep_ms(ms: u64) {
    let ticks = (tsc_freq() as u128 * ms as u128) / 1000;
    let start = read_tsc_counter() as u128;
    while read_tsc_counter() as u128 - start < ticks {
        core::hint::spin_loop();
    }
}
