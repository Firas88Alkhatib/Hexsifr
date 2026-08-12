use x86_64::instructions::{interrupts::without_interrupts, port::Port};

use crate::time::date_time::DateTime;

const CMOS_ADDR: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x02;
const REG_HOURS: u8 = 0x04;
const REG_DAY: u8 = 0x07;
const REG_MONTH: u8 = 0x08;
const REG_YEAR: u8 = 0x09;
const REG_STATUS_A: u8 = 0x0A;
const REG_STATUS_B: u8 = 0x0B;

fn bcd_to_binary(value: u8) -> u8 {
    (value & 0x0F) + ((value >> 4) * 10)
}
fn read_cmos(reg: u8) -> u8 {
    unsafe {
        Port::<u8>::new(CMOS_ADDR).write(reg);
        Port::<u8>::new(CMOS_DATA).read()
    }
}
fn is_update_in_progress() -> bool {
    (read_cmos(REG_STATUS_A) & 0x80) != 0
}

pub fn read_rtc(fadt_century_reg: Option<u8>) -> DateTime {
    let is_binary_mode = read_cmos(REG_STATUS_B) & 0x04 != 0;
    let read_value = |reg: u8| -> u8 {
        let value = read_cmos(reg);
        if is_binary_mode { value } else { bcd_to_binary(value) }
    };

    without_interrupts(|| {
        while is_update_in_progress() {
            core::hint::spin_loop();
        }

        let second = read_value(REG_SECONDS);
        let minute = read_value(REG_MINUTES);
        let hour = read_value(REG_HOURS);
        let day = read_value(REG_DAY);
        let month = read_value(REG_MONTH);
        let year_raw = read_value(REG_YEAR) as u16;

        let century =
            fadt_century_reg.map(|reg| read_value(reg) as u16).filter(|&c| c != 0).unwrap_or_else(|| if year_raw >= 90 { 19 } else { 20 });

        let year = (century * 100) + year_raw as u16;

        DateTime { year, month, day, hour, minute, second, millis: 0, micros: 0, nanos: 0 }
    })
}
