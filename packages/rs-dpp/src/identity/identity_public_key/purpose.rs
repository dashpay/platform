use crate::identity::Purpose::{
    AUTHENTICATION, DECRYPTION, ENCRYPTION, OWNER, SYSTEM, TRANSFER, VOTING,
};
use anyhow::bail;
use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;
use serde_repr::{Deserialize_repr, Serialize_repr};
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
    Ord,
    PartialOrd,
    Encode,
    Decode,
    Default,
    strum::EnumIter,
)]
pub enum Purpose {
    /// at least one authentication key must be registered for all security levels
    #[default]
    AUTHENTICATION = 0,
    /// this key cannot be used for signing documents
    ENCRYPTION = 1,
    /// this key cannot be used for signing documents
    DECRYPTION = 2,
    /// this key is used to sign credit transfer and withdrawal state transitions
    /// this key can also be used by identities for claims and transfers of tokens
    TRANSFER = 3,
    /// this key cannot be used for signing documents
    SYSTEM = 4,
    /// this key cannot be used for signing documents
    VOTING = 5,
    /// this key is used to prove ownership of a masternode or evonode
    OWNER = 6,
}

impl From<Purpose> for [u8; 1] {
    fn from(purpose: Purpose) -> Self {
        [purpose as u8]
    }
}

impl From<Purpose> for &'static [u8; 1] {
    fn from(purpose: Purpose) -> Self {
        match purpose {
            AUTHENTICATION => &[0],
            ENCRYPTION => &[1],
            DECRYPTION => &[2],
            TRANSFER => &[3],
            SYSTEM => &[4],
            VOTING => &[5],
            OWNER => &[6],
        }
    }
}

impl TryFrom<u8> for Purpose {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AUTHENTICATION),
            1 => Ok(ENCRYPTION),
            2 => Ok(DECRYPTION),
            3 => Ok(TRANSFER),
            4 => Ok(SYSTEM),
            5 => Ok(VOTING),
            6 => Ok(OWNER),
            value => bail!("unrecognized purpose: {}", value),
        }
    }
}

impl TryFrom<i32> for Purpose {
    type Error = anyhow::Error;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AUTHENTICATION),
            1 => Ok(ENCRYPTION),
            2 => Ok(DECRYPTION),
            3 => Ok(TRANSFER),
            4 => Ok(SYSTEM),
            5 => Ok(VOTING),
            6 => Ok(OWNER),
            value => bail!("unrecognized purpose: {}", value),
        }
    }
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for Purpose {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}
impl std::fmt::Display for Purpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Purpose {
    /// The full range of purposes
    pub fn full_range() -> [Purpose; 6] {
        [
            AUTHENTICATION,
            ENCRYPTION,
            DECRYPTION,
            TRANSFER,
            VOTING,
            OWNER,
        ]
    }
    /// Just the authentication and withdraw purposes
    pub fn searchable_purposes() -> [Purpose; 3] {
        [AUTHENTICATION, TRANSFER, VOTING]
    }
    /// Just the encryption and decryption purposes
    pub fn encryption_decryption() -> [Purpose; 2] {
        [ENCRYPTION, DECRYPTION]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- TryFrom<u8> valid values --

    #[test]
    fn test_try_from_u8_authentication() {
        assert_eq!(Purpose::try_from(0u8).unwrap(), AUTHENTICATION);
    }

    #[test]
    fn test_try_from_u8_encryption() {
        assert_eq!(Purpose::try_from(1u8).unwrap(), ENCRYPTION);
    }

    #[test]
    fn test_try_from_u8_decryption() {
        assert_eq!(Purpose::try_from(2u8).unwrap(), DECRYPTION);
    }

    #[test]
    fn test_try_from_u8_transfer() {
        assert_eq!(Purpose::try_from(3u8).unwrap(), TRANSFER);
    }

    #[test]
    fn test_try_from_u8_system() {
        assert_eq!(Purpose::try_from(4u8).unwrap(), SYSTEM);
    }

    #[test]
    fn test_try_from_u8_voting() {
        assert_eq!(Purpose::try_from(5u8).unwrap(), VOTING);
    }

    #[test]
    fn test_try_from_u8_owner() {
        assert_eq!(Purpose::try_from(6u8).unwrap(), OWNER);
    }

    // -- TryFrom<u8> invalid values --

    #[test]
    fn test_try_from_u8_invalid_7() {
        assert!(Purpose::try_from(7u8).is_err());
    }

    #[test]
    fn test_try_from_u8_invalid_255() {
        assert!(Purpose::try_from(255u8).is_err());
    }

    // -- TryFrom<i32> valid values --

    #[test]
    fn test_try_from_i32_all_valid() {
        assert_eq!(Purpose::try_from(0i32).unwrap(), AUTHENTICATION);
        assert_eq!(Purpose::try_from(1i32).unwrap(), ENCRYPTION);
        assert_eq!(Purpose::try_from(2i32).unwrap(), DECRYPTION);
        assert_eq!(Purpose::try_from(3i32).unwrap(), TRANSFER);
        assert_eq!(Purpose::try_from(4i32).unwrap(), SYSTEM);
        assert_eq!(Purpose::try_from(5i32).unwrap(), VOTING);
        assert_eq!(Purpose::try_from(6i32).unwrap(), OWNER);
    }

    // -- TryFrom<i32> invalid values --

    #[test]
    fn test_try_from_i32_invalid_positive() {
        assert!(Purpose::try_from(7i32).is_err());
    }

    #[test]
    fn test_try_from_i32_invalid_negative() {
        assert!(Purpose::try_from(-1i32).is_err());
    }

    // -- From<Purpose> for [u8; 1] --

    #[test]
    fn test_into_u8_array_all() {
        let arr: [u8; 1] = AUTHENTICATION.into();
        assert_eq!(arr, [0]);
        let arr: [u8; 1] = ENCRYPTION.into();
        assert_eq!(arr, [1]);
        let arr: [u8; 1] = DECRYPTION.into();
        assert_eq!(arr, [2]);
        let arr: [u8; 1] = TRANSFER.into();
        assert_eq!(arr, [3]);
        let arr: [u8; 1] = SYSTEM.into();
        assert_eq!(arr, [4]);
        let arr: [u8; 1] = VOTING.into();
        assert_eq!(arr, [5]);
        let arr: [u8; 1] = OWNER.into();
        assert_eq!(arr, [6]);
    }

    // -- From<Purpose> for &'static [u8; 1] --

    #[test]
    fn test_into_static_u8_array_ref_all() {
        let r: &'static [u8; 1] = AUTHENTICATION.into();
        assert_eq!(r, &[0]);
        let r: &'static [u8; 1] = ENCRYPTION.into();
        assert_eq!(r, &[1]);
        let r: &'static [u8; 1] = DECRYPTION.into();
        assert_eq!(r, &[2]);
        let r: &'static [u8; 1] = TRANSFER.into();
        assert_eq!(r, &[3]);
        let r: &'static [u8; 1] = SYSTEM.into();
        assert_eq!(r, &[4]);
        let r: &'static [u8; 1] = VOTING.into();
        assert_eq!(r, &[5]);
        let r: &'static [u8; 1] = OWNER.into();
        assert_eq!(r, &[6]);
    }

    // -- full_range() --

    #[test]
    fn test_full_range_has_six_elements() {
        let range = Purpose::full_range();
        assert_eq!(range.len(), 6);
    }

    #[test]
    fn test_full_range_excludes_system() {
        let range = Purpose::full_range();
        assert!(!range.contains(&SYSTEM));
    }

    #[test]
    fn test_full_range_contains_expected() {
        let range = Purpose::full_range();
        assert_eq!(
            range,
            [
                AUTHENTICATION,
                ENCRYPTION,
                DECRYPTION,
                TRANSFER,
                VOTING,
                OWNER
            ]
        );
    }

    // -- searchable_purposes() --

    #[test]
    fn test_searchable_purposes() {
        let purposes = Purpose::searchable_purposes();
        assert_eq!(purposes.len(), 3);
        assert_eq!(purposes, [AUTHENTICATION, TRANSFER, VOTING]);
    }

    // -- encryption_decryption() --

    #[test]
    fn test_encryption_decryption() {
        let purposes = Purpose::encryption_decryption();
        assert_eq!(purposes.len(), 2);
        assert_eq!(purposes, [ENCRYPTION, DECRYPTION]);
    }

    // -- Display --

    #[test]
    fn test_display_authentication() {
        assert_eq!(format!("{}", AUTHENTICATION), "AUTHENTICATION");
    }

    #[test]
    fn test_display_encryption() {
        assert_eq!(format!("{}", ENCRYPTION), "ENCRYPTION");
    }

    #[test]
    fn test_display_decryption() {
        assert_eq!(format!("{}", DECRYPTION), "DECRYPTION");
    }

    #[test]
    fn test_display_transfer() {
        assert_eq!(format!("{}", TRANSFER), "TRANSFER");
    }

    #[test]
    fn test_display_system() {
        assert_eq!(format!("{}", SYSTEM), "SYSTEM");
    }

    #[test]
    fn test_display_voting() {
        assert_eq!(format!("{}", VOTING), "VOTING");
    }

    #[test]
    fn test_display_owner() {
        assert_eq!(format!("{}", OWNER), "OWNER");
    }

    // -- Default --

    #[test]
    fn test_default_is_authentication() {
        assert_eq!(Purpose::default(), AUTHENTICATION);
    }

    // -- round-trip: u8 -> Purpose -> [u8; 1] --

    #[test]
    fn test_round_trip_all_valid_values() {
        for val in 0u8..=6 {
            let purpose = Purpose::try_from(val).unwrap();
            let arr: [u8; 1] = purpose.into();
            assert_eq!(arr[0], val);
        }
    }
}
