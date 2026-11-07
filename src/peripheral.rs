#![no_main]
#![no_std]

mod rgb_led;
mod rgb_led_state;

use rmk::macros::rmk_peripheral;

#[rmk_peripheral(id = 0)]
mod keyboard_peripheral {
    use crate::rgb_led::{new_nrf_rgb_led, RgbLedRole};
    use embassy_nrf::gpio::{Level, Output, OutputDrive};
    use rmk::controller::PollingController;

    #[controller(poll)]
    fn rgb_led_controller() {
        new_nrf_rgb_led(
            Output::new(p.P0_26, Level::High, OutputDrive::Standard),
            Output::new(p.P0_30, Level::High, OutputDrive::Standard),
            Output::new(p.P0_06, Level::High, OutputDrive::Standard),
            RgbLedRole::Peripheral,
        )
    }
}
