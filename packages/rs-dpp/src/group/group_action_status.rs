use anyhow::bail;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Eq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum GroupActionStatus {
    ActionActive,
    ActionClosed,
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for GroupActionStatus {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for GroupActionStatus {}

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
