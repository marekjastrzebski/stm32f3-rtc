/// Trait that determinate time write and access
pub trait TimeAccess {
    fn time(&self) -> Time;
    fn set_time(&mut self, time: Time);
}

/// Trait that determinate date write and access
pub trait DateAccess {
    fn date(&self) -> Date;
    fn set_date(&mut self, date: Date);
}

/// Keeps date in struct with easy access
pub struct Date {
    pub day: u8,
    pub month: u8,
    pub year: u32,
}

impl Date {
    /// Create a new Date struct from fallowing arguments,
    /// (day, month, year)
    pub fn from(day: u8, month: u8, year: u32) -> Date {
        Date { day, month, year }
    }
}

/// Keeps time in struct with easy access
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Time {
    /// Create a new Time struct from following arguments
    /// (hour, minute, second)
    pub fn from(hour: u8, minute: u8, second: u8) -> Time {
        Time {
            hour,
            minute,
            second,
        }
    }
}

/// Single BCD encoded value
pub struct Bcd<T> {
    pub(crate) tens: T,
    pub(crate) units: T,
}

impl BcdConvert for Bcd<u8> {}

impl Bcd<u8> {
    pub fn get(&self) -> u8 {
        Self::bcd_decode(self)
    }

    pub fn set(value: u8) -> Self {
        Self::bcd_encode(value)
    }
}

/// Keeps BCD time
pub(crate) struct BcdTime {
    pub(crate) hour: Bcd<u8>,
    pub(crate) minutes: Bcd<u8>,
    pub(crate) seconds: Bcd<u8>,
}

impl BcdTime {
    /// Returns the time in Time struct
    pub(crate) fn time(&self) -> Time {
        Time {
            hour: self.hour.get(),
            minute: self.minutes.get(),
            second: self.seconds.get(),
        }
    }
}

impl BcdConvert for BcdTime {}

/// API for easy create BCD time from Time struct
impl From<Time> for BcdTime {
    /// Create BCD time from Time struct
    fn from(time: Time) -> Self {
        BcdTime {
            hour: Self::bcd_encode(time.hour),
            minutes: Self::bcd_encode(time.minute),
            seconds: Self::bcd_encode(time.second),
        }
    }
}

/// Keeps BCD date
pub(crate) struct BcdDate {
    pub(crate) d: Bcd<u8>,
    pub(crate) m: Bcd<u8>,
    pub(crate) y: Bcd<u8>,
}

/// API for easy date access to BCD date converted into NaiveDate
impl BcdDate {
    /// Returns date converted from BCD to NaiveTime
    pub(crate) fn date(self) -> Date {
        let century: u32 = 2000;
        Date {
            day: self.d.get(),
            month: self.m.get(),
            year: u32::from(self.y.get()) + century,
        }
    }
}

/// API for easy create BCD date from NaiveDate
impl From<Date> for BcdDate {
    /// Create BCD encoded date from Date struct
    fn from(date: Date) -> Self {
        // Because of RTC_DR limitation we cannot use dates earlier than 2000
        let year = match date.year < 2000 || date.year > 2154 {
            true => 2000,
            false => date.year,
        };
        Self {
            d: Self::bcd_encode(date.day),
            m: Self::bcd_encode(date.month),
            y: Self::bcd_encode((year - 2000) as u8),
        }
    }
}

impl BcdConvert for BcdDate {}

/// Trait for converting BCD time
trait BcdConvert {
    /// Converts BCD values into integer value for easier usage
    fn bcd_decode(time: &Bcd<u8>) -> u8 {
        if time.tens == 0 && time.units == 0 {
            return 0;
        }

        (time.tens * 10) + time.units
    }

    /// Converts integer value into BCD format
    fn bcd_encode(time: u8) -> Bcd<u8> {
        if time == 0 {
            return Bcd { tens: 0, units: 0 };
        }
        let tens = u8::from(time) / 10;
        let units = u8::from(time) % 10;

        Bcd { tens, units }
    }
}
