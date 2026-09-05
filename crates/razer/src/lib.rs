#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[cfg(not(target_os = "linux"))]
compile_error!("the razer crate currently supports Linux only");

mod battery;
mod device;
mod error;
mod protocol;

pub mod devices;

pub use battery::{Battery, ChargingState};
pub use device::RazerDevice;
pub use error::RazerError;
pub use protocol::{PollingRate, RazerCommand, RazerResponse, RazerStatus};

pub use devices::deathadder_v3_hyperspeed::DaV3HS;
