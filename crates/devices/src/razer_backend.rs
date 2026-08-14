// SPDX-License-Identifier: Apache-2.0
use crate::{ BatteryStatus, ChargingState, Device, DeviceDescriptor, DeviceError, DeviceKind };
use razer::DaV3HS;

impl From<razer::ChargingState> for ChargingState {
    fn from(value: razer::ChargingState) -> Self {
        match value {
            razer::ChargingState::Charging => Self::Charging,
            razer::ChargingState::Discharging => Self::Discharging,
        }
    }
}

struct DeathAdderV3HyperSpeed(DaV3HS);

impl Device for DeathAdderV3HyperSpeed {
    fn poll_battery(&mut self) -> Result<BatteryStatus, DeviceError> {
        let battery = self.0.battery().map_err(|e| DeviceError(e.to_string()))?;
        Ok(BatteryStatus {
            percentage: battery.percentage,
            raw: battery.raw,
            state: battery.state.into(),
        })
    }
}

fn open_deathadder_v3_hyperspeed() -> Result<Box<dyn Device>, DeviceError> {
    DaV3HS::open()
        .map(|d| Box::new(DeathAdderV3HyperSpeed(d)) as Box<dyn Device>)
        .map_err(|e| DeviceError(e.to_string()))
}

// A real Razer keyboard uses a different HID interface/command-class
// than the mouse's battery protocol - this entry is a wiring placeholder
// until that protocol is implemented, so it's deliberately left out of
// RAZER_DEVICES rather than shipped half-working.

pub const RAZER_DEVICES: &[DeviceDescriptor] = &[
    DeviceDescriptor {
        brand: "Razer",
        model: "DeathAdder V3 HyperSpeed",
        kind: DeviceKind::Mouse,
        image_svg: include_str!("../../gui/assets/mouse.svg"),
        open: open_deathadder_v3_hyperspeed,
    },
];
