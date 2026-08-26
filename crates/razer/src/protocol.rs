pub const REPORT_LEN: usize = 90;
pub const HID_REPORT_LEN: usize = REPORT_LEN + 1;

pub const STATUS_NEW: u8 = 0x00;
pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_SUCCESS: u8 = 0x02;
pub const STATUS_FAILURE: u8 = 0x03;
pub const STATUS_TIMEOUT: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollingRate {
    Hz125,
    Hz500,
    Hz1000,
}

impl PollingRate {
    pub const fn hz(self) -> u16 {
        match self {
            Self::Hz125 => 125,
            Self::Hz500 => 500,
            Self::Hz1000 => 1000,
        }
    }

    pub(crate) const fn protocol_value(self) -> u8 {
        match self {
            Self::Hz125 => 0x08,
            Self::Hz500 => 0x02,
            Self::Hz1000 => 0x01,
        }
    }

    pub const fn from_protocol_value(value: u8) -> Option<Self> {
        match value {
            0x08 => Some(Self::Hz125),
            0x02 => Some(Self::Hz500),
            0x01 => Some(Self::Hz1000),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RazerStatus {
    New,
    Busy,
    Success,
    Failure,
    Timeout,
    Unknown(u8),
}

impl From<u8> for RazerStatus {
    fn from(value: u8) -> Self {
        match value {
            STATUS_NEW => Self::New,
            STATUS_BUSY => Self::Busy,
            STATUS_SUCCESS => Self::Success,
            STATUS_FAILURE => Self::Failure,
            STATUS_TIMEOUT => Self::Timeout,
            value => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RazerCommand {
    pub class: u8,
    pub id: u8,
}

impl RazerCommand {
    pub const BATTERY: Self = Self {
        class: 0x07,
        id: 0x80,
    };

    pub const CHARGING_STATUS: Self = Self {
        class: 0x07,
        id: 0x84,
    };

    pub const POLLING_RATE_SET: Self = Self {
        class: 0x00,
        id: 0x0e,
    };

    pub const POLLING_RATE_GET: Self = Self {
        class: 0x00,
        id: 0x8e,
    };
}

#[derive(Debug, Clone)]
pub struct RazerResponse {
    pub status: RazerStatus,
    pub transaction_id: u8,
    pub command: RazerCommand,
    pub arguments: [u8; 80],
    pub raw: [u8; REPORT_LEN],
}

impl RazerResponse {
    pub fn parse(raw: [u8; REPORT_LEN]) -> Self {
        let mut arguments = [0u8; 80];
        arguments.copy_from_slice(&raw[8..88]);

        Self {
            status: RazerStatus::from(raw[0]),
            transaction_id: raw[1],
            command: RazerCommand {
                class: raw[6],
                id: raw[7],
            },
            arguments,
            raw,
        }
    }

    pub fn argument(&self, index: usize) -> Option<u8> {
        self.arguments.get(index).copied()
    }

    pub fn is_success(&self) -> bool {
        self.status == RazerStatus::Success
    }

    pub fn verify_crc(&self) -> bool {
        calculate_crc(&self.raw) == self.raw[88]
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RazerRequest {
    pub(crate) transaction_id: u8,
    pub(crate) command: RazerCommand,
    pub(crate) arguments: [u8; 80],
    pub(crate) argument_len: usize,
}

impl RazerRequest {
    pub(crate) fn new(transaction_id: u8, command: RazerCommand) -> Self {
        Self {
            transaction_id,
            command,
            arguments: [0u8; 80],
            argument_len: 0,
        }
    }

    pub(crate) fn with_arguments(mut self, arguments: &[u8]) -> Self {
        assert!(arguments.len() <= 80);

        self.arguments[..arguments.len()].copy_from_slice(arguments);
        self.argument_len = arguments.len();

        self
    }

    pub(crate) fn battery(transaction_id: u8) -> Self {
        Self::new(transaction_id, RazerCommand::BATTERY)
    }

    pub(crate) fn charging_status(transaction_id: u8) -> Self {
        Self::new(transaction_id, RazerCommand::CHARGING_STATUS)
    }

    pub(crate) fn set_polling_rate(transaction_id: u8, rate: PollingRate) -> Self {
        Self::new(transaction_id, RazerCommand::POLLING_RATE_SET).with_arguments(
            &[0x01, rate.protocol_value()]
        )
    }

    pub(crate) fn get_polling_rate(transaction_id: u8) -> Self {
        Self::new(transaction_id, RazerCommand::POLLING_RATE_GET).with_arguments(&[0x01, 0x00])
    }

    pub(crate) fn encode(&self) -> [u8; REPORT_LEN] {
        let mut report = [0u8; REPORT_LEN];

        report[0] = STATUS_NEW;
        report[1] = self.transaction_id;

        report[2] = 0x00;
        report[3] = 0x00;
        report[4] = 0x00;

        report[5] = self.argument_len as u8;
        report[6] = self.command.class;
        report[7] = self.command.id;

        report[8..8 + self.argument_len].copy_from_slice(&self.arguments[..self.argument_len]);

        report[88] = calculate_crc(&report);
        report[89] = 0x00;

        report
    }
}

pub(crate) fn calculate_crc(report: &[u8; REPORT_LEN]) -> u8 {
    report[2..88]
        .iter()
        .copied()
        .fold(0u8, |crc, byte| crc ^ byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polling_rate_set_request_arguments() {
        let request = RazerRequest::set_polling_rate(0x1f, PollingRate::Hz125);
        let report = request.encode();

        assert_eq!(report[0], STATUS_NEW);
        assert_eq!(report[1], 0x1f);
        assert_eq!(report[5], 0x02);
        assert_eq!(report[6], 0x00);
        assert_eq!(report[7], 0x0e);
        assert_eq!(report[8], 0x01);
        assert_eq!(report[9], 0x08);
    }

    #[test]
    fn polling_rate_set_request_crc() {
        let request = RazerRequest::set_polling_rate(0x1f, PollingRate::Hz125);
        let report = request.encode();

        assert_eq!(report[88], calculate_crc(&report));
    }

    #[test]
    fn polling_rate_get_request_arguments() {
        let request = RazerRequest::get_polling_rate(0x1f);
        let report = request.encode();

        assert_eq!(report[6], 0x00);
        assert_eq!(report[7], 0x8e);
        assert_eq!(report[8], 0x01);
        assert_eq!(report[9], 0x00);
    }

    #[test]
    fn polling_rate_from_protocol_value_roundtrip() {
        assert_eq!(PollingRate::from_protocol_value(0x08), Some(PollingRate::Hz125));
        assert_eq!(PollingRate::from_protocol_value(0x02), Some(PollingRate::Hz500));
        assert_eq!(PollingRate::from_protocol_value(0x01), Some(PollingRate::Hz1000));
        assert_eq!(PollingRate::from_protocol_value(0xff), None);
    }
}
