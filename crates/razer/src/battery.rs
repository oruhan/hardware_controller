#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargingState {
    Charging,
    Discharging,
}

impl From<u8> for ChargingState {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Charging,
            _ => Self::Discharging,
        }
    }
}

impl ChargingState {
    pub fn is_charging(self) -> bool {
        matches!(self, Self::Charging)
    }

    pub fn is_discharging(self) -> bool {
        matches!(self, Self::Discharging)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub raw: u8,
    pub percentage: u8,
    pub state: ChargingState,
}

impl Battery {
    pub fn is_charging(&self) -> bool {
        self.state.is_charging()
    }

    pub fn is_discharging(&self) -> bool {
        self.state.is_discharging()
    }
}
