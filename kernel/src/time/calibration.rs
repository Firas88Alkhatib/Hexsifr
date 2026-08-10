use raw_cpuid::CpuId;
use x86_64::instructions::{interrupts::without_interrupts, port::Port};

use crate::{acpi::hpet::HPET, time::read_tsc};

const PIT_FREQ: u64 = 1_193_182;
const PIT_CMD_PORT: u16 = 0x43;
const PIT_CH0_DATA: u16 = 0x40;

pub fn calibrate_tsc(hpet: Option<HPET>) -> Option<u64> {
    if let Some(tsc_frequency) = CpuId::new().get_tsc_info().and_then(|i| i.tsc_frequency()) {
        return Some(tsc_frequency);
    }

    if let Some(hpet_freq) = hpet.and_then(|h| h.calibrate_tsc()) {
        return Some(hpet_freq);
    }

    return calibrate_tsc_with_pit();
}

fn calibrate_tsc_with_pit() -> Option<u64> {
    without_interrupts(|| {
        let count: u16 = 0xFFFF;
        let mut cmd = Port::<u8>::new(PIT_CMD_PORT);
        let mut data = Port::<u8>::new(PIT_CH0_DATA);

        unsafe {
            cmd.write(0x30);
            data.write((count & 0xFF) as u8);
            data.write((count >> 8) as u8);
        }

        let tsc_start = read_tsc();

        loop {
            unsafe {
                cmd.write(0xE2);
                if data.read() & 0x80 != 0 {
                    break;
                }
            }

            core::hint::spin_loop();
        }

        let tsc_end = read_tsc();

        let elapsed_tsc = tsc_end - tsc_start;
        if elapsed_tsc == 0 {
            return None;
        }
        Some((elapsed_tsc * PIT_FREQ) / count as u64)
    })
}
