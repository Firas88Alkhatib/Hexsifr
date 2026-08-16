use raw_cpuid::CpuId;

use x86_64::instructions::{interrupts::without_interrupts, port::Port};

use crate::{
    acpi::hpet::HPET,
    cpu::per_cpu::get_cpu_info,
    time::{read_tsc_counter, tsc_freq},
};

const PIT_FREQ: u64 = 1_193_182;
const PIT_CMD_PORT: u16 = 0x43;
const PIT_CH0_DATA: u16 = 0x40;

/// Try to measure the tsc frequency
pub fn calibrate_tsc_frequency() -> Option<u64> {
    if let Some(tsc_frequency) = CpuId::new().get_tsc_info().and_then(|i| i.tsc_frequency()) {
        return Some(tsc_frequency);
    }

    if let Some(hpet_freq) = HPET::new().ok().and_then(|h| h.calibrate_tsc()) {
        return Some(hpet_freq);
    }

    return calibrate_tsc_with_pit();
}

fn calibrate_tsc_with_pit() -> Option<u64> {
    without_interrupts(|| {
        let count: u16 = 0xFFFF;
        let mut cmd = Port::<u8>::new(PIT_CMD_PORT);
        let mut ch0 = Port::<u8>::new(PIT_CH0_DATA);

        unsafe {
            cmd.write(0x30);
            ch0.write((count & 0xFF) as u8);
            ch0.write((count >> 8) as u8);
        }

        let tsc_start = read_tsc_counter();

        loop {
            unsafe {
                cmd.write(0xE2);
                if ch0.read() & 0x80 != 0 {
                    break;
                }
            }

            core::hint::spin_loop();
        }

        let tsc_end = read_tsc_counter();

        let elapsed_tsc = tsc_end - tsc_start;
        if elapsed_tsc == 0 {
            return None;
        }
        Some((elapsed_tsc * PIT_FREQ) / count as u64)
    })
}

pub fn calibrate_lapic_timer() -> Option<u64> {
    without_interrupts(|| {
        let lapic = &mut get_cpu_info().local.as_mut().expect("Failed to get lapic from current per cpu info").lapic;
        let tsc_freq = tsc_freq();
        let tsc_ticks = tsc_freq / 20; // 50 ms

        unsafe { lapic.set_timer_initial(u32::MAX) }
        let tsc_start = read_tsc_counter();
        let lapic_start = unsafe { lapic.timer_current() };
        while read_tsc_counter().wrapping_sub(tsc_start) < tsc_ticks {
            core::hint::spin_loop();
        }
        let lapic_end = unsafe { lapic.timer_current() };

        let elapsed_lapic = lapic_start.wrapping_sub(lapic_end) as u64;
        if elapsed_lapic == 0 {
            return None;
        }
        let lapic_freq = elapsed_lapic.saturating_mul(tsc_freq) / tsc_ticks;
        let initial = (lapic_freq * 4 / 1000) as u32; // 4 ms
        unsafe {
            lapic.set_timer_initial(initial);
        }
        Some(lapic_freq)
    })
}
