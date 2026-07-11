#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;

pub mod accessors;
pub mod fields;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_fee_strategy;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

use crate::balances::credits::CREDITS_PER_DUFF;
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
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path(
    "dpp.state_transition_serialization_versions.address_credit_withdrawal_state_transition"
)]
pub enum AddressCreditWithdrawalTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(AddressCreditWithdrawalTransitionV0),
}

impl AddressCreditWithdrawalTransition {
    pub fn default_versioned(platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .address_funds
            .credit_withdrawal
        {
            0 => Ok(AddressCreditWithdrawalTransition::V0(
                AddressCreditWithdrawalTransitionV0::default(),
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "AddressCreditWithdrawalTransition::default_versioned".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl JsonConvertible for AddressCreditWithdrawalTransition {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl ValueConvertible for AddressCreditWithdrawalTransition {}

impl StateTransitionFieldTypes for AddressCreditWithdrawalTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![OUTPUT_SCRIPT]
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
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::identity::core_script::CoreScript;
    use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use crate::withdrawal::Pooling;
    use platform_value::{platform_value, BinaryData, Value};
    use serde_json::json;
    use std::collections::BTreeMap;

    pub(crate) fn fixture() -> AddressCreditWithdrawalTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x01; 20]), (5u32, 900_000u64));

        let v0 = AddressCreditWithdrawalTransitionV0 {
            inputs,
            output: Some((PlatformAddress::P2sh([0x02; 20]), 100_000u64)),
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 21,
            pooling: Pooling::IfAvailable,
            output_script: CoreScript::from_bytes(vec![0xaa, 0xbb, 0xcc]),
            user_fee_increase: 19,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: BinaryData::new(vec![0xef; 65]),
            }],
        };
        AddressCreditWithdrawalTransition::V0(v0)
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields lose their size on the JSON wire (single number type):
        //   - `inputs[].nonce` is u32, `inputs[].amount` / `output.amount` are u64,
        //   - `feeStrategy[].index` is u16,
        //   - `coreFeePerByte` is u32, `userFeeIncrease` is u16.
        // The Value-path assertion below locks the typed variants.
        // `pooling` is encoded as the camelCase string `"ifAvailable"` in JSON
        // (HR path of the custom `pooling_serde`), but as `Value::U8(1)` in
        // non-HR. `outputScript` (CoreScript) is base64 in JSON, raw bytes in
        // Value. `BinaryData` (witness signature, address bytes) is base64 in
        // JSON, `Value::Bytes` in Value. `PlatformAddress` is hex string in
        // JSON, raw bytes in Value.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "inputs": [
                    {
                        "address": "000101010101010101010101010101010101010101",
                        "nonce": 5,
                        "amount": 900_000,
                    },
                ],
                "output": {
                    "address": "010202020202020202020202020202020202020202",
                    "amount": 100_000,
                },
                "feeStrategy": [
                    {"$type": "deductFromInput", "index": 0},
                ],
                "coreFeePerByte": 21,
                "pooling": "ifAvailable",
                "outputScript": "qrvM",
                "userFeeIncrease": 19,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": "7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7+8=",
                    },
                ],
            })
        );
        let recovered = AddressCreditWithdrawalTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // PlatformAddress emits raw bytes (21-byte: 1 type byte + 20 hash) in
        // non-HR. Pooling::IfAvailable is `Value::U8(1)`. CoreScript and
        // BinaryData both serialize as `Value::Bytes`. Sized integers stay
        // sized: nonces are U32, credit amounts U64, fee_strategy index U16,
        // core_fee_per_byte U32, user_fee_increase U16, pooling U8.
        let mut input_addr_bytes = vec![0x00u8];
        input_addr_bytes.extend_from_slice(&[0x01u8; 20]);
        let mut output_addr_bytes = vec![0x01u8];
        output_addr_bytes.extend_from_slice(&[0x02u8; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "inputs": [
                    {
                        "address": Value::Bytes(input_addr_bytes),
                        "nonce": 5u32,
                        "amount": 900_000u64,
                    },
                ],
                "output": {
                    "address": Value::Bytes(output_addr_bytes),
                    "amount": 100_000u64,
                },
                "feeStrategy": [
                    {"$type": "deductFromInput", "index": 0u16},
                ],
                "coreFeePerByte": 21u32,
                "pooling": 1u8,
                "outputScript": Value::Bytes(vec![0xaa, 0xbb, 0xcc]),
                "userFeeIncrease": 19u16,
                "inputWitnesses": [
                    {
                        "$type": "p2pkh",
                        "signature": Value::Bytes(vec![0xef; 65]),
                    },
                ],
            })
        );
        let recovered = AddressCreditWithdrawalTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
