// SPDX-License-Identifier: Apache-2.0
mod razer_backend;

pub use razer_backend::RAZER_DEVICES;

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

#[derive(Debug, Clone, Copy)]
pub struct BatteryStatus {
    pub percentage: u8,
    pub raw: u8,
    pub state: ChargingState,
}

#[derive(Debug, Clone)]
pub struct DeviceError(pub String);

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DeviceError {}

/// One connected peripheral, independent of whichever vendor SDK
/// actually talks to it. Every brand backend (see `razer_backend`)
/// implements this so the GUI layer only ever depends on this trait.
pub trait Device: Send {
    fn poll_battery(&mut self) -> Result<BatteryStatus, DeviceError>;
}

/// Static entry shown in the "add device" picker before anything is
/// opened - `open` is only called once the user actually selects it.
pub struct DeviceDescriptor {
    pub brand: &'static str,
    pub model: &'static str,
    pub kind: DeviceKind,
    pub image_svg: &'static str,
    pub open: fn() -> Result<Box<dyn Device>, DeviceError>,
}

/// Every descriptor from every backend, for the picker to list. New
/// brands add their own `&[DeviceDescriptor]` here.
pub fn catalog() -> Vec<&'static DeviceDescriptor> {
    RAZER_DEVICES.iter().collect()
}
