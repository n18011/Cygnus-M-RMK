pub const TICK_MS: u32 = 50;

const PATTERN_QUEUE_CAPACITY: usize = 16;
const STATUS_BLINK_MS: u32 = 100;
const CRITICAL_BATTERY_BLINK_MS: u32 = 250;
const CRITICAL_BATTERY_BLINKS: u8 = 3;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbColor {
    Off,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl RgbColor {
    /// Returns the RGB bit mask used by the active-low GPIO driver.
    pub const fn mask(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Red => 0b001,
            Self::Green => 0b010,
            Self::Yellow => 0b011,
            Self::Blue => 0b100,
            Self::Magenta => 0b101,
            Self::Cyan => 0b110,
            Self::White => 0b111,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbLedRole {
    Central,
    Peripheral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbBleState {
    Advertising,
    Connected,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbConnectionType {
    Usb,
    Ble,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RgbLedEvent {
    Battery(u8),
    ChargingState(bool),
    Layer(u8),
    ConnectionType(RgbConnectionType),
    SplitCentral(bool),
    Sleep(bool),
    BleState(RgbBleState),
}

#[derive(Clone, Copy)]
struct BlinkPattern {
    color: RgbColor,
    on_ticks: u8,
    off_ticks: u8,
    repeats: u8,
}

impl BlinkPattern {
    const fn new(color: RgbColor, on_ms: u32, off_ms: u32, repeats: u8) -> Self {
        Self {
            color,
            on_ticks: ticks_for(on_ms),
            off_ticks: ticks_for(off_ms),
            repeats,
        }
    }
}

#[derive(Clone, Copy)]
struct ActivePattern {
    color: RgbColor,
    on_ticks: u8,
    off_ticks: u8,
    repeats_left: u8,
    phase_on: bool,
    ticks_left: u8,
}

struct PatternQueue {
    items: [Option<BlinkPattern>; PATTERN_QUEUE_CAPACITY],
    len: usize,
}

impl PatternQueue {
    const fn new() -> Self {
        Self {
            items: [None; PATTERN_QUEUE_CAPACITY],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.items = [None; PATTERN_QUEUE_CAPACITY];
        self.len = 0;
    }

    fn push(&mut self, pattern: BlinkPattern) -> bool {
        if self.len == PATTERN_QUEUE_CAPACITY {
            return false;
        }
        self.items[self.len] = Some(pattern);
        self.len += 1;
        true
    }

    fn pop(&mut self) -> Option<BlinkPattern> {
        let pattern = self.items[0]?;
        for index in 1..self.len {
            self.items[index - 1] = self.items[index];
        }
        self.items[self.len - 1] = None;
        self.len -= 1;
        Some(pattern)
    }
}

pub struct RgbLedState {
    role: RgbLedRole,
    current_color: RgbColor,
    persistent_color: RgbColor,
    queue: PatternQueue,
    active: Option<ActivePattern>,
    sleeping: bool,
    last_battery: Option<u8>,
    connection_type: RgbConnectionType,
    ble_state: RgbBleState,
}

impl RgbLedState {
    pub const fn new(role: RgbLedRole) -> Self {
        Self {
            role,
            current_color: RgbColor::Off,
            persistent_color: RgbColor::Off,
            queue: PatternQueue::new(),
            active: None,
            sleeping: false,
            last_battery: None,
            connection_type: RgbConnectionType::Usb,
            ble_state: RgbBleState::None,
        }
    }

    pub fn current_color(&self) -> RgbColor {
        self.current_color
    }

    #[allow(dead_code)]
    pub fn has_pending_pattern(&self) -> bool {
        self.active.is_some() || self.queue.len != 0
    }

    pub fn handle_event(&mut self, event: RgbLedEvent) {
        match event {
            RgbLedEvent::Battery(level) => self.handle_battery(level),
            RgbLedEvent::ChargingState(charging) => {
                if charging {
                    self.set_persistent_color(RgbColor::Green);
                } else if let Some(level) = self.last_battery {
                    self.set_persistent_color(battery_color(level));
                }
            }
            RgbLedEvent::Layer(layer) if self.role == RgbLedRole::Central => {
                self.handle_layer(layer)
            }
            RgbLedEvent::Layer(_) => {}
            RgbLedEvent::ConnectionType(connection_type) => {
                self.connection_type = connection_type;
                if self.role == RgbLedRole::Central {
                    self.set_persistent_color(self.central_connection_color());
                }
            }
            RgbLedEvent::SplitCentral(connected) if self.role == RgbLedRole::Peripheral => {
                self.set_persistent_color(if connected {
                    RgbColor::Blue
                } else {
                    RgbColor::Red
                });
            }
            RgbLedEvent::SplitCentral(_) => {}
            RgbLedEvent::Sleep(sleeping) => self.handle_sleep(sleeping),
            RgbLedEvent::BleState(ble_state) if self.role == RgbLedRole::Central => {
                self.ble_state = ble_state;
                self.set_persistent_color(self.central_connection_color());
            }
            RgbLedEvent::BleState(_) => {}
        }
    }

    pub fn tick(&mut self) {
        if self.sleeping {
            self.current_color = RgbColor::Off;
            return;
        }

        let Some(mut active) = self.active else {
            self.start_next_pattern();
            return;
        };

        if active.ticks_left > 1 {
            active.ticks_left -= 1;
            self.active = Some(active);
            return;
        }

        if active.phase_on {
            active.phase_on = false;
            active.ticks_left = active.off_ticks;
            self.current_color = RgbColor::Off;
            self.active = Some(active);
        } else if active.repeats_left > 1 {
            active.repeats_left -= 1;
            active.phase_on = true;
            active.ticks_left = active.on_ticks;
            self.current_color = active.color;
            self.active = Some(active);
        } else {
            self.active = None;
            self.current_color = self.persistent_color;
            self.start_next_pattern();
        }
    }

    fn handle_battery(&mut self, level: u8) {
        let previous = self.last_battery;
        self.last_battery = Some(level);
        self.set_persistent_color(battery_color(level));

        if let Some(previous) = previous {
            if level > 0 && level <= 5 && previous > 5 {
                self.enqueue_pattern(BlinkPattern::new(
                    RgbColor::Red,
                    CRITICAL_BATTERY_BLINK_MS,
                    CRITICAL_BATTERY_BLINK_MS,
                    CRITICAL_BATTERY_BLINKS,
                ));
            }
        }
    }

    fn handle_layer(&mut self, layer: u8) {
        for _ in 0..layer {
            self.enqueue_pattern(BlinkPattern::new(
                RgbColor::Cyan,
                STATUS_BLINK_MS,
                STATUS_BLINK_MS,
                1,
            ));
        }
    }

    fn handle_sleep(&mut self, sleeping: bool) {
        self.sleeping = sleeping;
        self.queue.clear();
        self.active = None;
        self.current_color = if sleeping {
            RgbColor::Off
        } else {
            self.persistent_color
        };
    }

    fn central_connection_color(&self) -> RgbColor {
        match self.ble_state {
            RgbBleState::Advertising => RgbColor::Yellow,
            RgbBleState::Connected => RgbColor::Blue,
            RgbBleState::None => match self.connection_type {
                RgbConnectionType::Usb => RgbColor::Cyan,
                RgbConnectionType::Ble | RgbConnectionType::Other => RgbColor::Red,
            },
        }
    }

    fn set_persistent_color(&mut self, color: RgbColor) {
        self.persistent_color = color;
        if self.active.is_none() && !self.sleeping {
            self.current_color = color;
        }
    }

    fn enqueue_pattern(&mut self, pattern: BlinkPattern) {
        if self.queue.push(pattern) && self.active.is_none() && !self.sleeping {
            self.start_next_pattern();
        }
    }

    fn start_next_pattern(&mut self) {
        if self.sleeping {
            return;
        }

        if let Some(pattern) = self.queue.pop() {
            self.active = Some(ActivePattern {
                color: pattern.color,
                on_ticks: pattern.on_ticks,
                off_ticks: pattern.off_ticks,
                repeats_left: pattern.repeats,
                phase_on: true,
                ticks_left: pattern.on_ticks,
            });
            self.current_color = pattern.color;
        } else {
            self.current_color = self.persistent_color;
        }
    }
}

const fn ticks_for(milliseconds: u32) -> u8 {
    ((milliseconds + TICK_MS - 1) / TICK_MS) as u8
}

pub const fn battery_color(level: u8) -> RgbColor {
    match level {
        0 => RgbColor::Magenta,
        1..=19 => RgbColor::Red,
        20..=79 => RgbColor::Yellow,
        _ => RgbColor::Green,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        battery_color, RgbBleState, RgbColor, RgbConnectionType, RgbLedEvent, RgbLedRole,
        RgbLedState,
    };

    #[test]
    fn battery_levels_map_to_widget_colors() {
        assert_eq!(battery_color(0), RgbColor::Magenta);
        assert_eq!(battery_color(19), RgbColor::Red);
        assert_eq!(battery_color(20), RgbColor::Yellow);
        assert_eq!(battery_color(79), RgbColor::Yellow);
        assert_eq!(battery_color(80), RgbColor::Green);
        assert_eq!(battery_color(100), RgbColor::Green);
    }

    #[test]
    fn colors_use_red_green_blue_bits() {
        assert_eq!(RgbColor::Off.mask(), 0b000);
        assert_eq!(RgbColor::Red.mask(), 0b001);
        assert_eq!(RgbColor::Green.mask(), 0b010);
        assert_eq!(RgbColor::Blue.mask(), 0b100);
        assert_eq!(RgbColor::White.mask(), 0b111);
    }

    #[test]
    fn central_ble_state_uses_widget_connection_colors() {
        let mut state = RgbLedState::new(RgbLedRole::Central);
        state.handle_event(RgbLedEvent::BleState(RgbBleState::Advertising));
        assert_eq!(state.current_color(), RgbColor::Yellow);
        state.handle_event(RgbLedEvent::BleState(RgbBleState::Connected));
        assert_eq!(state.current_color(), RgbColor::Blue);
        state.handle_event(RgbLedEvent::ConnectionType(RgbConnectionType::Usb));
        state.handle_event(RgbLedEvent::BleState(RgbBleState::None));
        assert_eq!(state.current_color(), RgbColor::Cyan);
    }

    #[test]
    fn peripheral_split_state_uses_connection_colors() {
        let mut state = RgbLedState::new(RgbLedRole::Peripheral);
        state.handle_event(RgbLedEvent::SplitCentral(false));
        assert_eq!(state.current_color(), RgbColor::Red);
        state.handle_event(RgbLedEvent::SplitCentral(true));
        assert_eq!(state.current_color(), RgbColor::Blue);
    }

    #[test]
    fn critical_battery_change_starts_a_red_warning() {
        let mut state = RgbLedState::new(RgbLedRole::Central);
        state.handle_event(RgbLedEvent::Battery(80));
        state.handle_event(RgbLedEvent::Battery(5));
        assert_eq!(state.current_color(), RgbColor::Red);
        assert!(state.has_pending_pattern());
    }

    #[test]
    fn layer_change_blinks_highest_layer_number_on_central_only() {
        let mut central = RgbLedState::new(RgbLedRole::Central);
        let mut peripheral = RgbLedState::new(RgbLedRole::Peripheral);
        central.handle_event(RgbLedEvent::Layer(3));
        peripheral.handle_event(RgbLedEvent::Layer(3));
        assert_eq!(central.current_color(), RgbColor::Cyan);
        assert!(central.has_pending_pattern());
        assert_eq!(peripheral.current_color(), RgbColor::Off);
        assert!(!peripheral.has_pending_pattern());
        for _ in 0..12 {
            central.tick();
        }
        assert_eq!(central.current_color(), RgbColor::Off);
    }

    #[test]
    fn sleep_clears_transient_indicators() {
        let mut state = RgbLedState::new(RgbLedRole::Central);
        state.handle_event(RgbLedEvent::Layer(2));
        state.handle_event(RgbLedEvent::Sleep(true));
        assert_eq!(state.current_color(), RgbColor::Off);
        assert!(!state.has_pending_pattern());
        state.handle_event(RgbLedEvent::Sleep(false));
        assert_eq!(state.current_color(), RgbColor::Off);
    }
}
