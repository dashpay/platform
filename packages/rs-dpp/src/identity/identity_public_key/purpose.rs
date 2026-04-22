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

    // -- TryFrom<u8> valid --
    #[test]
    fn test_purpose_try_from_u8_valid_all_variants() {
        assert_eq!(Purpose::try_from(0u8).unwrap(), AUTHENTICATION);
        assert_eq!(Purpose::try_from(1u8).unwrap(), ENCRYPTION);
        assert_eq!(Purpose::try_from(2u8).unwrap(), DECRYPTION);
        assert_eq!(Purpose::try_from(3u8).unwrap(), TRANSFER);
        assert_eq!(Purpose::try_from(4u8).unwrap(), SYSTEM);
        assert_eq!(Purpose::try_from(5u8).unwrap(), VOTING);
        assert_eq!(Purpose::try_from(6u8).unwrap(), OWNER);
    }

    // -- TryFrom<u8> invalid --
    #[test]
    fn test_purpose_try_from_u8_invalid() {
        assert!(Purpose::try_from(7u8).is_err());
        assert!(Purpose::try_from(255u8).is_err());
    }

    // -- TryFrom<i32> valid + invalid --
    #[test]
    fn test_purpose_try_from_i32_valid_all_variants() {
        assert_eq!(Purpose::try_from(0i32).unwrap(), AUTHENTICATION);
        assert_eq!(Purpose::try_from(1i32).unwrap(), ENCRYPTION);
        assert_eq!(Purpose::try_from(2i32).unwrap(), DECRYPTION);
        assert_eq!(Purpose::try_from(3i32).unwrap(), TRANSFER);
        assert_eq!(Purpose::try_from(4i32).unwrap(), SYSTEM);
        assert_eq!(Purpose::try_from(5i32).unwrap(), VOTING);
        assert_eq!(Purpose::try_from(6i32).unwrap(), OWNER);
    }

    #[test]
    fn test_purpose_try_from_i32_invalid() {
        assert!(Purpose::try_from(-1i32).is_err());
        assert!(Purpose::try_from(7i32).is_err());
        assert!(Purpose::try_from(1_000_000i32).is_err());
    }

    // -- From<Purpose> for [u8; 1] (by-value) --
    #[test]
    fn test_purpose_to_owned_byte_array() {
        let arr: [u8; 1] = AUTHENTICATION.into();
        assert_eq!(arr, [0]);
        let arr: [u8; 1] = OWNER.into();
        assert_eq!(arr, [6]);
        let arr: [u8; 1] = SYSTEM.into();
        assert_eq!(arr, [4]);
    }

    // -- From<Purpose> for &'static [u8; 1] --
    #[test]
    fn test_purpose_to_static_byte_ref_all_variants() {
        let r: &'static [u8; 1] = AUTHENTICATION.into();
        assert_eq!(r, &[0u8]);
        let r: &'static [u8; 1] = ENCRYPTION.into();
        assert_eq!(r, &[1u8]);
        let r: &'static [u8; 1] = DECRYPTION.into();
        assert_eq!(r, &[2u8]);
        let r: &'static [u8; 1] = TRANSFER.into();
        assert_eq!(r, &[3u8]);
        let r: &'static [u8; 1] = SYSTEM.into();
        assert_eq!(r, &[4u8]);
        let r: &'static [u8; 1] = VOTING.into();
        assert_eq!(r, &[5u8]);
        let r: &'static [u8; 1] = OWNER.into();
        assert_eq!(r, &[6u8]);
    }

    // -- Display (via Debug) --
    #[test]
    fn test_purpose_display_matches_debug_form() {
        assert_eq!(format!("{}", AUTHENTICATION), "AUTHENTICATION");
        assert_eq!(format!("{}", ENCRYPTION), "ENCRYPTION");
        assert_eq!(format!("{}", DECRYPTION), "DECRYPTION");
        assert_eq!(format!("{}", TRANSFER), "TRANSFER");
        assert_eq!(format!("{}", SYSTEM), "SYSTEM");
        assert_eq!(format!("{}", VOTING), "VOTING");
        assert_eq!(format!("{}", OWNER), "OWNER");
    }

    // -- Default --
    #[test]
    fn test_purpose_default_is_authentication() {
        assert_eq!(Purpose::default(), AUTHENTICATION);
    }

    // -- Range helpers --
    #[test]
    fn test_purpose_full_range_contents() {
        // NOTE: full_range() intentionally excludes SYSTEM.
        let full = Purpose::full_range();
        assert_eq!(full.len(), 6);
        assert!(full.contains(&AUTHENTICATION));
        assert!(full.contains(&ENCRYPTION));
        assert!(full.contains(&DECRYPTION));
        assert!(full.contains(&TRANSFER));
        assert!(full.contains(&VOTING));
        assert!(full.contains(&OWNER));
        assert!(!full.contains(&SYSTEM));
    }

    #[test]
    fn test_purpose_searchable_purposes_contents() {
        let searchable = Purpose::searchable_purposes();
        assert_eq!(searchable.len(), 3);
        assert_eq!(searchable, [AUTHENTICATION, TRANSFER, VOTING]);
    }

    #[test]
    fn test_purpose_encryption_decryption_contents() {
        let ed = Purpose::encryption_decryption();
        assert_eq!(ed.len(), 2);
        assert_eq!(ed, [ENCRYPTION, DECRYPTION]);
    }

    // -- round-trip: Purpose -> u8 -> Purpose --
    #[test]
    fn test_purpose_round_trip_u8() {
        for val in 0u8..=6 {
            let p = Purpose::try_from(val).unwrap();
            assert_eq!(p as u8, val);
        }
    }

    // -- ordering --
    #[test]
    fn test_purpose_ordering_matches_discriminant() {
        assert!(AUTHENTICATION < ENCRYPTION);
        assert!(ENCRYPTION < DECRYPTION);
        assert!(DECRYPTION < TRANSFER);
        assert!(TRANSFER < SYSTEM);
        assert!(SYSTEM < VOTING);
        assert!(VOTING < OWNER);
    }
}
