pub const REPORT_LEN: usize = 90;
pub const HID_REPORT_LEN: usize = REPORT_LEN + 1;

pub const STATUS_NEW: u8 = 0x00;
pub const STATUS_BUSY: u8 = 0x01;
pub const STATUS_SUCCESS: u8 = 0x02;
pub const STATUS_FAILURE: u8 = 0x03;
pub const STATUS_TIMEOUT: u8 = 0x04;

pub const REQUEST_TYPE: u8 = 0x02;

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
}

impl RazerRequest {
    pub(crate) fn new(transaction_id: u8, command: RazerCommand) -> Self {
        Self {
            transaction_id,
            command,
        }
    }

    pub(crate) fn battery(transaction_id: u8) -> Self {
        Self::new(transaction_id, RazerCommand::BATTERY)
    }

    pub(crate) fn charging_status(transaction_id: u8) -> Self {
        Self::new(transaction_id, RazerCommand::CHARGING_STATUS)
    }

    pub(crate) fn encode(&self) -> [u8; REPORT_LEN] {
        let mut report = [0u8; REPORT_LEN];

        report[0] = STATUS_NEW;
        report[1] = self.transaction_id;

        report[2] = 0x00;
        report[3] = 0x00;
        report[4] = 0x00;

        report[5] = REQUEST_TYPE;
        report[6] = self.command.class;
        report[7] = self.command.id;

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
