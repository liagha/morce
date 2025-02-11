use chrono::{DateTime, Datelike, FixedOffset, Local, Utc};

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CompactTimestamp {
    #[prost(int32, tag = "1")]
    pub seconds: i32,
}

pub trait TimeConversion {
    fn to_timestamp(&self) -> CompactTimestamp {
        CompactTimestamp {
            seconds: 0,
        }
    }
    fn to_datetime(&self) -> DateTime<Utc> {
        DateTime::default()
    }

    fn is_persian(&self) -> bool {
        false
    }

    fn convert_to_jalali(&mut self) {}

    fn to_local(&self) -> DateTime<Local> {
        DateTime::default()
    }

    fn gregorian_to_jalali(g_y: i32, g_m: u32, g_d: u32) -> (i32, u32, u32) {
        let g_days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let j_month_days = [31, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 29];

        let mut g_day_no = 365 * (g_y - 1600) + ((g_y - 1600) / 4) - ((g_y - 1600) / 100) + ((g_y - 1600) / 400);

        for i in 0..(g_m - 1) as usize {
            g_day_no += g_days_in_month[i];
        }

        if g_m > 2 && ((g_y % 4 == 0 && g_y % 100 != 0) || (g_y % 400 == 0)) {
            g_day_no += 1;
        }

        g_day_no += g_d as i32 - 1;
        let j_day_no = g_day_no - 79;
        let j_np = j_day_no / 12053;
        let mut j_day_no = j_day_no % 12053;

        let mut j_y = 979 + (33 * j_np) + (4 * (j_day_no / 1461));
        j_day_no %= 1461;

        if j_day_no >= 366 {
            j_y += (j_day_no - 1) / 365;
            j_day_no = (j_day_no - 1) % 365;
        }

        let mut j_m = 0;
        while j_m < 11 && j_day_no >= j_month_days[j_m] {
            j_day_no -= j_month_days[j_m];
            j_m += 1;
        }

        (j_y, j_m as u32 + 1, j_day_no as u32 + 2)
    }
}

impl TimeConversion for DateTime<Utc> {
    fn to_timestamp(&self) -> CompactTimestamp {
        CompactTimestamp {
            seconds: self.timestamp() as i32,
        }
    }
    fn convert_to_jalali(&mut self) {
        let (year, month, day) = (self.year(), self.month(), self.day());
        let (j_year, j_month, j_day) = Self::gregorian_to_jalali(year, month, day);
        *self = self.with_year(j_year).unwrap();
        *self = self.with_month(j_month).unwrap();
        *self = self.with_day(j_day).unwrap();
    }

    fn to_local(&self) -> DateTime<Local> {
        let mut time = self.with_timezone(&Local);

        if time.is_persian() {
            time.convert_to_jalali()
        }

        time
    }
}

impl TimeConversion for DateTime<Local> {
    fn to_timestamp(&self) -> CompactTimestamp {
        CompactTimestamp {
            seconds: self.timestamp() as i32,
        }
    }
    fn is_persian(&self) -> bool {
        *self.offset() == FixedOffset::east_opt(3 * 3600 + 30 * 60).unwrap()
    }
    fn convert_to_jalali(&mut self) {
        let (year, month, day) = (self.year(), self.month(), self.day());
        let (j_year, j_month, j_day) = Self::gregorian_to_jalali(year, month, day);
        *self = self.with_year(j_year).unwrap();
        *self = self.with_month(j_month).unwrap();
        *self = self.with_day(j_day).unwrap();
    }
}

impl TimeConversion for CompactTimestamp {
    fn to_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.seconds as i64, 0).unwrap_or_default()
    }
}