# stm32f3-rtc
![](./logo.png)
It is complex API for RTC peripheral management. It is made with concept **"minimal user interaction"**,
it means that user using this API doesn't waste his time on setups or configuration. Contain some default settings 
for particular elements, but if you want to dive into some individual settings it gave you that opportunity.
But if you want to just quickly setup clock that holds specified date or time, you just doing it with 2 lines of code.

## Features
List of features that are already available 

1. [x] [RTC Setup](#1-creating-rtc-instance) 
2. [x] [LSI - Low Speed Internal clock](#4-setup-different-clock-source)
3. [x] [LSE - Low Speed External clock](#4-setup-different-clock-source)
4. [x] [HSE - High Speed External clock (max **12 MHz**)](#4-setup-different-clock-source)
5. [x] [Time access/setup](#2-setup-and-read-time)
6. [x] [Date access/setup](#3-setup-and-read-date)
7. [x] Delay in seconds
8. [x] Automatic **Wake up** Setup
9. [ ] Alarms
10. [ ] Time-stamps
11. [ ] Tamper
12. [ ] Daylight saving (Summer/Winter time)

## Compatibility
This lib is designed to work with STM32 F3 family microcontrollers, especially with 
those described in [RM0316](https://www.google.com/url?sa=t&rct=j&q=&esrc=s&source=web&cd=&ved=2ahUKEwjnjfyY1OKEAxW7QvEDHU2ABBQQFnoECBMQAQ&url=https%3A%2F%2Fwww.st.com%2Fresource%2Fen%2Freference_manual%2Frm0316-stm32f303xbcde-stm32f303x68-stm32f328x8-stm32f358xc-stm32f398xe-advanced-armbased-mcus-stmicroelectronics.pdf&usg=AOvVaw0mltpVxT-GB1zXjNXCP50O&opi=89978449)

## Dependencies
Library is wrapper for stm32f3xx-hal, but it use it minimal way, only for peripheral access.

## Basic Usage
### Cargo.toml
```toml
[dependencies]
// Please pick feature that align with your device
stm32f3xx-hal = { version="0.10.1", features = ["stm32f303xc"]}
stm32f3-rtc = {version="0.1", features = ["stm32f303xc"]}
```
### Code usage: 
**By default, rtc using LSI** clock source that is built into microcontroller, but it is not accurate.
If you don't need super accurate clock it is most of the time enough.
 #### 1. Creating RTC instance
 ```rust
 use stm32f3_rtc::rtc::Rtc;
 use stm32f3xx_hal::pac;

 let mut peripheral = pac::Peripherals::take().unwrap();
 let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
 ```

 #### 2. Setup and read time:
 ```rust
 use stm32f3_rtc::datetime::{Time, TimeAccess};
 use stm32f3_rtc::rtc::Rtc;
 use stm32f3xx_hal::pac;
 use cortex_m_semihosting;

 let mut peripheral = pac::Peripherals::take().unwrap();
 let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
 rtc.set_time(Time::from(12,30,0));
 let time = rtc.time();
 hprintln!("{}:{}:{}", time.hour, time.minute, time.second);
 //Print: 12:30:0
 ```
#### 3. Setup and read date:
 ```rust
 use stm32f3_rtc::datetime::{Date, DateAccess};
 use stm32f3_rtc::rtc::Rtc;
 use stm32f3xx_hal::pac;
use cortex_m_semihosting::hprintln;

 let mut peripheral = pac::Peripherals::take().unwrap();
 let rtc = Rtc::new(peripheral.RTC).start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
 rtc.set_date(Date::from(1,1,2024));
 let date = rtc.date();
 hprintln!("{}.{}.{}", date.day, date.month, date.year);
 //Print: 1.1.2024
 ```
#### 4. Setup different clock source:
 This example shows how to run LSE clock with defoult prescalers for 32,768kHz frequency.
 You can pick your own prescalers by using set_prescalers() function. If you up to please read
 its documentation.
 ```rust
 use stm32f3_rtc::rtc::{ClockSource, Rtc};
 use stm32f3xx_hal::pac;
use cortex_m_semihosting::hprintln;

 let mut peripheral = pac::Peripherals::take().unwrap();
 let rtc = Rtc::new(peripheral.RTC)
     .set_clock_source(ClockSource::LSE(true))
     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
 ```

#### 5. Using delay:
 ```rust
 use stm32f3_rtc::rtc::{ClockSource, Rtc};
 use stm32f3xx_hal::pac;
use cortex_m_semihosting::hprintln;

 let mut peripheral = pac::Peripherals::take().unwrap();
 let rtc = Rtc::new(peripheral.RTC)
     .start_clock(&mut peripheral.PWR, &mut peripheral.RCC);
rtc.set_time(Time::from(12,30,0));

loop {
    rtc.delay(2);
    hprintln!("This text will appear every 2 seconds");
}
 ```

