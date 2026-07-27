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

    // -- TryFrom<u8> valid --
    #[test]
    fn test_security_level_try_from_u8_all_valid() {
        assert_eq!(SecurityLevel::try_from(0u8).unwrap(), SecurityLevel::MASTER);
        assert_eq!(
            SecurityLevel::try_from(1u8).unwrap(),
            SecurityLevel::CRITICAL
        );
        assert_eq!(SecurityLevel::try_from(2u8).unwrap(), SecurityLevel::HIGH);
        assert_eq!(SecurityLevel::try_from(3u8).unwrap(), SecurityLevel::MEDIUM);
    }

    // -- TryFrom<u8> invalid returns UnknownSecurityLevelError --
    #[test]
    fn test_security_level_try_from_u8_invalid_is_consensus_error() {
        let err = SecurityLevel::try_from(4u8).unwrap_err();
        // Confirm it is a ProtocolError::ConsensusError wrapping BasicError::UnknownSecurityLevelError.
        match err {
            ProtocolError::ConsensusError(ce) => match *ce {
                ConsensusError::BasicError(BasicError::UnknownSecurityLevelError(_)) => {}
                other => panic!("unexpected inner consensus error: {:?}", other),
            },
            other => panic!("expected ProtocolError::ConsensusError, got {:?}", other),
        }
    }

    #[test]
    fn test_security_level_try_from_u8_invalid_255() {
        assert!(SecurityLevel::try_from(255u8).is_err());
    }

    // -- From<SecurityLevel> for [u8; 1] (owned) --
    #[test]
    fn test_security_level_to_owned_byte_array() {
        let arr: [u8; 1] = SecurityLevel::MASTER.into();
        assert_eq!(arr, [0]);
        let arr: [u8; 1] = SecurityLevel::CRITICAL.into();
        assert_eq!(arr, [1]);
        let arr: [u8; 1] = SecurityLevel::HIGH.into();
        assert_eq!(arr, [2]);
        let arr: [u8; 1] = SecurityLevel::MEDIUM.into();
        assert_eq!(arr, [3]);
    }

    // -- From<SecurityLevel> for &'static [u8; 1] --
    #[test]
    fn test_security_level_to_static_byte_ref_all_variants() {
        let r: &'static [u8; 1] = SecurityLevel::MASTER.into();
        assert_eq!(r, &[0u8]);
        let r: &'static [u8; 1] = SecurityLevel::CRITICAL.into();
        assert_eq!(r, &[1u8]);
        let r: &'static [u8; 1] = SecurityLevel::HIGH.into();
        assert_eq!(r, &[2u8]);
        let r: &'static [u8; 1] = SecurityLevel::MEDIUM.into();
        assert_eq!(r, &[3u8]);
    }

    // -- Display --
    #[test]
    fn test_security_level_display_matches_debug_form() {
        assert_eq!(format!("{}", SecurityLevel::MASTER), "MASTER");
        assert_eq!(format!("{}", SecurityLevel::CRITICAL), "CRITICAL");
        assert_eq!(format!("{}", SecurityLevel::HIGH), "HIGH");
        assert_eq!(format!("{}", SecurityLevel::MEDIUM), "MEDIUM");
    }

    // -- Default is HIGH --
    #[test]
    fn test_security_level_default_is_high() {
        assert_eq!(SecurityLevel::default(), SecurityLevel::HIGH);
    }

    // -- full_range, last, lowest_level, highest_level --
    #[test]
    fn test_security_level_full_range() {
        let r = SecurityLevel::full_range();
        assert_eq!(r.len(), 4);
        assert_eq!(
            r,
            [
                SecurityLevel::MASTER,
                SecurityLevel::CRITICAL,
                SecurityLevel::HIGH,
                SecurityLevel::MEDIUM,
            ]
        );
    }

    #[test]
    fn test_security_level_last_and_lowest_are_medium() {
        assert_eq!(SecurityLevel::last(), SecurityLevel::MEDIUM);
        assert_eq!(SecurityLevel::lowest_level(), SecurityLevel::MEDIUM);
    }

    #[test]
    fn test_security_level_highest_is_master() {
        assert_eq!(SecurityLevel::highest_level(), SecurityLevel::MASTER);
    }

    // -- stronger_security_than: strict < --
    #[test]
    fn test_stronger_security_than_master_vs_medium() {
        // Master (0) is stronger than Medium (3) because 0 < 3.
        assert!(SecurityLevel::MASTER.stronger_security_than(SecurityLevel::MEDIUM));
        // Medium is NOT stronger than Master.
        assert!(!SecurityLevel::MEDIUM.stronger_security_than(SecurityLevel::MASTER));
    }

    #[test]
    fn test_stronger_security_than_is_not_reflexive() {
        // A level is not strictly stronger than itself.
        assert!(!SecurityLevel::HIGH.stronger_security_than(SecurityLevel::HIGH));
        assert!(!SecurityLevel::MASTER.stronger_security_than(SecurityLevel::MASTER));
    }

    #[test]
    fn test_stronger_security_than_full_matrix() {
        let all = SecurityLevel::full_range();
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                // full_range is ordered strongest -> weakest, so index i < j iff a is stronger.
                assert_eq!(a.stronger_security_than(*b), i < j);
            }
        }
    }

    // -- stronger_or_equal_security_than --
    #[test]
    fn test_stronger_or_equal_security_than_reflexive() {
        for lvl in SecurityLevel::full_range() {
            assert!(lvl.stronger_or_equal_security_than(lvl));
        }
    }

    #[test]
    fn test_stronger_or_equal_security_than_strict() {
        assert!(SecurityLevel::MASTER.stronger_or_equal_security_than(SecurityLevel::HIGH));
        assert!(!SecurityLevel::HIGH.stronger_or_equal_security_than(SecurityLevel::MASTER));
    }

    // -- Ordering derives --
    #[test]
    fn test_security_level_ordering_master_lt_critical_lt_high_lt_medium() {
        assert!(SecurityLevel::MASTER < SecurityLevel::CRITICAL);
        assert!(SecurityLevel::CRITICAL < SecurityLevel::HIGH);
        assert!(SecurityLevel::HIGH < SecurityLevel::MEDIUM);
    }

    // -- round-trip u8 -> SecurityLevel -> u8 --
    #[test]
    fn test_security_level_round_trip_u8() {
        for v in 0u8..=3 {
            let lvl = SecurityLevel::try_from(v).unwrap();
            assert_eq!(lvl as u8, v);
        }
    }
}
