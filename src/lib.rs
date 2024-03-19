#![no_std]
extern crate stm32f3xx_hal;
extern crate cortex_m_semihosting;
extern crate cortex_m_rt;
extern crate cortex_m;

pub mod datetime;
pub mod rtc;
pub mod wakeup;
pub mod rtc_interrupt;
