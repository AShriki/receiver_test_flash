#![no_main]
#![no_std]

use core::panic::PanicInfo;

use core::cell::{
    RefCell,
};

use cortex_m::{
    self,
    interrupt::{free, Mutex},
    peripheral::NVIC,
};
use cortex_m_rt::entry;

use stm32_hal2::{
    clocks::Clocks,
    gpio::{Pin, PinMode, Port},
    low_power,
    pac::{interrupt,self},
    rtc::{Rtc, RtcClockSource, RtcConfig},
    make_globals,
    access_global,
};

// we're going to use S3 to store and retreive firmware

make_globals!((RTC, Rtc));

#[entry]
fn main() -> ! {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();

    let clock_cfg = Clocks::default();
    clock_cfg.setup().unwrap();

    let mut rtc = Rtc::new(
        dp.RTC,
        RtcConfig {
            clock_source: RtcClockSource::Lsi,
            async_prescaler: 127,
            sync_prescaler: 255,
            bypass_lse_output: false
        },
    );
    rtc.set_wakeup(2.);

    free(|cs| {
        RTC.borrow(cs).replace(Some(rtc));
    });
    let mut led1 = Pin::new(Port::B,2,PinMode::Output);
    let mut led2 = Pin::new(Port::A,10,PinMode::Output);
    led1.set_high();
    led2.set_low();
    unsafe {
        NVIC::unmask(pac::Interrupt::RTC_WKUP);
        cp.NVIC.set_priority(pac::Interrupt::RTC_WKUP, 1);
    }
    let mut cycle = 0;
    let mut wakeup_time: f32 = 2.;
    loop {
        low_power::stop(low_power::StopMode::Two);
        clock_cfg.reselect_input();
        free(|cs| {
            access_global!(RTC, rtc, cs);
            rtc.disable_wakeup();
            if cycle >= 7 {
                wakeup_time = 1./wakeup_time;
                rtc.set_wakeup(wakeup_time);
                cycle = 0;
            }
            rtc.enable_wakeup();
        });
        
        match led1.get_state(){
            stm32_hal2::gpio::PinState::High => led1.set_low(),
            stm32_hal2::gpio::PinState::Low => led1.set_high(),
        }
        match led2.get_state(){
            stm32_hal2::gpio::PinState::High => led2.set_low(),
            stm32_hal2::gpio::PinState::Low => led2.set_high(),
        }
        cycle += 1
    }
}

#[interrupt]
/// RTC wakeup handler
fn RTC_WKUP() {
    free(|cs| {
        // Reset pending bit for interrupt line
        unsafe {
            (*pac::EXTI::ptr()).pr1.modify(|_, w| w.pr20().set_bit());
        }
        access_global!(RTC, rtc, cs);
        rtc.clear_wakeup_flag();

    });
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}