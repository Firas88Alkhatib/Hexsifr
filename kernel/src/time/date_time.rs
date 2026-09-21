const NANOS_PER_MICRO: u64 = 1_000;
const NANOS_PER_MILLI: u64 = 1_000_000;
const NANOS_PER_SEC: u64 = 1_000_000_000;
const SECS_PER_MIN: u64 = 60;
const SECS_PER_HOUR: u64 = 3600;
const SECS_PER_DAY: u64 = 86400;

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millis: u16,
    pub micros: u16,
    pub nanos: u16,
}

impl DateTime {
    pub fn to_nanos(&self) -> u64 {
        // --- Convert calendar date to days since Unix epoch ---
        // Algorithm: "Days From Civil" (Hinnant)
        let y = self.year as i64 - (self.month <= 2) as i64;
        let m = self.month as i64 + if self.month <= 2 { 9 } else { -3 };
        let d = self.day as i64;

        let era = y / 400;
        let yoe = y - era * 400;
        let doy = (153 * m + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;

        let days_since_epoch = era * 146097 + doe - 719468;

        // --- Convert time-of-day to seconds ---
        let seconds = days_since_epoch as u64 * SECS_PER_DAY
            + (self.hour as u64) * SECS_PER_HOUR
            + (self.minute as u64) * SECS_PER_MIN
            + (self.second as u64);

        // --- Convert sub-second fields ---
        let nanos = seconds * NANOS_PER_SEC
            + (self.millis as u64) * NANOS_PER_MILLI
            + (self.micros as u64) * NANOS_PER_MICRO
            + (self.nanos as u64);

        nanos
    }
    pub fn from_nanos(ts: u64) -> Self {
        let nanos = (ts % NANOS_PER_MICRO) as u16;
        let micros = ((ts / NANOS_PER_MICRO) % 1000) as u16;
        let millis = ((ts / NANOS_PER_MILLI) % 1000) as u16;
        let total_seconds = ts / NANOS_PER_SEC;

        let days_since_epoch = total_seconds / SECS_PER_DAY;
        let sec_of_day = total_seconds % SECS_PER_DAY;

        let hour = (sec_of_day / SECS_PER_HOUR) as u8;
        let minute = ((sec_of_day % SECS_PER_HOUR) / SECS_PER_MIN) as u8;
        let second = (sec_of_day % SECS_PER_MIN) as u8;

        // --- Convert days since epoch to calendar date ---
        // Algorithm: "Civil From Days" (Hinnant), used by chrono, C++20, Rust core.
        let z = days_since_epoch as i64 + 719468;
        let era = (z >= 0).then(|| z / 146097).unwrap_or((z - 146096) / 146097);
        let doe = z - era * 146097; // day of era
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year
        let mp = (5 * doy + 2) / 153; // month period
        let d = doy - (153 * mp + 2) / 5 + 1; // day of month
        let m = mp + if mp < 10 { 3 } else { -9 }; // month (1–12)
        let year = (y + (m <= 2) as i64) as u16;

        DateTime { year, month: m as u8, day: d as u8, hour, minute, second, millis, micros, nanos }
    }
}
