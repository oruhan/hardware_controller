// SPDX-License-Identifier: Apache-2.0
mod razer_backend;

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Headset,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargingState {
    Charging,
    Discharging,
    Unknown,
}

/// How the device is physically talking to the host right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Usb,
    Wireless2_4Ghz,
    Bluetooth,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct BatteryStatus {
    pub percentage: u8,
    pub raw: u8,
    pub state: ChargingState,
    pub connection: ConnectionType,
}

#[derive(Debug, Clone)]
pub struct DeviceError(pub String);

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DeviceError {}

pub trait Device: Send {
    fn poll_battery(&mut self) -> Result<BatteryStatus, DeviceError>;
}

pub struct DeviceDescriptor {
    pub brand: &'static str,
    pub model: &'static str,
    pub kind: DeviceKind,
    pub image_svg: &'static str,
    pub open: fn() -> Result<Box<dyn Device>, DeviceError>,
}

pub fn catalog() -> Vec<&'static DeviceDescriptor> {
    razer_backend::RAZER_DEVICES.iter().collect()
}
