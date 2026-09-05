use hidapi::HidApi;

use crate::battery::Battery;
use crate::device::RazerDevice;
use crate::error::RazerError;
use crate::protocol::PollingRate;

const RAZER_VID: u16 = 0x1532;

const DEATHADDER_V3_HYPERSPEED_WIRED_PID: u16 = 0x00c4;
const DEATHADDER_V3_HYPERSPEED_WIRELESS_PID: u16 = 0x00c5;

pub struct DaV3HS {
    device: RazerDevice,
    wireless: bool,
}

impl DaV3HS {
    pub fn open() -> Result<Self, RazerError> {
        let api = HidApi::new()?;

        Self::open_with_api(&api)
    }

    pub fn open_with_api(api: &HidApi) -> Result<Self, RazerError> {
        for info in api.device_list() {
            if info.vendor_id() != RAZER_VID {
                continue;
            }

            let pid = info.product_id();

            if pid != DEATHADDER_V3_HYPERSPEED_WIRED_PID
                && pid != DEATHADDER_V3_HYPERSPEED_WIRELESS_PID
            {
                continue;
            }

            /*
             * The battery/control collection is:
             *
             *   Usage Page: 0x0001
             *   Usage:      0x0002
             *   Interface:  0
             *
             * This collection exposes the 90-byte feature
             * report used by the battery and charging commands.
             */
            if info.interface_number() != 0 {
                continue;
            }

            if info.usage_page() != 0x0001 {
                continue;
            }

            if info.usage() != 0x0002 {
                continue;
            }

            let device = info.open_device(api)?;

            return Ok(Self {
                device: RazerDevice::new(device),
                wireless: pid == DEATHADDER_V3_HYPERSPEED_WIRELESS_PID,
            });
        }

        Err(RazerError::DeviceNotFound)
    }

    /// True when connected through the HyperSpeed 2.4GHz dongle rather
    /// than the wired USB PID.
    pub fn is_wireless(&self) -> bool {
        self.wireless
    }

    pub fn set_polling_rate(&mut self, rate: PollingRate) -> Result<(), RazerError> {
        self.device.set_polling_rate(rate)
    }

    pub fn get_polling_rate(&mut self) -> Result<PollingRate, RazerError> {
        self.device.get_polling_rate()
    }

    pub fn battery(&mut self) -> Result<Battery, RazerError> {
        self.device.battery()
    }
}
