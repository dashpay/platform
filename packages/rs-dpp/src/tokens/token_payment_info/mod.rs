//! Token payment metadata and helpers.
//!
//! This module defines the versioned `TokenPaymentInfo` wrapper used to describe how a
//! client intends to pay with tokens for an operation (for example, creating,
//! transferring, purchasing, or updating the price of a document/NFT).
//! It captures which token to use, optional price bounds, and who covers gas fees.
//!
//! The enum is versioned to allow future evolution without breaking callers.
//! [`v0::TokenPaymentInfoV0`] pays from the identity's token balance; [`v1::TokenPaymentInfoV1`]
//! adds a [`v1::TokenShieldedPayment`], a spend bundle that pays the cost out of the token's
//! shielded pool instead (protocol version 15 and up). Accessors are provided via
//! [`v0::v0_accessors::TokenPaymentInfoAccessorsV0`], and convenience methods (such
//! as `token_id()` and `is_valid_for_required_cost()`) are available through
//! [`methods::v0::TokenPaymentInfoMethodsV0`].
//!
//! Typical usage:
//!
//! ```ignore
//! use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
//! use dpp::data_contract::TokenContractPosition;
//! use dpp::tokens::token_payment_info::{TokenPaymentInfo, v0::TokenPaymentInfoV0};
//!
//! // Client indicates payment preferences for a transition
//! let info: TokenPaymentInfo = TokenPaymentInfoV0 {
//!     // `None` => use a token defined on the current contract
//!     payment_token_contract_id: None,
//!     // Which token (by position/index) on the contract to use
//!     token_contract_position: 0u16,
//!     // Optional bounds to guard against unexpected price changes
//!     minimum_token_cost: None,
//!     maximum_token_cost: Some(1_000u64.into()),
//!     // Who pays gas: user, contract owner, or prefer contract owner
//!     gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
//! }.into();
//! ```
//!
//! Deserialization from a platform `BTreeMap<String, Value>` requires a
//! `$formatVersion` key. For V0 the map may contain:
//! - `paymentTokenContractId` (`Identifier` as bytes)
//! - `tokenContractPosition` (`u16`)
//! - `minimumTokenCost` (`u64`)
//! - `maximumTokenCost` (`u64`)
//! - `gasFeesPaidBy` (one of: `"DocumentOwner"`, `"ContractOwner"`, `"PreferContractOwner"`)
//!
//! For V1 the map additionally carries `shieldedPayment` (`amount`, `actions`, `anchor`,
//! `proof`, `bindingSignature`).
//!
//! Unknown `$formatVersion` values yield an `UnknownVersionMismatch` error.
//!
use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::methods::v0::TokenPaymentInfoMethodsV0;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use crate::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use crate::tokens::token_payment_info::v1::v1_accessors::TokenPaymentInfoAccessorsV1;
use crate::tokens::token_payment_info::v1::{TokenPaymentInfoV1, TokenShieldedPayment};
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
#[cfg(feature = "value-conversion")]
use platform_value::Error;
use platform_value::{Identifier, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod methods;
pub mod v0;
pub mod v1;

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSerialize,
    PartialEq,
    Display,
    From,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
/// Versioned container describing how a client intends to pay with tokens.
///
/// The `TokenPaymentInfo` enum allows the protocol to evolve the underlying structure
/// across versions while keeping a stable API for callers. Use the accessor trait
/// [`v0::v0_accessors::TokenPaymentInfoAccessorsV0`] to read or update fields, and
/// [`methods::v0::TokenPaymentInfoMethodsV0`] for helpers like `token_id()` and
/// `is_valid_for_required_cost()`.
///
/// See [`v0::TokenPaymentInfoV0`] for the current set of fields and semantics.
pub enum TokenPaymentInfo {
    #[display("V0({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(TokenPaymentInfoV0),
    /// `V0` plus a shielded payment: the token cost is paid out of the token's shielded pool.
    #[display("V1({})", "_0")]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
    V1(TokenPaymentInfoV1),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenPaymentInfo {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenPaymentInfo {}

impl TokenPaymentInfoMethodsV0 for TokenPaymentInfo {}

impl TokenPaymentInfoAccessorsV1 for TokenPaymentInfo {
    fn shielded_payment(&self) -> Option<&TokenShieldedPayment> {
        match self {
            TokenPaymentInfo::V0(_) => None,
            TokenPaymentInfo::V1(v1) => Some(v1.shielded_payment()),
        }
    }

    fn set_shielded_payment(&mut self, shielded_payment: Option<TokenShieldedPayment>) {
        let current = std::mem::replace(self, TokenPaymentInfo::V0(TokenPaymentInfoV0::default()));
        *self = match (current, shielded_payment) {
            (TokenPaymentInfo::V0(v0), Some(payment)) => {
                TokenPaymentInfo::V1(TokenPaymentInfoV1::from_v0(v0, payment))
            }
            (TokenPaymentInfo::V1(v1), Some(payment)) => TokenPaymentInfo::V1(TokenPaymentInfoV1 {
                shielded_payment: payment,
                ..v1
            }),
            (TokenPaymentInfo::V1(v1), None) => TokenPaymentInfo::V0(v1.into_v0()),
            (v0 @ TokenPaymentInfo::V0(_), None) => v0,
        };
    }
}

impl TokenPaymentInfoAccessorsV0 for TokenPaymentInfo {
    // Getters
    fn payment_token_contract_id(&self) -> Option<Identifier> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.payment_token_contract_id(),
            TokenPaymentInfo::V1(v1) => v1.payment_token_contract_id(),
        }
    }

    fn payment_token_contract_id_ref(&self) -> &Option<Identifier> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.payment_token_contract_id_ref(),
            TokenPaymentInfo::V1(v1) => v1.payment_token_contract_id_ref(),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            TokenPaymentInfo::V0(v0) => v0.token_contract_position(),
            TokenPaymentInfo::V1(v1) => v1.token_contract_position(),
        }
    }

    fn minimum_token_cost(&self) -> Option<TokenAmount> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.minimum_token_cost(),
            TokenPaymentInfo::V1(v1) => v1.minimum_token_cost(),
        }
    }

    fn maximum_token_cost(&self) -> Option<TokenAmount> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.maximum_token_cost(),
            TokenPaymentInfo::V1(v1) => v1.maximum_token_cost(),
        }
    }

    fn gas_fees_paid_by(&self) -> GasFeesPaidBy {
        match self {
            TokenPaymentInfo::V0(v0) => v0.gas_fees_paid_by(),
            TokenPaymentInfo::V1(v1) => v1.gas_fees_paid_by(),
        }
    }

    // Setters
    fn set_payment_token_contract_id(&mut self, id: Option<Identifier>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_payment_token_contract_id(id),
            TokenPaymentInfo::V1(v1) => v1.set_payment_token_contract_id(id),
        }
    }

    fn set_token_contract_position(&mut self, position: TokenContractPosition) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_token_contract_position(position),
            TokenPaymentInfo::V1(v1) => v1.set_token_contract_position(position),
        }
    }

    fn set_minimum_token_cost(&mut self, cost: Option<TokenAmount>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_minimum_token_cost(cost),
            TokenPaymentInfo::V1(v1) => v1.set_minimum_token_cost(cost),
        }
    }

    fn set_maximum_token_cost(&mut self, cost: Option<TokenAmount>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_maximum_token_cost(cost),
            TokenPaymentInfo::V1(v1) => v1.set_maximum_token_cost(cost),
        }
    }

    fn set_gas_fees_paid_by(&mut self, payer: GasFeesPaidBy) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_gas_fees_paid_by(payer),
            TokenPaymentInfo::V1(v1) => v1.set_gas_fees_paid_by(payer),
        }
    }
}

impl TryFrom<BTreeMap<String, Value>> for TokenPaymentInfo {
    type Error = ProtocolError;

    fn try_from(map: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        // Expect a `$formatVersion` discriminator and dispatch to the
        // corresponding versioned structure. This allows backward-compatible
        // support for older serialized payloads.
        let format_version = map.get_str("$formatVersion")?;
        match format_version {
            "0" => {
                let token_payment_info: TokenPaymentInfoV0 = map.try_into()?;

                Ok(token_payment_info.into())
            }
            "1" => {
                let token_payment_info: TokenPaymentInfoV1 = map.try_into()?;

                Ok(token_payment_info.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenPaymentInfo::from_value".to_string(),
                known_versions: vec![0, 1],
                received: version
                    .parse()
                    .map_err(|_| ProtocolError::Generic("Conversion error".to_string()))?,
            }),
        }
    }
}

#[cfg(feature = "value-conversion")]
impl TryFrom<TokenPaymentInfo> for Value {
    type Error = Error;
    /// Serialize the versioned token payment info into a platform `Value`.
    ///
    /// This mirrors the map format accepted by `TryFrom<BTreeMap<String, Value>>`,
    /// including the `$formatVersion` discriminator.
    fn try_from(value: TokenPaymentInfo) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> TokenPaymentInfo {
        TokenPaymentInfo::V0(TokenPaymentInfoV0 {
            payment_token_contract_id: Some(Identifier::new([0x99; 32])),
            token_contract_position: 3,
            minimum_token_cost: Some(100),
            maximum_token_cost: Some(1_000),
            gas_fees_paid_by: GasFeesPaidBy::ContractOwner,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Internally-tagged enum (`tag = "$formatVersion"`); inner V0 has
        // `rename_all = "camelCase"`. `Identifier` -> base58 in JSON.
        // `token_contract_position` is `TokenContractPosition` (= u16) and
        // `minimum_token_cost` / `maximum_token_cost` are `TokenAmount` (= u64);
        // JSON erases the size — see the value-path assertion for typed locks.
        // `gas_fees_paid_by` is the unit enum `GasFeesPaidBy` and serializes
        // as `"ContractOwner"` (no `rename_all`).
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "paymentTokenContractId": "BLbDu5FZUdSfLrGejhuaWw5iMJBo3j3TVRyPv9rfJyMA",
                "tokenContractPosition": 3,
                "minimumTokenCost": 100,
                "maximumTokenCost": 1_000,
                "gasFeesPaidBy": "ContractOwner",
            })
        );
        let recovered = TokenPaymentInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    fn v1_fixture() -> TokenPaymentInfo {
        use crate::shielded::SerializedAction;
        use crate::tokens::token_payment_info::v1::{TokenPaymentInfoV1, TokenShieldedPayment};
        TokenPaymentInfo::V1(TokenPaymentInfoV1 {
            payment_token_contract_id: None,
            token_contract_position: 1,
            minimum_token_cost: None,
            maximum_token_cost: Some(10),
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            shielded_payment: TokenShieldedPayment {
                amount: 10,
                actions: vec![SerializedAction {
                    nullifier: [1u8; 32],
                    rk: [2u8; 32],
                    cmx: [3u8; 32],
                    encrypted_note: vec![4u8; 216],
                    cv_net: [5u8; 32],
                    spend_auth_sig: [6u8; 64],
                }],
                anchor: [7u8; 32],
                proof: vec![8u8; 10],
                binding_signature: [9u8; 64],
            },
        })
    }

    #[test]
    fn v1_json_and_value_round_trip() {
        use crate::serialization::{JsonConvertible, ValueConvertible};
        let original = v1_fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "1");
        assert_eq!(json["shieldedPayment"]["amount"], 10);
        let recovered = TokenPaymentInfo::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        let value = original.to_object().expect("to_object");
        let recovered = TokenPaymentInfo::from_object(value.clone()).expect("from_object");
        assert_eq!(original, recovered);
        // the manual map path (`$tokenPaymentInfo` inside a document map) parses V1 too
        let map = value.into_btree_string_map().expect("map");
        let recovered: TokenPaymentInfo = map.try_into().expect("try_into");
        assert_eq!(original, recovered);
    }

    #[test]
    fn setting_and_clearing_the_shielded_payment_switches_the_format_version() {
        let mut info = fixture();
        let TokenPaymentInfo::V1(v1) = v1_fixture() else {
            unreachable!()
        };
        info.set_shielded_payment(Some(v1.shielded_payment.clone()));
        assert!(matches!(info, TokenPaymentInfo::V1(_)));
        assert_eq!(info.shielded_payment(), Some(&v1.shielded_payment));
        assert_eq!(info.maximum_token_cost(), Some(1_000));
        info.set_shielded_payment(None);
        assert_eq!(info, fixture());
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `Identifier` flows as `Value::Identifier` when interpolated.
        // `3u16` locks `Value::U16`; `100u64` / `1_000u64` lock `Value::U64`.
        let payment_token_contract_id = Identifier::new([0x99; 32]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "paymentTokenContractId": payment_token_contract_id,
                "tokenContractPosition": 3u16,
                "minimumTokenCost": 100u64,
                "maximumTokenCost": 1_000u64,
                "gasFeesPaidBy": "ContractOwner",
            })
        );
        let recovered = TokenPaymentInfo::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
