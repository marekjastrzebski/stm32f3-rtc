use crate::datetime::{Bcd, BcdDate, BcdTime, DateAccess, TimeAccess};
use chrono::{NaiveDate, NaiveTime};
use stm32f3xx_hal::pac::rcc::BDCR;
use stm32f3xx_hal::pac::{PWR, RCC, RTC};

enum Init {
    Start,
    Stop,
}

enum Protection {
    Enable,
    Disable,
}

pub enum ClockSource {
    LSI,
    LSE(bool),
    HSE(bool),
}

struct Prediv {
    a: u8,
    s: u16,
}

/// Create instance of RTC register API for easy manipulate values in this
/// register. Gives you easy access to date, time, alarms, milliseconds or wakeup.
/// It enable RTC without any additional actions needed.
///
/// It contains default (most typical clocks frequencies) for LSI, LSE and HSE.
/// You can run without any perscaller setup:
/// - LSI - 40 kHz - this clock source might be not accurate
/// - LSE - 32.768 kHz
/// - HSE - 24MHz
///
///
/// # Basic usage
/// 1. Creating RTC instance
/// ```
/// use chrono::NaiveTime;
/// use stm32f3xx_hal::pac::{Peripherals};
/// use stm32f3xx_hal::prelude::*;
/// use stm32f3_pcm::rtc::{Rtc, ClockSource};
/// use cortex_m_semihosting::hprintln;
/// ...
/// let mut peripheral = Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC)
///     .set_clock_source(ClockSource::LSI)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// ```
///
/// 2. Setup and reed time:
/// ```
/// use chrono::NaiveTime;
/// use stm32f3xx_hal::pac::{Peripherals};
/// use stm32f3xx_hal::prelude::*;
/// use stm32f3_pcm::rtc::{Rtc, ClockSource};
/// use cortex_m_semihosting::hprintln;
/// ...
/// let mut peripheral = Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC)
///     .set_clock_source(ClockSource::LSI)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// rtc.set_time(NaiveTime::from_hms_opt(12, 30, 10).unwrap());
/// hprintln!("{:?}", rtc.time().format("%H:%M:%S").to_string());
/// // print: 12:30:10
/// ```
/// 3. Setup and read date:
/// ```
/// use chrono::NaiveDate;
/// use stm32f3xx_hal::pac::{Peripherals};
/// use stm32f3xx_hal::prelude::*;
/// use stm32f3_pcm::rtc::{Rtc, ClockSource};
/// use cortex_m_semihosting::hprintln;
/// ...
/// let mut peripheral = Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC)
///     .set_clock_source(ClockSource::LSI)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// rtc.set_date(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap());
/// hprintln!("{:?}", rtc.date().format(""%Y-%m-%d"").to_string());
/// // print: 2024-3-1
/// ```
pub struct Rtc {
    rtc: RTC,
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
            source,
            prediv: Prediv { a: 0, s: 0 },
            default: true,
        }
    }

    pub fn set_clock_source(&mut self, clock_source: ClockSource) -> &Self {
        self.source = clock_source;
        if !self.default {
            return self;
        }
        match self.source {
            ClockSource::LSI => {
                // Default prediv for 40kHz clock
                self.prediv = Prediv { a: 39, s: 999 };
            }
            ClockSource::LSE(bypass) => {
                // Default prediv for 32.768 kHz clock
                self.prediv = Prediv { a: 127, s: 255 };
            }
            ClockSource::HSE(bypass) => {
                // Default prediv for 24 MHz clock
                self.prediv = Prediv { a: 0, s: 0 };
            }
        }
        self
    }

    pub fn set_prescalers(&mut self, a: u8, s: u16) -> &Self {
        self.default = false;
        self.prediv = Prediv { a, s };
        self
    }

    pub fn start_clock(&self, pwr: &mut PWR, rcc: &mut RCC) -> &Self {
        self.enable_clock_source(&mut rcc.bdcr, rcc)
            .enable_bdr(rcc, pwr)
            .enable_rtc(&mut rcc.bdcr)
            .set_prediv();
        self
    }

    fn modify<F>(&mut self, mut function: F)
    where
        F: FnMut(&mut RTC),
    {
        self.rtc.wpr.write(|w| w.key().bits(self.unlock_key.0));
        self.rtc.wpr.write(|w| w.key().bits(self.unlock_key.1));
        self.initf(Init::Start);
        function(&mut self.rtc);
        self.initf(Init::Stop);
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
                if self.rtc.isr.read().init().bit_is_clear() {
                    self.rtc.isr.modify(|_, w| w.init().clear_bit());
                    while !self.rtc.isr.read().initf().bit_is_clear() {}
                }
            }
        }
    }

    /// Enable/Disable write protection for RTC module
    fn write_protection(&mut self, protection: Protection) {
        match protection {
            Protection::Disable => {
                self.rtc.wpr.write(|w| w.key().bits(self.unlock_key.0));
                self.rtc.wpr.write(|w| w.key().bits(self.unlock_key.1));
            }
            Protection::Enable => self.rtc.wpr.write(|w| w.key().bits(0xC0)),
        }
    }
}

impl RtcSetup<Rtc> for Rtc {
    /// Enable different clock sources for RTC, picked by user
    fn enable_clock_source(&self, bdcr: &mut BDCR, rcc: &mut RCC) -> &Self {
        match self.source {
            ClockSource::LSI => {
                rcc.csr.modify(|_, w| w.lsion().set_bit());
                while rcc.csr.read().lsirdy().bit_is_clear() {}
            }
            ClockSource::LSE(bypass) => {
                bdcr.modify(|_, w| {
                    w.lseon().set_bit();
                    w.lsebyp().bit(bypass)
                });
                while bdcr.read().lserdy().bit_is_clear() {}
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

    fn enable_bdr(&self, rcc: &mut RCC, pwr: &mut PWR) -> &Self {
        rcc.apb1enr.modify(|_, w| w.pwren().enabled());
        pwr.cr.modify(|_, w| w.dbp().set_bit());
        while pwr.cr.read().dbp().bit_is_clear() {}
        self
    }

    /// Enable RTC with clock source selected by user
    fn enable_rtc(&self, bdcr: &mut BDCR) -> &Self {
        bdcr.modify(|_, w| w.bdrst().enabled());
        match self.source {
            ClockSource::LSI => bdcr.modify(|_, w| w.rtcsel().lsi()),
            ClockSource::LSE(_) => bdcr.modify(|_, w| w.rtcsel().lse()),
            ClockSource::HSE(_) => bdcr.modify(|_, w| w.rtcsel().hse()),
        }
        bdcr.modify(|_, w| {
            w.rtcen().enabled();
            w.bdrst().disabled()
        });
        self
    }

    /// Set prescaler value for RTC
    fn set_prediv(&self) -> &Self {
        self.rtc.prer.modify(|_, w| {
            w.prediv_a().bits(self.prediv.a);
            w.prediv_s().bits(self.prediv.s)
        });
        self
    }
}

impl TimeAccess for Rtc {
    fn time(&self) -> NaiveTime {
        BcdTime {
            h: Bcd {
                tens: self.rtc.tr.read().ht().bits(),
                units: self.rtc.tr.read().hu().bits(),
            },
            m: Bcd {
                tens: self.rtc.tr.read().mnt().bits(),
                units: self.rtc.tr.read().mnu().bits(),
            },
            s: Bcd {
                tens: self.rtc.tr.read().st().bits(),
                units: self.rtc.tr.read().su().bits(),
            },
        }
        .time()
    }

    fn set_time(&self, time: NaiveTime) {
        let bcd_time = BcdTime::from(time);
        self.rtc.tr.modify(|_, w| {
            w.ht().bits(bcd_time.h.tens);
            w.hu().bits(bcd_time.h.units);
            w.mnt().bits(bcd_time.m.tens);
            w.mnu().bits(bcd_time.m.units);
            w.st().bits(bcd_time.s.tens);
            w.su().bits(bcd_time.s.units)
        })
    }

    fn ms(&self) -> u32 {
        BcdTime::calc_ms(
            self.rtc.ssr.read().bits(),
            self.rtc.prer.read().prediv_a().bits(),
            self.rtc.prer.read().prediv_s().bits(),
        )
    }
}

impl DateAccess for Rtc {
    fn date(&self) -> NaiveDate {
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

    fn set_date(&self, date: NaiveDate) {
        let bcd_date = BcdDate::from(date);
        self.rtc.dr.modify(|_, w| {
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
    }
}

trait RtcSetup<T> {
    fn enable_clock_source(self, bdcr: &mut BDCR, rcc: &mut RCC) -> T;
    fn enable_bdr(self, rcc: &mut RCC, pwr: &mut PWR) -> T;
    fn enable_rtc(self, bdcr: &mut BDCR) -> T;
    fn set_prediv(self) -> T;
}
