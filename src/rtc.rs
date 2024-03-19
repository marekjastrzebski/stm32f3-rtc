use crate::datetime::{Bcd, BcdDate, BcdTime, DateAccess, TimeAccess};
use datetime::{Date, Time};
use stm32f3xx_hal::pac::{PWR, RCC, RTC};
use wakeup::WakeupManager;

enum Init {
    Start,
    Stop,
}

pub(crate) enum Protection {
    Enable,
    Disable,
}

/// Offers clock source options LSI, LSE and HSE. Two of this source can have bypass on,
/// by filling bool parameter
pub enum ClockSource {
    /// Low Speed Internal clock source - it is built in microcontroller,
    /// so you sure that you have this one. But you have to know that this one is not that accurate
    /// like others.
    LSI,
    /// Low Speed External clock source - it is external clock source, very accurate.
    /// Please note that you can set bypass by setting bool argument. (Preferably should by true)
    /// ```
    /// use stm32f3_rtc::rtc::ClockSource;
    /// ClockSource::LSE(true);
    /// ```
    LSE(bool),
    /// High Speed External clock source - it is external clock source, very accurate.
    /// Please note that you can set bypass by setting bool argument. (Preferably should be true)
    /// ```
    /// use stm32f3_rtc::rtc::ClockSource;
    /// ClockSource::HSE(true);
    /// ```
    HSE(bool),
}

struct Prediv {
    a: u8,
    s: u16,
}

/// Create instance of RTC register API for easy manipulate values in this
/// register. Gives you easy access to date, time, alarms, milliseconds or wakeup.
/// It enables RTC without any additional actions needed.
///
/// It contains default (most typical clocks frequencies) for LSI, LSE and HSE.
/// You can run without any perscaller setup:
/// - LSI - 40 kHz - this clock source might be not accurate. There might be sight
/// differences in second counting frequency. But for sure you have this clock source
/// because it is built in.
/// - LSE - 32.768 kHz
/// - HSE - 12MHz
///
///
/// # Basic usage
/// 1. Creating RTC instance
/// ```
/// use stm32f3_rtc::rtc::Rtc;
/// use stm32f3xx_hal::pac;
///
/// let mut peripheral = pac::Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// ```
///
/// 2. Setup and read time:
/// By default it runs on LSI clock source
/// ```
/// use stm32f3_rtc::datetime::{Time, TimeAccess};
/// use stm32f3_rtc::rtc::Rtc;
/// use stm32f3xx_hal::pac;
/// use cortex_m_semihosting::hprintln;
///
/// let mut peripheral = pac::Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// rtc.set_time(Time::from(12,30,0));
/// let time = rtc.time();
/// hprintln!("{}:{}:{}", time.hour, time.minute, time.second);
/// //Print: 12:30:0
/// ```
/// 3. Setup and read date:
/// ```
/// use stm32f3_rtc::datetime::{Date, DateAccess};
/// use stm32f3_rtc::rtc::Rtc;
/// use stm32f3xx_hal::pac;
/// use cortex_m_semihosting::hprintln;
///
/// let mut peripheral = pac::Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// rtc.set_date(Date::from(1,1,2024));
/// let date = rtc.date();
/// hprintln!("{}.{}.{}", date.day, date.month, date.year);
/// //Print: 1.1.2024
/// ```
/// 4. Setup diferent clock source:
/// This example shows how to run LSE clock with defoult prescalers for 32,768kHz frequency.
/// You can pick your own prescalers by using set_prescalers() function. If you up to please read
/// its documentation.
/// ```
/// use stm32f3_rtc::rtc::{ClockSource, Rtc};
/// use stm32f3xx_hal::pac;
/// use cortex_m_semihosting::hprintln;
///
/// let mut peripheral = pac::Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC)
///     .set_clock_source(ClockSource::LSE(true))
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// ```
pub struct Rtc {
    pub(crate) rtc: RTC,
    unlock_key: (u8, u8),
    source: ClockSource,
    prediv: Prediv,
    default: bool,
}

impl Rtc {
    pub fn new(rtc: RTC) -> Self {
        Self {
            rtc,
            unlock_key: (0xCA, 0x53),
            source: ClockSource::LSI,
            prediv: Prediv { a: 127, s: 319 },
            default: true,
        }
    }

    /// Please select your own clock source, by picking it from ClockSource enum
    pub fn set_clock_source(&mut self, clock_source: ClockSource) -> &Self {
        self.source = clock_source;
        if !self.default {
            return self;
        }
        match self.source {
            ClockSource::LSI => {
                // Default prediv for 40kHz clock
                self.prediv = Prediv { a: 127, s: 319 };
            }
            ClockSource::LSE(_) => {
                // Default prediv for 32.768 kHz clock
                self.prediv = Prediv { a: 127, s: 255 };
            }
            ClockSource::HSE(_) => {
                // Default prediv for 12 MHz clock
                self.prediv = Prediv { a: 200, s: 60_000 };
            }
        }
        self
    }

    /// If you want to set up your own precalers you have to define it here
    /// you have to remember that you have to do it following this equation:
    /// **Frequency = (PREDIV_A + 1) * (PREDIV_S + 1)**
    pub fn set_prescalers(&mut self, a: u8, s: u16) -> &Self {
        self.default = false;
        self.prediv = Prediv { a, s };
        self
    }

    /// Starts RTC clock
    pub fn start_clock(&mut self, pwr: &mut PWR, rcc: &mut RCC) -> &mut Self {
        self.enable_clock_source(rcc)
            .enable_bdr(rcc, pwr)
            .enable_rtc(rcc);

        self.rtc.cr.modify(|_, w| w.fmt().set_bit());
        self.set_prediv();
        self
    }

    /// Stop executing program for a given seconds
    ///
    /// **Note:** Works only when RTC is started.
    ///
    /// **Note:** If seconds > 86390 = no delay effect
    pub fn delay(&self, seconds: u32) {
        if seconds > 86390 {
            return;
        }
        let time = self.time();
        let stop_sleep = time.to_seconds() + seconds;
        if stop_sleep > 86400 {
            while time.to_seconds() < 86400 {}
            while time.to_seconds() < (stop_sleep - 86400) {}
            return;
        }
        while self.time().to_seconds() < stop_sleep {}
    }

    pub(crate) fn modify<F>(&mut self, mut function: F)
    where
        F: FnMut(&mut RTC),
    {
        self.write_protection(Protection::Disable);
        self.initf(Init::Start);
        function(&mut self.rtc);
        self.initf(Init::Stop);
        self.write_protection(Protection::Enable)
    }

    pub fn get_wakeup_manager(&mut self) -> WakeupManager {
        WakeupManager::new(self)
    }
    fn initf(&mut self, init: Init) {
        match init {
            Init::Start => {
                if self.rtc.isr.read().init().bit_is_clear() {
                    self.rtc.isr.modify(|_, w| w.init().set_bit());
                    while self.rtc.isr.read().initf().bit_is_clear() {}
                }
            }
            Init::Stop => {
                if !self.rtc.isr.read().init().bit_is_clear() {
                    self.rtc.isr.modify(|_, w| w.init().clear_bit());
                    while !self.rtc.isr.read().initf().bit_is_clear() {}
                }
            }
        }
    }

    /// Enable/Disable write protection for RTC module
    pub(crate) fn write_protection(&self, protection: Protection) {
        match protection {
            Protection::Disable => {
                self.rtc.wpr.write(|w| w.key().bits(self.unlock_key.0));
                self.rtc.wpr.write(|w| w.key().bits(self.unlock_key.1))
            }
            Protection::Enable => self.rtc.wpr.write(|w| w.key().bits(0xC0)),
        }
    }
}

impl RtcSetup<Rtc> for Rtc {
    /// Enable different clock sources for RTC, picked by user
    fn enable_clock_source(&self, rcc: &mut RCC) -> &Self {
        match self.source {
            ClockSource::LSI => {
                rcc.csr.modify(|_, w| w.lsion().set_bit());
                while rcc.csr.read().lsirdy().bit_is_clear() {}
            }
            ClockSource::LSE(bypass) => {
                rcc.bdcr.modify(|_, w| {
                    w.lseon().set_bit();
                    w.lsebyp().bit(bypass)
                });
                while rcc.bdcr.read().lserdy().bit_is_clear() {}
            }
            ClockSource::HSE(bypass) => {
                rcc.cr.modify(|_, w| {
                    w.hseon().set_bit();
                    w.hsebyp().bit(bypass)
                });
                while rcc.cr.read().hserdy().bit_is_clear() {}
            }
        }
        self
    }

    /// Enable bdr
    fn enable_bdr(&self, rcc: &mut RCC, pwr: &mut PWR) -> &Self {
        rcc.apb1enr.modify(|_, w| w.pwren().enabled());
        pwr.cr.modify(|_, w| w.dbp().set_bit());
        while pwr.cr.read().dbp().bit_is_clear() {}
        self
    }

    /// Enable RTC with clock source selected by user
    fn enable_rtc(&self, rcc: &mut RCC) -> &Self {
        rcc.bdcr.modify(|_, w| w.bdrst().enabled());
        rcc.bdcr.modify(|_, w| {
            match self.source {
                ClockSource::LSI => w.rtcsel().lsi(),
                ClockSource::LSE(_) => w.rtcsel().lse(),
                ClockSource::HSE(_) => w.rtcsel().hse(),
            };
            w.rtcen().enabled();
            w.bdrst().disabled()
        });
        self
    }

    /// Set prescaler value for RTC
    fn set_prediv(&mut self) -> &Self {
        let a = self.prediv.a;
        let s = self.prediv.s;
        self.modify(|rtc| {
            rtc.prer.modify(|_, w| {
                w.prediv_a().bits(a);
                w.prediv_s().bits(s)
            })
        });
        self
    }
}

impl TimeAccess for Rtc {
    /// Returns current time as Time struct
    fn time(&self) -> Time {
        BcdTime {
            hour: Bcd {
                tens: self.rtc.tr.read().ht().bits(),
                units: self.rtc.tr.read().hu().bits(),
            },
            minutes: Bcd {
                tens: self.rtc.tr.read().mnt().bits(),
                units: self.rtc.tr.read().mnu().bits(),
            },
            seconds: Bcd {
                tens: self.rtc.tr.read().st().bits(),
                units: self.rtc.tr.read().su().bits(),
            },
        }
        .time()
    }

    /// Set time by Time struct
    /// ```
    /// rtc.set_time(Time::from(12,30,0));
    /// ```
    fn set_time(&mut self, time: Time) {
        let bcd_time = BcdTime::from(time);
        self.modify(|rtc| {
            rtc.tr.modify(|_, w| {
                w.ht().bits(bcd_time.hour.tens);
                w.hu().bits(bcd_time.hour.units);
                w.mnt().bits(bcd_time.minutes.tens);
                w.mnu().bits(bcd_time.minutes.units);
                w.st().bits(bcd_time.seconds.tens);
                w.su().bits(bcd_time.seconds.units)
            })
        })
    }
}

impl DateAccess for Rtc {
    /// Returns current date as Date struct
    fn date(&self) -> Date {
        BcdDate {
            d: Bcd {
                tens: self.rtc.dr.read().dt().bits(),
                units: self.rtc.dr.read().du().bits(),
            },
            m: Bcd {
                tens: u8::from(self.rtc.dr.read().mt().bit()),
                units: self.rtc.dr.read().mu().bits(),
            },
            y: Bcd {
                tens: self.rtc.dr.read().yt().bits(),
                units: self.rtc.dr.read().yu().bits(),
            },
        }
        .date()
    }

    /// Set date with Date struct, It takes year between 2000 and 2154,
    /// if you will pick some other year it is going to reset it to 2000
    /// ```
    /// rtc.set_date(Date::from(1,1,2024));
    /// ```
    fn set_date(&mut self, date: Date) {
        let bcd_date = BcdDate::from(date);
        self.modify(|rtc| {
            rtc.dr.modify(|_, w| {
                match bcd_date.m.tens > 0 {
                    true => w.mt().bit(true),
                    false => w.mt().bit(false),
                };
                w.dt().bits(bcd_date.d.tens);
                w.du().bits(bcd_date.d.units);
                w.mu().bits(bcd_date.m.units);
                w.yt().bits(bcd_date.y.tens);
                w.yu().bits(bcd_date.y.units)
            })
        })
    }
}

trait RtcSetup<T> {
    fn enable_clock_source(&self, rcc: &mut RCC) -> &T;
    fn enable_bdr(&self, rcc: &mut RCC, pwr: &mut PWR) -> &T;
    fn enable_rtc(&self, rcc: &mut RCC) -> &T;
    fn set_prediv(&mut self) -> &T;
}
