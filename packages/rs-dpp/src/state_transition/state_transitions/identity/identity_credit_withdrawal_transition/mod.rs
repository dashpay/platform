#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;

pub mod accessors;
pub mod fields;
mod identity_signed;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
pub mod v0;
pub mod v1;
mod version;

use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::balances::credits::CREDITS_PER_DUFF;
use crate::identity::state_transition::OptionallyAssetLockProved;
use crate::state_transition::identity_credit_withdrawal_transition::v1::{
    IdentityCreditWithdrawalTransitionV1, IdentityCreditWithdrawalTransitionV1Signable,
};
use crate::ProtocolError;
use bincode::{Decode, Encode};
use dashcore::transaction::special_transaction::asset_unlock::qualified_asset_unlock::ASSET_UNLOCK_TX_SIZE;
use derive_more::From;
use fields::*;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Minimal core per byte. Must be a fibonacci number
pub const MIN_CORE_FEE_PER_BYTE: u32 = 1;

/// Minimal amount in credits (x1000) to avoid "dust" error in Core.
///
/// NOTE: This is the protocol-v11-and-below floor (190 duffs). Consensus reads the
/// *versioned* `platform_version.system_limits.min_withdrawal_amount` (raised to 1000 duffs
/// in v12); keep `SYSTEM_LIMITS_V1.min_withdrawal_amount` in sync with this value.
pub const MIN_WITHDRAWAL_AMOUNT: u64 =
    (ASSET_UNLOCK_TX_SIZE as u64) * (MIN_CORE_FEE_PER_BYTE as u64) * CREDITS_PER_DUFF;

// Compile-time lock: if a dashcore `ASSET_UNLOCK_TX_SIZE` (or fee-rate) change moves this
// value, the build breaks here — a prompt to re-sync `SYSTEM_LIMITS_V1.min_withdrawal_amount`
// (the consensus source of truth) with the new figure.
const _: () = assert!(
    MIN_WITHDRAWAL_AMOUNT == 190_000,
    "MIN_WITHDRAWAL_AMOUNT changed; re-sync SYSTEM_LIMITS_V1.min_withdrawal_amount"
);

pub type IdentityCreditWithdrawalTransitionLatest = IdentityCreditWithdrawalTransitionV1;

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path(
    "dpp.state_transition_serialization_versions.identity_credit_withdrawal_state_transition"
)]
pub enum IdentityCreditWithdrawalTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreditWithdrawalTransitionV0),
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(IdentityCreditWithdrawalTransitionV1),
}

impl IdentityCreditWithdrawalTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .identities
            .credit_withdrawal
            .default_constructor
        {
            0 => Ok(IdentityCreditWithdrawalTransition::V0(
                IdentityCreditWithdrawalTransitionV0::default(),
            )),
            1 => Ok(IdentityCreditWithdrawalTransition::V1(
                IdentityCreditWithdrawalTransitionV1::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityCreditWithdrawalTransition::default_versioned".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl StateTransitionFieldTypes for IdentityCreditWithdrawalTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, OUTPUT_SCRIPT]
    }
}

impl OptionallyAssetLockProved for IdentityCreditWithdrawalTransition {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::identity::core_script::CoreScript;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
    use crate::state_transition::{
        StateTransitionEstimatedFeeValidation, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentityEstimatedFeeValidation, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::version::LATEST_PLATFORM_VERSION;
    use crate::withdrawal::Pooling;
    use platform_value::{BinaryData, Identifier};

    fn make_withdrawal_v0() -> IdentityCreditWithdrawalTransition {
        IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::random(),
            amount: 300_000,
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: 3,
            user_fee_increase: 1,
            signature_public_key_id: 1,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    fn make_withdrawal_v1() -> IdentityCreditWithdrawalTransition {
        IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
            identity_id: Identifier::random(),
            amount: 400_000,
            core_fee_per_byte: 2,
            pooling: Pooling::Standard,
            output_script: None,
            nonce: 5,
            user_fee_increase: 2,
            signature_public_key_id: 3,
            signature: [0u8; 65].to_vec().into(),
        })
    }

    #[test]
    fn test_default_versioned() {
        let t = IdentityCreditWithdrawalTransition::default_versioned(LATEST_PLATFORM_VERSION)
            .expect("should create default");
        match t {
            IdentityCreditWithdrawalTransition::V0(_)
            | IdentityCreditWithdrawalTransition::V1(_) => {}
        }
    }

    #[test]
    fn test_serialization_roundtrip_v0() {
        let t = make_withdrawal_v0();
        let bytes = t.serialize_to_bytes().expect("should serialize");
        let restored = IdentityCreditWithdrawalTransition::deserialize_from_bytes(&bytes)
            .expect("should deserialize");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_serialization_roundtrip_v1() {
        let t = make_withdrawal_v1();
        let bytes = t.serialize_to_bytes().expect("should serialize");
        let restored = IdentityCreditWithdrawalTransition::deserialize_from_bytes(&bytes)
            .expect("should deserialize");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_state_transition_like_v0() {
        let t = make_withdrawal_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreditWithdrawal
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn test_state_transition_like_v1() {
        let t = make_withdrawal_v1();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreditWithdrawal
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn test_owner_id() {
        let t = make_withdrawal_v0();
        assert_eq!(t.owner_id(), t.identity_id());
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_withdrawal_v0();
        assert_eq!(t.user_fee_increase(), 1);
        t.set_user_fee_increase(50);
        assert_eq!(t.user_fee_increase(), 50);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_withdrawal_v0();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2]));
        assert_eq!(t.signature().as_slice(), &[1, 2]);
        t.set_signature_bytes(vec![3, 4]);
        assert_eq!(t.signature().as_slice(), &[3, 4]);
    }

    #[test]
    fn test_accessors() {
        let mut t = make_withdrawal_v0();
        assert_eq!(t.amount(), 300_000);
        t.set_amount(500_000);
        assert_eq!(t.amount(), 500_000);
        assert_eq!(t.nonce(), 3);
        t.set_nonce(99);
        assert_eq!(t.nonce(), 99);
        assert_eq!(t.pooling(), Pooling::Never);
        t.set_pooling(Pooling::Standard);
        assert_eq!(t.pooling(), Pooling::Standard);
        assert_eq!(t.core_fee_per_byte(), 1);
        t.set_core_fee_per_byte(5);
        assert_eq!(t.core_fee_per_byte(), 5);
    }

    #[test]
    fn test_accessors_v1_output_script_none() {
        let t = make_withdrawal_v1();
        assert!(t.output_script().is_none());
    }

    #[test]
    fn test_field_types() {
        let sig = IdentityCreditWithdrawalTransition::signature_property_paths();
        assert_eq!(sig.len(), 2);
        let ids = IdentityCreditWithdrawalTransition::identifiers_property_paths();
        assert_eq!(ids.len(), 1);
        let bin = IdentityCreditWithdrawalTransition::binary_property_paths();
        assert_eq!(bin.len(), 2);
    }

    // Legacy `StateTransitionValueConvert` round-trip and
    // unknown-version tests deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised on
    // the outer enum derive (see `json_convertible_tests` below) — these
    // tested methods that no longer exist.

    #[test]
    fn test_estimated_fee_sufficient() {
        let t = make_withdrawal_v0();
        let fee = t
            .calculate_min_required_fee(LATEST_PLATFORM_VERSION)
            .expect("fee calc should work");
        assert!(fee > 0);
        let result = t
            .validate_estimated_fee(fee + t.amount() + 1000, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(result.is_valid());
    }

    #[test]
    fn test_estimated_fee_insufficient() {
        let t = make_withdrawal_v0();
        let result = t
            .validate_estimated_fee(0, LATEST_PLATFORM_VERSION)
            .expect("validation should succeed");
        assert!(!result.is_valid());
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_min_withdrawal_amount_constant() {
        assert!(MIN_WITHDRAWAL_AMOUNT > 0);
        assert!(MIN_CORE_FEE_PER_BYTE == 1);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;

    use crate::identity::core_script::CoreScript;
    use crate::withdrawal::Pooling;
    use platform_value::{platform_value, BinaryData, Identifier};
    use serde_json::json;

    pub(crate) fn fixture() -> IdentityCreditWithdrawalTransition {
        IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
            identity_id: Identifier::new([0x33; 32]),
            amount: 9_876_543,
            core_fee_per_byte: 5,
            pooling: Pooling::Never,
            output_script: CoreScript::from_bytes(vec![0x76, 0xa9, 0x14]),
            nonce: 11,
            user_fee_increase: 2,
            signature_public_key_id: 4,
            signature: BinaryData::new(vec![0xb2; 65]),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `coreFeePerByte` (u32), `nonce` (u64), `userFeeIncrease` (u16),
        // `signaturePublicKeyId` (u32). The value-path assertion below uses
        // explicit suffixes to lock in the typed variants.
        // `pooling` uses a custom `pooling_serde` that emits the camelCase name
        // string in HR and the u8 discriminant in non-HR.
        // `outputScript` is base64 in HR (CoreScript Serialize) and bytes in non-HR.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x33; 32]),
                "amount": 9_876_543u64,
                "coreFeePerByte": 5u32,
                "pooling": "never",
                "outputScript": "dqkU",
                "nonce": 11u64,
                "userFeeIncrease": 2u16,
                "signaturePublicKeyId": 4u32,
                "signature": "srKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrKysrI=",
            })
        );
        let recovered = IdentityCreditWithdrawalTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `amount` u64, `coreFeePerByte`
        // u32, `nonce` u64 (IdentityNonce), `userFeeIncrease` u16,
        // `signaturePublicKeyId` u32 (KeyID).
        // `pooling`: `pooling_serde` emits the u8 discriminant on the non-HR
        // path (`Pooling::Never as u8 == 0`).
        // `outputScript`: CoreScript serializes as Bytes on the non-HR path.
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "identityId": Identifier::new([0x33; 32]),
                "amount": 9_876_543u64,
                "coreFeePerByte": 5u32,
                "pooling": 0u8,
                "outputScript": BinaryData::new(vec![0x76, 0xa9, 0x14]),
                "nonce": 11u64,
                "userFeeIncrease": 2u16,
                "signaturePublicKeyId": 4u32,
                "signature": BinaryData::new(vec![0xb2; 65]),
            })
        );
        let recovered =
            IdentityCreditWithdrawalTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    // --- V1 with output_script: None (restored coverage from Phase D step 9) ---
    //
    // V1 has `output_script: Option<CoreScript>` where None means "send to the
    // address set by core" (vs V0's mandatory script). These tests round-trip
    // the None case through both JSON and Value paths, replacing the
    // `make_withdrawal_v1_no_script` fixture deleted in step 9.
    pub(crate) fn fixture_v1_no_script() -> IdentityCreditWithdrawalTransition {
        IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
            identity_id: Identifier::new([0x44; 32]),
            amount: 1_234_567,
            core_fee_per_byte: 7,
            pooling: Pooling::Standard,
            output_script: None,
            nonce: 13,
            user_fee_increase: 3,
            signature_public_key_id: 5,
            signature: BinaryData::new(vec![0xa3; 65]),
        })
    }

    #[test]
    fn json_round_trip_v1_with_none_output_script() {
        use crate::serialization::JsonConvertible;
        let original = fixture_v1_no_script();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "1",
                "identityId": Identifier::new([0x44; 32]),
                "amount": 1_234_567u64,
                "coreFeePerByte": 7u32,
                "pooling": "standard",
                "outputScript": null,
                "nonce": 13u64,
                "userFeeIncrease": 3u16,
                "signaturePublicKeyId": 5u32,
                "signature": "o6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6Ojo6M=",
            })
        );
        let recovered = IdentityCreditWithdrawalTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_v1_with_none_output_script() {
        use crate::serialization::ValueConvertible;
        let original = fixture_v1_no_script();
        let value = original.to_object().expect("to_object");
        // Pooling::Standard = 2 (discriminant; Never=0, IfAvailable=1, Standard=2)
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "1",
                "identityId": Identifier::new([0x44; 32]),
                "amount": 1_234_567u64,
                "coreFeePerByte": 7u32,
                "pooling": 2u8,
                "outputScript": platform_value::Value::Null,
                "nonce": 13u64,
                "userFeeIncrease": 3u16,
                "signaturePublicKeyId": 5u32,
                "signature": BinaryData::new(vec![0xa3; 65]),
            })
        );
        let recovered =
            IdentityCreditWithdrawalTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    // --- Unknown `$formatVersion` error coverage ---
    //
    // Representative test for the canonical `serde(tag = "$formatVersion")`
    // dispatch error path. We assert that an unknown version (`"99"`) is
    // rejected by both `from_json` and `from_object` paths. This pattern
    // is unified across every `$formatVersion`-tagged outer enum in the
    // codebase, so one representative test on a multi-variant enum
    // (V0 + V1) demonstrates the contract; per-enum coverage would be
    // mechanical noise.
    #[test]
    fn from_json_rejects_unknown_format_version() {
        use crate::serialization::JsonConvertible;
        let bad = json!({
            "$formatVersion": "99",
            "identityId": Identifier::new([0x33; 32]),
            "amount": 1u64,
            "coreFeePerByte": 1u32,
            "pooling": "never",
            "outputScript": "AA==",
            "nonce": 1u64,
            "userFeeIncrease": 1u16,
            "signaturePublicKeyId": 1u32,
            "signature": "AA==",
        });
        let result = IdentityCreditWithdrawalTransition::from_json(bad);
        assert!(
            result.is_err(),
            "from_json should reject an unknown $formatVersion (got: {:?})",
            result.as_ref().map(|_| "Ok")
        );
    }

    #[test]
    fn from_object_rejects_unknown_format_version() {
        use crate::serialization::ValueConvertible;
        let bad = platform_value!({
            "$formatVersion": "99",
            "identityId": Identifier::new([0x33; 32]),
            "amount": 1u64,
            "coreFeePerByte": 1u32,
            "pooling": 0u8,
            "outputScript": BinaryData::new(vec![]),
            "nonce": 1u64,
            "userFeeIncrease": 1u16,
            "signaturePublicKeyId": 1u32,
            "signature": BinaryData::new(vec![]),
        });
        let result = IdentityCreditWithdrawalTransition::from_object(bad);
        assert!(
            result.is_err(),
            "from_object should reject an unknown $formatVersion (got: {:?})",
            result.as_ref().map(|_| "Ok")
        );
    }
}
