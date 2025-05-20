use anyhow::bail;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Eq)]
pub enum GroupActionStatus {
    ActionActive,
    ActionClosed,
}

impl TryFrom<u8> for GroupActionStatus {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ActionActive),
            1 => Ok(Self::ActionClosed),
            value => bail!("unrecognized action status: {}", value),
        }
    }
}

impl TryFrom<i32> for GroupActionStatus {
    type Error = anyhow::Error;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ActionActive),
            1 => Ok(Self::ActionClosed),
            value => bail!("unrecognized action status: {}", value),
        }
    }
}
