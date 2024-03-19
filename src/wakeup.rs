use rtc::{Protection, Rtc};
use rtc_interrupt::RtcInterrupt;
use stm32f3xx_hal::interrupt;
use stm32f3xx_hal::pac::{Interrupt, EXTI, NVIC, RTC};

static mut INSTANCE: Option<fn()> = None;

/// Contains all WakeUp counter divisions that are available to use
pub enum WakeupRtcDivision {
    /// When used on WakeUp timer slows counting **16 times**
    RtcDiv16 = 0b000,
    /// When used on WakeUp timer slows counting **8 times**
    RtcDiv8 = 0b001,
    /// When used on WakeUp timer slows counting **4 times**
    RtcDiv4 = 0b010,
    /// When used on WakeUp timer slows counting **2 times**
    RtcDiv2 = 0b011,
    /// Do not affect on counter (counting seconds)
    RtcNoDiv = 0b100,
    /// Counter is increase by 0x10000 (65536)
    RtcOffset = 0b110,
}

impl WakeupRtcDivision {
    /// Returns bits that need to be written in to register to achieve particular division
    pub fn get_bits(self) -> u8 {
        self as u8
    }
}

/// By using this struct you can easily set up your WakeUp timer and interrupt.
/// WakeUp feature might be useful when you want your device to work in some time intervals.
/// This feature gives you ability to wake up device from low power consumption modes
/// like **SLEEP, STANDBY, STOP**
///
/// ## Usage:
/// 1. If you want to just basic WakeUp functionality with counter that count seconds:<br/>
/// **max counter time is 65535** <br/>
/// **Note:** This setup is able to wake up device from **StandBy** mode (Lowes power consumption mode)
/// ```
/// use stm32f3_rtc::rtc::Rtc;
/// use stm32f3xx_hal::pac;
///
/// let mut peripheral = pac::Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// rtc.get_wakeup_manager().set_counter(200).enable();
/// ```
/// 2. Enable WakeUp Interrupt that will wake up your device from **Stop** and **Sleep** modes with
/// interrupt handler.<br/>
/// **Note:** Note that that is not recommended to use loops, interrupt handler should work as quick
/// as possible, in other case it may slow down you program.
/// ```
/// use stm32f3_rtc::rtc::{Rtc};
/// use stm32f3_rtc::wakeup::WakeupManager;
/// use stm32f3xx_hal::pac;
/// use cortex_m_semihosting::hprintln;
///
/// let mut peripheral = pac::Peripherals::take().unwrap();
/// let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC)
///     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
/// rtc.get_wakeup_manager()
///     .set_counter(200)
///     .set_interrupt(true,peripheral.EXTI)
///     .enable();
/// WakeupManager::set_interrupt_handler(|| {hprintln!("Interupt handler works")})
/// ```
pub struct WakeupManager<'a> {
    rtc: &'a mut Rtc,
    sel: u8,
    time: u16,
    interrupt: RtcInterrupt,
    en_interrupt: bool,
}

impl<'a> WakeupManager<'a> {
    /// Returns new WakeupManager instance
    pub fn new(rtc: &'a mut Rtc) -> WakeupManager<'a> {
        Self {
            rtc,
            sel: WakeupRtcDivision::RtcNoDiv.get_bits(),
            time: 360,
            interrupt: RtcInterrupt::new(),
            en_interrupt: false,
        }
    }
    /// Configure your Interrupt by setting output and polarity.
    ///
    /// **Output selection (OSEL):** By setting this option you determinate with functionality
    /// of RTC will activate RTC_ALARM output event. STM32F3 device contain pin with RTC_ALARM
    /// Alternate Function.<br/>
    /// **Polarity (POL):** By setting this option you select witch state **(High/Low)**
    /// will be triggered on pin.
    ///
    /// **Note:** By default this OSEL is Disabled and POL is High
    ///
    /// ## Example:
    /// ```
    /// use stm32f3_rtc::rtc_interrupt::{RtcInterrupt, RtcInterruptOutputPolarity, RtcInterruptOutputSelection};
    /// ...
    /// let mut wkup = rtc.get_wakeup_manager();
    ///
    /// wkup.configure_interrupt(
    ///     RtcInterrupt::new()
    ///     .set_output_selection(RtcInterruptOutputSelection::WakeUp)
    ///     .set_polarity(RtcInterruptOutputPolarity::High)
    /// );
    /// ```
    pub fn configure_interrupt(&mut self, interrupt: RtcInterrupt) -> &Self {
        self.interrupt = interrupt;
        self
    }

    /// Enable the interrupt for RTC WKUP event
    ///
    /// ## Takes:
    /// enable: bool -> On(true) Off(false)
    /// exti: EXTI -> takes peripheral from stm32f3xx_hal
    ///
    /// ## Example
    /// ```
    /// let mut peripheral = pac::Peripherals::take().unwrap();
    /// wkup.set_interrupt(true, peripheral.EXTI);
    /// ```
    pub fn set_interrupt(mut self, enable: bool, exti: EXTI) -> Self {
        self.en_interrupt = enable;
        exti.imr1.modify(|_, w| w.mr20().unmasked());
        exti.rtsr1.modify(|_, w| w.tr20().enabled());
        unsafe { NVIC::unmask(Interrupt::RTC_WKUP) };
        self
    }

    /// You can set up you interrupt handler
    ///
    /// ## Example:
    /// ### 1:
    /// ```
    /// WakeupManager::set_interrupt_handler(|| {hprintln!("Interupt handler works")})
    /// ```
    /// ### 2:
    /// ```
    /// fn handler(number: u8) {
    /// hprintln!("My number: {}", number);
    /// }
    /// ...
    /// WakeupManager::set_interrupt_handler(|| handler(3))
    /// ```
    pub fn set_interrupt_handler(function: fn()) {
        unsafe { INSTANCE = Some(function) }
    }

    /// Please set counter for your WakeUp event. Every time counter will finish
    /// counting time that you set WakeUp event will happen.
    pub fn set_counter(mut self, time: u16) -> Self {
        self.time = time;
        self
    }


    /// You can set division for your RTC clock that will affect by slowing down
    /// WukeUp timer. Please read **WakeupRtcDivision** documentation.
    ///
    pub fn set_clock_division(mut self, division: WakeupRtcDivision) -> Self {
        self.sel = division.get_bits();
        self
    }

    /// Enable wakeup timer. I can be reused to reconfigure the timer.
    pub fn enable(mut self) -> Self {
        // Disable Wakeup Timer and waiting for ready flag
        self.rtc.rtc.cr.modify(|_, w| w.wute().disabled());
        while self.rtc.rtc.isr.read().wutwf().is_update_not_allowed() {}
        self.set_wutsel();
        self.rtc.write_protection(Protection::Disable);
        // Interrupt enabling
        match self.en_interrupt {
            true => self.enable_interrupts(),
            false => self.rtc.rtc.cr.modify(|_, w| w.wutie().disabled()),
        }
        self.set_time();
        self.rtc.rtc.cr.modify(|_, w| w.wute().enabled());
        self.rtc.rtc.isr.modify(|_, w| w.wutf().bit(false));
        self.rtc.write_protection(Protection::Enable);
        while self.rtc.rtc.isr.read().wutwf().is_update_allowed() {}
        self
    }

    fn set_wutsel(&mut self) {
        self.rtc
            .modify(|rtc| rtc.cr.modify(|_, w| w.wucksel().clock_spare()));
    }

    fn set_time(&mut self) {
        self.rtc
            .rtc
            .wutr
            .modify(|_, w| w.wut().bits(self.time as u16));
    }

    fn enable_interrupts(&mut self) {
        self.rtc.rtc.cr.modify(|_, w| w.wutie().enabled());
        self.rtc.rtc.cr.modify(|_, w| {
            w.osel()
                .bits(self.interrupt.output_selection.clone().into());
            w.pol().bit(self.interrupt.polarity.clone().into())
        })
    }
}

#[interrupt]
unsafe fn RTC_WKUP() {
    match INSTANCE {
        None => {}
        Some(_function) => _function(),
    }
    (*RTC::PTR).isr.modify(|_, w| w.wutf().clear_bit());
    (*EXTI::PTR).pr1.modify(|_, w| w.pr20().set_bit());
}
