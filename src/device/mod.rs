// Device domain types and helpers
use anyhow::Result;
use librazer::types::{BatteryCare, LightsAlwaysOn, LogoMode, FanMode, PerfMode};
use librazer::{command, device};

#[derive(Debug, Clone, PartialEq)]
pub struct CompleteDeviceState {
    pub perf_mode: PerfMode,
    pub fan_mode: FanMode,
    pub fan_rpm: Option<u16>,
    pub logo_mode: LogoMode,
    pub keyboard_brightness: u8,
    pub lights_always_on: LightsAlwaysOn,
    pub battery_care: BatteryCare,
}

impl Default for CompleteDeviceState {
    fn default() -> Self {
        Self {
            perf_mode: PerfMode::Performance,
            fan_mode: FanMode::Auto,
            fan_rpm: None,
            logo_mode: LogoMode::Off,
            keyboard_brightness: 50,
            lights_always_on: LightsAlwaysOn::Disable,
            battery_care: BatteryCare::Enable,
        }
    }
}

impl CompleteDeviceState {
    pub fn read_from_device(device: &device::Device) -> Result<Self> {
        let (perf_mode, fan_mode) = command::get_perf_mode(device)?;
        let fan_rpm = match fan_mode {
            FanMode::Manual => Some(command::get_fan_rpm(device, librazer::types::FanZone::Zone1)?),
            FanMode::Auto => None,
        };
        let logo_mode = command::get_logo_mode(device)?;
        let keyboard_brightness = command::get_keyboard_brightness(device)?;
        let lights_always_on = command::get_lights_always_on(device)?;
        let battery_care = command::get_battery_care(device)?;

        Ok(Self {
            perf_mode,
            fan_mode,
            fan_rpm,
            logo_mode,
            keyboard_brightness,
            lights_always_on,
            battery_care,
        })
    }
}
