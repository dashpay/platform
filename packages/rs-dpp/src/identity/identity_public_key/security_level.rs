use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;

use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::consensus::basic::data_contract::UnknownSecurityLevelError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use std::convert::TryFrom;

#[repr(u8)]
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
    Serialize_repr,
    Deserialize_repr,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    Default,
    strum::EnumIter,
)]
pub enum SecurityLevel {
    MASTER = 0,
    CRITICAL = 1,
    #[default]
    HIGH = 2,
    MEDIUM = 3,
}

impl From<SecurityLevel> for [u8; 1] {
    fn from(security_level: SecurityLevel) -> Self {
        [security_level as u8]
    }
}

impl From<SecurityLevel> for &'static [u8; 1] {
    fn from(security_level: SecurityLevel) -> Self {
        match security_level {
            SecurityLevel::MASTER => &[0],
            SecurityLevel::CRITICAL => &[1],
            SecurityLevel::HIGH => &[2],
            SecurityLevel::MEDIUM => &[3],
        }
    }
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for SecurityLevel {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}

impl TryFrom<u8> for SecurityLevel {
    type Error = ProtocolError;
    fn try_from(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::MASTER),
            1 => Ok(Self::CRITICAL),
            2 => Ok(Self::HIGH),
            3 => Ok(Self::MEDIUM),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownSecurityLevelError(
                    UnknownSecurityLevelError::new(vec![0, 1, 2, 3], value),
                ))
                .into(),
            )),
        }
    }
}

impl SecurityLevel {
    /// The full range of security levels
    pub fn full_range() -> [SecurityLevel; 4] {
        [Self::MASTER, Self::CRITICAL, Self::HIGH, Self::MEDIUM]
    }
    pub fn last() -> SecurityLevel {
        Self::MEDIUM
    }
    pub fn lowest_level() -> SecurityLevel {
        Self::MEDIUM
    }
    pub fn highest_level() -> SecurityLevel {
        Self::MASTER
    }
    pub fn stronger_security_than(self: SecurityLevel, rhs: SecurityLevel) -> bool {
        // Example:
        // self: High 2 rhs: Master 0
        // Master has a stronger security level than high
        // We expect False
        // High < Master
        // 2 < 0 <=> false
        (self as u8) < (rhs as u8)
    }

    pub fn stronger_or_equal_security_than(self: SecurityLevel, rhs: SecurityLevel) -> bool {
        (self as u8) <= (rhs as u8)
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- TryFrom<u8> valid values --

    #[test]
    fn test_try_from_u8_master() {
        assert_eq!(SecurityLevel::try_from(0u8).unwrap(), SecurityLevel::MASTER);
    }

    #[test]
    fn test_try_from_u8_critical() {
        assert_eq!(
            SecurityLevel::try_from(1u8).unwrap(),
            SecurityLevel::CRITICAL
        );
    }

    #[test]
    fn test_try_from_u8_high() {
        assert_eq!(SecurityLevel::try_from(2u8).unwrap(), SecurityLevel::HIGH);
    }

    #[test]
    fn test_try_from_u8_medium() {
        assert_eq!(SecurityLevel::try_from(3u8).unwrap(), SecurityLevel::MEDIUM);
    }

    // -- TryFrom<u8> invalid values --

    #[test]
    fn test_try_from_u8_invalid_4() {
        assert!(SecurityLevel::try_from(4u8).is_err());
    }

    #[test]
    fn test_try_from_u8_invalid_255() {
        assert!(SecurityLevel::try_from(255u8).is_err());
    }

    // -- From<SecurityLevel> for [u8; 1] --

    #[test]
    fn test_into_u8_array_master() {
        let arr: [u8; 1] = SecurityLevel::MASTER.into();
        assert_eq!(arr, [0]);
    }

    #[test]
    fn test_into_u8_array_critical() {
        let arr: [u8; 1] = SecurityLevel::CRITICAL.into();
        assert_eq!(arr, [1]);
    }

    #[test]
    fn test_into_u8_array_high() {
        let arr: [u8; 1] = SecurityLevel::HIGH.into();
        assert_eq!(arr, [2]);
    }

    #[test]
    fn test_into_u8_array_medium() {
        let arr: [u8; 1] = SecurityLevel::MEDIUM.into();
        assert_eq!(arr, [3]);
    }

    // -- From<SecurityLevel> for &'static [u8; 1] --

    #[test]
    fn test_into_static_u8_array_ref() {
        let r: &'static [u8; 1] = SecurityLevel::MASTER.into();
        assert_eq!(r, &[0]);
        let r: &'static [u8; 1] = SecurityLevel::CRITICAL.into();
        assert_eq!(r, &[1]);
        let r: &'static [u8; 1] = SecurityLevel::HIGH.into();
        assert_eq!(r, &[2]);
        let r: &'static [u8; 1] = SecurityLevel::MEDIUM.into();
        assert_eq!(r, &[3]);
    }

    // -- full_range() --

    #[test]
    fn test_full_range_contains_all_variants() {
        let range = SecurityLevel::full_range();
        assert_eq!(range.len(), 4);
        assert_eq!(range[0], SecurityLevel::MASTER);
        assert_eq!(range[1], SecurityLevel::CRITICAL);
        assert_eq!(range[2], SecurityLevel::HIGH);
        assert_eq!(range[3], SecurityLevel::MEDIUM);
    }

    // -- stronger_security_than --

    #[test]
    fn test_master_stronger_than_critical() {
        assert!(SecurityLevel::MASTER.stronger_security_than(SecurityLevel::CRITICAL));
    }

    #[test]
    fn test_master_stronger_than_high() {
        assert!(SecurityLevel::MASTER.stronger_security_than(SecurityLevel::HIGH));
    }

    #[test]
    fn test_master_stronger_than_medium() {
        assert!(SecurityLevel::MASTER.stronger_security_than(SecurityLevel::MEDIUM));
    }

    #[test]
    fn test_critical_not_stronger_than_master() {
        assert!(!SecurityLevel::CRITICAL.stronger_security_than(SecurityLevel::MASTER));
    }

    #[test]
    fn test_same_level_not_stronger() {
        assert!(!SecurityLevel::HIGH.stronger_security_than(SecurityLevel::HIGH));
    }

    #[test]
    fn test_medium_not_stronger_than_high() {
        assert!(!SecurityLevel::MEDIUM.stronger_security_than(SecurityLevel::HIGH));
    }

    // -- stronger_or_equal_security_than --

    #[test]
    fn test_master_stronger_or_equal_to_master() {
        assert!(SecurityLevel::MASTER.stronger_or_equal_security_than(SecurityLevel::MASTER));
    }

    #[test]
    fn test_master_stronger_or_equal_to_medium() {
        assert!(SecurityLevel::MASTER.stronger_or_equal_security_than(SecurityLevel::MEDIUM));
    }

    #[test]
    fn test_medium_not_stronger_or_equal_to_master() {
        assert!(!SecurityLevel::MEDIUM.stronger_or_equal_security_than(SecurityLevel::MASTER));
    }

    #[test]
    fn test_high_stronger_or_equal_to_high() {
        assert!(SecurityLevel::HIGH.stronger_or_equal_security_than(SecurityLevel::HIGH));
    }

    #[test]
    fn test_critical_stronger_or_equal_to_high() {
        assert!(SecurityLevel::CRITICAL.stronger_or_equal_security_than(SecurityLevel::HIGH));
    }

    // -- Display --

    #[test]
    fn test_display_master() {
        assert_eq!(format!("{}", SecurityLevel::MASTER), "MASTER");
    }

    #[test]
    fn test_display_critical() {
        assert_eq!(format!("{}", SecurityLevel::CRITICAL), "CRITICAL");
    }

    #[test]
    fn test_display_high() {
        assert_eq!(format!("{}", SecurityLevel::HIGH), "HIGH");
    }

    #[test]
    fn test_display_medium() {
        assert_eq!(format!("{}", SecurityLevel::MEDIUM), "MEDIUM");
    }

    // -- last / lowest_level / highest_level --

    #[test]
    fn test_last_is_medium() {
        assert_eq!(SecurityLevel::last(), SecurityLevel::MEDIUM);
    }

    #[test]
    fn test_lowest_level_is_medium() {
        assert_eq!(SecurityLevel::lowest_level(), SecurityLevel::MEDIUM);
    }

    #[test]
    fn test_highest_level_is_master() {
        assert_eq!(SecurityLevel::highest_level(), SecurityLevel::MASTER);
    }

    // -- Default --

    #[test]
    fn test_default_is_high() {
        assert_eq!(SecurityLevel::default(), SecurityLevel::HIGH);
    }

    // -- round-trip: u8 -> SecurityLevel -> [u8; 1] --

    #[test]
    fn test_round_trip_all_valid_values() {
        for val in 0u8..=3 {
            let level = SecurityLevel::try_from(val).unwrap();
            let arr: [u8; 1] = level.into();
            assert_eq!(arr[0], val);
        }
    }
}
