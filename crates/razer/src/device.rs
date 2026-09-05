use std::thread;
use std::time::Duration;

use hidapi::HidDevice;

use crate::battery::{Battery, ChargingState};
use crate::error::RazerError;
use crate::protocol::{
    HID_REPORT_LEN, PollingRate, REPORT_LEN, RazerRequest, RazerResponse, RazerStatus,
    calculate_crc,
};

pub struct RazerDevice {
    device: HidDevice,
    transaction_id: u8,
}

impl RazerDevice {
    pub(crate) fn new(device: HidDevice) -> Self {
        Self {
            device,
            // OpenRazer uses 0x1f for all DeathAdder V3 HyperSpeed commands.
            transaction_id: 0x1f,
        }
    }

    const fn transaction_id(&self) -> u8 {
        self.transaction_id
    }

    fn send_request(&self, request: RazerRequest) -> Result<(), RazerError> {
        let report = request.encode();
        let mut feature_report = [0u8; HID_REPORT_LEN];
        feature_report[0] = 0x00;
        feature_report[1..].copy_from_slice(&report);
        self.device.send_feature_report(&feature_report)?;
        Ok(())
    }

    fn read_response(&self) -> Result<RazerResponse, RazerError> {
        let mut buffer = [0u8; HID_REPORT_LEN];
        buffer[0] = 0x00;
        let length = self.device.get_feature_report(&mut buffer)?;

        if length < HID_REPORT_LEN {
            return Err(RazerError::ShortReport {
                expected: HID_REPORT_LEN,
                received: length,
            });
        }

        let mut raw = [0u8; REPORT_LEN];
        raw.copy_from_slice(&buffer[1..HID_REPORT_LEN]);
        Ok(RazerResponse::parse(raw))
    }

    fn execute(&mut self, request: RazerRequest) -> Result<RazerResponse, RazerError> {
        const MAX_ATTEMPTS: usize = 5;
        let mut last_error = RazerError::Timeout;

        for _ in 0..MAX_ATTEMPTS {
            // OpenRazer resends the request on every attempt. This matters when a
            // wireless receiver is waking up and drops the first control transfer.
            if let Err(error) = self.send_request(request) {
                last_error = error;
                continue;
            }

            // This receiver family needs the longer delay used by OpenRazer.
            thread::sleep(Duration::from_millis(10));
            let response = match self.read_response() {
                Ok(response) => response,
                Err(error) => {
                    last_error = error;
                    continue;
                }
            };

            if response.transaction_id != request.transaction_id {
                last_error = RazerError::TransactionMismatch {
                    expected: request.transaction_id,
                    received: response.transaction_id,
                };
                continue;
            }

            if response.command != request.command {
                last_error = RazerError::UnexpectedCommand {
                    expected: request.command,
                    received: response.command,
                };
                continue;
            }

            let expected_crc = calculate_crc(&response.raw);
            let received_crc = response.raw[88];
            if received_crc != expected_crc {
                last_error = RazerError::CrcMismatch {
                    expected: expected_crc,
                    received: received_crc,
                };
                continue;
            }

            match response.status {
                // OpenRazer treats BUSY as a valid response for commands on
                // devices whose firmware reports success this way.
                RazerStatus::Success | RazerStatus::Busy => return Ok(response),
                RazerStatus::New => last_error = RazerError::UnexpectedStatus(RazerStatus::New),
                RazerStatus::Timeout => last_error = RazerError::Timeout,
                status => last_error = RazerError::UnexpectedStatus(status),
            }
        }

        Err(last_error)
    }

    pub fn set_polling_rate(&mut self, rate: PollingRate) -> Result<(), RazerError> {
        let transaction_id = self.transaction_id();
        let request = RazerRequest::set_polling_rate(transaction_id, rate);

        self.execute(request)?;

        Ok(())
    }

    pub fn get_polling_rate(&mut self) -> Result<PollingRate, RazerError> {
        let transaction_id = self.transaction_id();
        let request = RazerRequest::get_polling_rate(transaction_id);
        let response = self.execute(request)?;
        let raw = response.argument(0).ok_or(RazerError::InvalidArgument(0))?;
        PollingRate::from_protocol_value(raw).ok_or(RazerError::UnknownPollingRate(raw))
    }

    fn query_battery_level(&mut self) -> Result<u8, RazerError> {
        let transaction_id = self.transaction_id();
        let request = RazerRequest::battery(transaction_id);
        let response = self.execute(request)?;
        response.argument(1).ok_or(RazerError::InvalidArgument(1))
    }

    fn query_charging_state(&mut self) -> Result<ChargingState, RazerError> {
        let transaction_id = self.transaction_id();
        let request = RazerRequest::charging_status(transaction_id);
        let response = self.execute(request)?;
        let raw = response.argument(1).ok_or(RazerError::InvalidArgument(1))?;
        Ok(ChargingState::from(raw))
    }

    pub fn battery(&mut self) -> Result<Battery, RazerError> {
        let raw = self.query_battery_level()?;
        let percentage = (((raw as u16) * 100) / 255) as u8;
        let state = self.query_charging_state()?;
        Ok(Battery {
            raw,
            percentage,
            state,
        })
    }
}
