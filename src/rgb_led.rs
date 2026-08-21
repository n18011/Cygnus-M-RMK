use embassy_time::Duration;
use rmk::channel::{ControllerSub, CONTROLLER_CHANNEL};
use rmk::controller::{Controller, PollingController};
use rmk::event::ControllerEvent;

pub use crate::rgb_led_state::RgbLedRole;
use crate::rgb_led_state::{RgbBleState, RgbConnectionType, RgbLedEvent, RgbLedState, TICK_MS};

/// A small GPIO abstraction keeps the state machine testable without nRF hardware.
pub trait LedPin {
    fn set_high(&mut self);
    fn set_low(&mut self);
}

impl<'d> LedPin for embassy_nrf::gpio::Output<'d> {
    fn set_high(&mut self) {
        embassy_nrf::gpio::Output::set_high(self);
    }

    fn set_low(&mut self) {
        embassy_nrf::gpio::Output::set_low(self);
    }
}

pub struct RgbLedController<R, G, B> {
    red: R,
    green: G,
    blue: B,
    state: RgbLedState,
    sub: ControllerSub,
}

impl<R, G, B> RgbLedController<R, G, B>
where
    R: LedPin,
    G: LedPin,
    B: LedPin,
{
    pub fn new(red: R, green: G, blue: B, role: RgbLedRole) -> Self {
        Self {
            red,
            green,
            blue,
            state: RgbLedState::new(role),
            sub: defmt::unwrap!(CONTROLLER_CHANNEL.subscriber()),
        }
    }

    fn apply_color(&mut self) {
        let mask = self.state.current_color().mask();

        if mask & 0b001 == 0 {
            self.red.set_high();
        } else {
            self.red.set_low();
        }

        if mask & 0b010 == 0 {
            self.green.set_high();
        } else {
            self.green.set_low();
        }

        if mask & 0b100 == 0 {
            self.blue.set_high();
        } else {
            self.blue.set_low();
        }
    }
}

impl<R, G, B> Controller for RgbLedController<R, G, B>
where
    R: LedPin,
    G: LedPin,
    B: LedPin,
{
    type Event = ControllerEvent;

    async fn process_event(&mut self, event: Self::Event) {
        if let Some(event) = map_controller_event(event) {
            self.state.handle_event(event);
            self.apply_color();
        }
    }

    async fn next_message(&mut self) -> Self::Event {
        self.sub.next_message_pure().await
    }
}

impl<R, G, B> PollingController for RgbLedController<R, G, B>
where
    R: LedPin,
    G: LedPin,
    B: LedPin,
{
    const INTERVAL: Duration = Duration::from_millis(TICK_MS as u64);

    async fn update(&mut self) {
        self.state.tick();
        self.apply_color();
    }
}

fn map_controller_event(event: ControllerEvent) -> Option<RgbLedEvent> {
    match event {
        ControllerEvent::Battery(level) => Some(RgbLedEvent::Battery(level)),
        ControllerEvent::ChargingState(charging) => Some(RgbLedEvent::ChargingState(charging)),
        ControllerEvent::Layer(layer) => Some(RgbLedEvent::Layer(layer)),
        ControllerEvent::ConnectionType(connection_type) => {
            Some(RgbLedEvent::ConnectionType(match connection_type {
                0 => RgbConnectionType::Usb,
                1 => RgbConnectionType::Ble,
                _ => RgbConnectionType::Other,
            }))
        }
        ControllerEvent::SplitCentral(connected) => Some(RgbLedEvent::SplitCentral(connected)),
        ControllerEvent::Sleep(sleeping) => Some(RgbLedEvent::Sleep(sleeping)),
        ControllerEvent::BleState(_, state) => Some(RgbLedEvent::BleState(match state {
            rmk::ble::BleState::Advertising => RgbBleState::Advertising,
            rmk::ble::BleState::Connected => RgbBleState::Connected,
            rmk::ble::BleState::None => RgbBleState::None,
        })),
        _ => None,
    }
}

pub type NrfRgbLedController = RgbLedController<
    embassy_nrf::gpio::Output<'static>,
    embassy_nrf::gpio::Output<'static>,
    embassy_nrf::gpio::Output<'static>,
>;

pub fn new_nrf_rgb_led(
    red: embassy_nrf::gpio::Output<'static>,
    green: embassy_nrf::gpio::Output<'static>,
    blue: embassy_nrf::gpio::Output<'static>,
    role: RgbLedRole,
) -> NrfRgbLedController {
    RgbLedController::new(red, green, blue, role)
}
