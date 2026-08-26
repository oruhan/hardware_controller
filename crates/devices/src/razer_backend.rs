// SPDX-License-Identifier: Apache-2.0
use crate::{
    BatteryStatus,
    ChargingState,
    ConnectionType,
    Device,
    DeviceDescriptor,
    DeviceError,
    DeviceKind,
};
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
        let connection = if self.0.is_wireless() {
            ConnectionType::Wireless2_4Ghz
        } else {
            ConnectionType::Usb
        };

        Ok(BatteryStatus {
            percentage: battery.percentage,
            raw: battery.raw,
            state: battery.state.into(),
            connection,
        })
    }

    fn supports_polling_rate(&self) -> bool {
        true
    }

    fn get_polling_rate(&mut self) -> Result<crate::PollingRate, DeviceError> {
        self.0.get_polling_rate().map_err(|e| DeviceError(e.to_string()))
    }

    fn set_polling_rate(&mut self, rate: crate::PollingRate) -> Result<(), DeviceError> {
        self.0.set_polling_rate(rate).map_err(|e| DeviceError(e.to_string()))
    }
}

fn open_deathadder_v3_hyperspeed() -> Result<Box<dyn Device>, DeviceError> {
    DaV3HS::open()
        .map(|d| Box::new(DeathAdderV3HyperSpeed(d)) as Box<dyn Device>)
        .map_err(|e| DeviceError(e.to_string()))
}

pub(super) const RAZER_DEVICES: &[DeviceDescriptor] = &[
    DeviceDescriptor {
        brand: "Razer",
        model: "DeathAdder V3 HyperSpeed",
        kind: DeviceKind::Mouse,
        image_svg: include_str!("../../gui/assets/mouse.svg"),
        open: open_deathadder_v3_hyperspeed,
    },
];
