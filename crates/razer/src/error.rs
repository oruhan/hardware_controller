use crate::protocol::{ RazerCommand, RazerStatus };

#[derive(Debug, thiserror::Error)]
pub enum RazerError {
    #[error("HID error: {0}")] Hid(#[from] hidapi::HidError),

    #[error("DeathAdder V3 HyperSpeed battery interface not found")]
    DeviceNotFound,

    #[error("short feature report: expected {expected} bytes, received {received}")] ShortReport {
        expected: usize,
        received: usize,
    },

    #[error("unexpected response status: {0:?}")] UnexpectedStatus(RazerStatus),

    #[error(
        "unexpected command: expected {:02X}:{:02X}, received {:02X}:{:02X}",
        expected.class,
        expected.id,
        received.class,
        received.id
    )] UnexpectedCommand {
        expected: RazerCommand,
        received: RazerCommand,
    },

    #[error(
        "transaction mismatch: expected {:02X}, received {:02X}",
        expected,
        received
    )] TransactionMismatch {
        expected: u8,
        received: u8,
    },

    #[error("CRC mismatch: expected {expected:02X}, received {received:02X}")] CrcMismatch {
        expected: u8,
        received: u8,
    },

    #[error("command timed out")]
    Timeout,

    #[error("invalid argument index: {0}")] InvalidArgument(usize),
}
