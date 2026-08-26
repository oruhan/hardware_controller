use std::thread;
use std::time::Duration;

use hidapi::HidDevice;

use crate::battery::{ Battery, ChargingState };
use crate::error::RazerError;
use crate::protocol::{
    calculate_crc,
    PollingRate,
    RazerRequest,
    RazerResponse,
    RazerStatus,
    HID_REPORT_LEN,
    REPORT_LEN,
};

pub struct RazerDevice {
    device: HidDevice,
    transaction_id: u8,
}

impl RazerDevice {
    pub(crate) fn new(device: HidDevice) -> Self {
        Self {
            device,
            transaction_id: 0x06,
        }
    }

    fn next_transaction_id(&mut self) -> u8 {
        let id = self.transaction_id;
        self.transaction_id = self.transaction_id.wrapping_add(1);
        id
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
        self.send_request(request)?;

        const MAX_ATTEMPTS: usize = 5;

        for _ in 0..MAX_ATTEMPTS {
            thread::sleep(Duration::from_millis(5));
            let response = self.read_response()?;

            if response.transaction_id != request.transaction_id {
                continue;
            }

            match response.status {
                RazerStatus::New | RazerStatus::Busy => continue,
                RazerStatus::Success => {
                    if response.command != request.command {
                        return Err(RazerError::UnexpectedCommand {
                            expected: request.command,
                            received: response.command,
                        });
                    }

                    let expected_crc = calculate_crc(&response.raw);
                    let received_crc = response.raw[88];

                    if received_crc != expected_crc {
                        return Err(RazerError::CrcMismatch {
                            expected: expected_crc,
                            received: received_crc,
                        });
                    }

                    return Ok(response);
                }
                RazerStatus::Failure => {
                    return Err(RazerError::UnexpectedStatus(RazerStatus::Failure));
                }
                RazerStatus::Timeout => return Err(RazerError::Timeout),
                status => return Err(RazerError::UnexpectedStatus(status)),
            }
        }

        Err(RazerError::Timeout)
    }

    pub(crate) fn set_polling_rate(
        &mut self,
        rate: PollingRate,
    ) -> Result<(), RazerError> {
        let transaction_id = self.next_transaction_id();
        let request = RazerRequest::polling_rate(transaction_id, rate.protocol_value());
        let response = self.execute(request)?;

        if response.argument(0) != Some(0x01) {
            return Err(RazerError::InvalidArgument(0));
        }

        if response.argument(1) != Some(rate.protocol_value()) {
            return Err(RazerError::InvalidArgument(1));
        }

        Ok(())
    }

    fn query_battery_level(&mut self) -> Result<u8, RazerError> {
        let transaction_id = self.next_transaction_id();
        let request = RazerRequest::battery(transaction_id);
        let response = self.execute(request)?;
        response.argument(1).ok_or(RazerError::InvalidArgument(1))
    }

    fn query_charging_state(&mut self) -> Result<ChargingState, RazerError> {
        let transaction_id = self.next_transaction_id();
        let request = RazerRequest::charging_status(transaction_id);
        let response = self.execute(request)?;
        let raw = response.argument(1).ok_or(RazerError::InvalidArgument(1))?;
        Ok(ChargingState::from(raw))
    }

    pub fn battery(&mut self) -> Result<Battery, RazerError> {
        let raw = self.query_battery_level()?;
        let percentage = (((raw as u16) * 100) / 255) as u8;
        let state = self.query_charging_state()?;
        Ok(Battery { raw, percentage, state })
    }
}
