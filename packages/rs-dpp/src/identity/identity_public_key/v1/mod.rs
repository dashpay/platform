mod accessors;
mod methods;

use bincode::{Decode, DecodeUntrusted, Encode};

use crate::fee::Credits;
use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::{KeyID, KeyType, Purpose, SecurityLevel, TimestampMillis};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "state-transitions")]
use crate::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
use platform_value::BinaryData;
use serde::{Deserialize, Serialize};

/// An identity public key that may carry usage limits.
///
/// The first eight fields are the `IdentityPublicKeyV0` fields in the same order. `budget` and
/// `expires_at` exist from protocol version 14 and are only allowed on AUTHENTICATION keys below
/// the MASTER security level. A key without limits keeps being written as V0.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Default,
    Debug,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    Ord,
    PartialOrd,
    Hash,
    DecodeUntrusted,
)]
#[serde(rename_all = "camelCase")]
pub struct IdentityPublicKeyV1 {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    pub contract_bounds: Option<ContractBounds>,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub read_only: bool,
    pub data: BinaryData,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at: Option<TimestampMillis>,
    /// The total credits that state transitions signed with this key may take from the
    /// identity. How much of it is left lives in Drive, not in the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Credits>,
    /// The block time, in milliseconds, from which the key can no longer sign.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<TimestampMillis>,
}

impl IdentityPublicKeyV1 {
    /// Adds usage limits to a V0 key.
    pub fn from_v0_with_limits(
        key: IdentityPublicKeyV0,
        budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
    ) -> Self {
        let IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds,
            key_type,
            read_only,
            data,
            disabled_at,
        } = key;
        IdentityPublicKeyV1 {
            id,
            purpose,
            security_level,
            contract_bounds,
            key_type,
            read_only,
            data,
            disabled_at,
            budget,
            expires_at,
        }
    }
}

#[cfg(feature = "state-transitions")]
impl From<&IdentityPublicKeyV1> for IdentityPublicKeyInCreationV1 {
    fn from(key: &IdentityPublicKeyV1) -> Self {
        IdentityPublicKeyInCreationV1 {
            id: key.id,
            key_type: key.key_type,
            purpose: key.purpose,
            security_level: key.security_level,
            contract_bounds: key.contract_bounds.clone(),
            read_only: key.read_only,
            data: key.data.clone(),
            budget: key.budget,
            expires_at: key.expires_at,
            signature: BinaryData::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use crate::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
    use crate::identity::IdentityPublicKey;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};

    fn key_v0() -> IdentityPublicKeyV0 {
        IdentityPublicKeyV0 {
            id: 7,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![3; 33]),
            disabled_at: None,
        }
    }

    #[test]
    fn should_round_trip_a_limited_key_through_platform_serialization() {
        let key: IdentityPublicKey =
            IdentityPublicKeyV1::from_v0_with_limits(key_v0(), Some(5_000_000), Some(1_800_000))
                .into();
        let bytes = key.serialize_to_bytes().expect("expected to serialize");
        let decoded = IdentityPublicKey::deserialize_from_bytes_untrusted(&bytes)
            .expect("expected to deserialize");
        assert_eq!(decoded, key);
        assert_eq!(decoded.budget(), Some(5_000_000));
        assert_eq!(decoded.expires_at(), Some(1_800_000));
    }

    #[test]
    fn should_leave_the_version_0_encoding_untouched() {
        // Every key already in state is a version 0 key, so its bytes must keep decoding to the
        // same key, and a key without limits must keep encoding to the same bytes.
        let key: IdentityPublicKey = key_v0().into();
        let bytes = key.serialize_to_bytes().expect("expected to serialize");
        assert_eq!(bytes[0], 0, "the variant index of a version 0 key");
        let decoded = IdentityPublicKey::deserialize_from_bytes_untrusted(&bytes)
            .expect("expected to deserialize");
        assert!(matches!(decoded, IdentityPublicKey::V0(_)));
        assert!(!decoded.has_limits());

        // Version 1 is the version 0 body followed by the two limits.
        let limited: IdentityPublicKey =
            IdentityPublicKeyV1::from_v0_with_limits(key_v0(), None, None).into();
        let limited_bytes = limited.serialize_to_bytes().expect("expected to serialize");
        assert_eq!(limited_bytes[0], 1);
        assert_eq!(limited_bytes[1..bytes.len()], bytes[1..]);
        assert_eq!(limited_bytes[bytes.len()..], [0, 0]);
    }

    #[test]
    fn should_upgrade_a_version_0_key_when_limits_are_added() {
        let key: IdentityPublicKey = key_v0().into();
        let limited = key.clone().with_limits(Some(10), None);
        assert!(matches!(limited, IdentityPublicKey::V1(_)));
        assert_eq!(limited.id(), key.id());
        assert_eq!(limited.data(), key.data());
        assert_eq!(limited.budget(), Some(10));
        assert_eq!(limited.expires_at(), None);

        let relimited = limited.with_limits(None, Some(20));
        assert_eq!(relimited.budget(), None);
        assert_eq!(relimited.expires_at(), Some(20));
    }

    #[test]
    fn should_be_expired_from_the_expiry_instant_on() {
        let key = IdentityPublicKeyV1::from_v0_with_limits(key_v0(), None, Some(1_000));
        assert!(!key.is_expired_at(999));
        assert!(key.is_expired_at(1_000));
        assert!(key.is_expired_at(1_001));

        let never = IdentityPublicKeyV1::from_v0_with_limits(key_v0(), Some(1), None);
        assert!(!never.is_expired_at(u64::MAX));
    }

    #[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
    #[test]
    fn should_tag_the_json_shape_with_format_version_1() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;

        let key: IdentityPublicKey =
            IdentityPublicKeyV1::from_v0_with_limits(key_v0(), Some(5_000), Some(1_800_000)).into();
        let json = key.to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], json!("1"));
        assert_eq!(json["budget"], json!(5_000));
        assert_eq!(json["expiresAt"], json!(1_800_000));
        assert_eq!(IdentityPublicKey::from_json(json).expect("from_json"), key);

        // The limits are left out of the JSON shape when absent, like `disabledAt`.
        let unlimited: IdentityPublicKey =
            IdentityPublicKeyV1::from_v0_with_limits(key_v0(), None, None).into();
        let json = unlimited.to_json().expect("to_json");
        assert!(json.get("budget").is_none());
        assert!(json.get("expiresAt").is_none());
        assert_eq!(
            IdentityPublicKey::from_json(json).expect("from_json"),
            unlimited
        );
    }
}
