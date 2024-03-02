use chrono::{Datelike, NaiveDate, NaiveTime, Timelike};
use num_traits::{FromPrimitive, Num, NumCast, ToPrimitive};

/// Trait that determinate time write and access
pub trait TimeAccess {
    fn time(self) -> NaiveTime;
    fn set_time(self, time: NaiveTime);
    fn ms(self) -> u32;
}

/// Trait that determinate date write and access
pub trait DateAccess {
    fn date(self) -> NaiveDate;
    fn set_date(self, date: NaiveDate);
}

/// Single BCD encoded value
pub(crate) struct Bcd<T> {
    pub(crate) tens: T,
    pub(crate) units: T,
}

/// Keeps BCD time
pub(crate) struct BcdTime {
    pub(crate) h: Bcd<u8>,
    pub(crate) m: Bcd<u8>,
    pub(crate) s: Bcd<u8>,
}

impl BcdTime {
    /// Returns the time in NaiveDate format
    pub(crate) fn time(self) -> NaiveTime {
        NaiveTime::from_hms_opt(
            Self::bcd_decode(self.h),
            Self::bcd_decode(self.m),
            Self::bcd_decode(self.s),
        )
        .unwrap()
    }

    /// Returns milliseconds
    pub(crate) fn calc_ms(subs: u32, prediv_a: u8, prediv_s: u16) -> u32 {
        let prescaler: u32 =
            <u32 as NumCast>::from(prediv_s * <u16 as NumCast>::from(prediv_a).unwrap()).unwrap();
        let ms = (f32::from_bits(1 - (subs / prescaler)) / 0.001) as u32;

        ms
    }
}

impl BcdConvert for BcdTime {}

/// API for easy create BCD time from NaiveTime
impl From<NaiveTime> for BcdTime {
    /// Create BCD time from NaiveTime
    fn from(time: NaiveTime) -> Self {
        BcdTime {
            h: Self::bcd_encode(time.hour()),
            m: Self::bcd_encode(time.minute()),
            s: Self::bcd_encode(time.second()),
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
    pub(crate) fn date(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(
            Self::bcd_decode(self.y),
            Self::bcd_decode(self.m),
            Self::bcd_decode(self.d),
        )
        .unwrap()
    }
}

/// API for easy create BCD date from NaiveDate
impl From<NaiveDate> for BcdDate {
    /// Create BCD encoded date from NaiveDate
    fn from(date: NaiveDate) -> Self {
        Self {
            d: Self::bcd_encode(date.day()),
            m: Self::bcd_encode(date.month()),
            y: Self::bcd_encode(date.year()),
        }
    }
}

impl BcdConvert for BcdDate {}

/// Trait for converting BCD time
trait BcdConvert {
    /// Converts BCD values into integer value for easier usage
    fn bcd_decode<T: NumCast + FromPrimitive>(time: Bcd<u8>) -> T {
        if time.tens == 0 && time.units == 0 {
            T::from_u8(0).unwrap();
        }

        T::from_u8((time.tens * 10) + time.units).unwrap()
    }

    /// Converts integer value into BCD format
    fn bcd_encode<ArgType: Num + Copy + FromPrimitive + ToPrimitive>(time: ArgType) -> Bcd<u8> {
        if time == ArgType::from_u8(0).unwrap() {
            return Bcd { tens: 0, units: 0 };
        }
        let tens = <u8 as NumCast>::from(time).unwrap() / 10;
        let units = <u8 as NumCast>::from(time).unwrap() % 10;

        Bcd { tens, units }
    }
}
