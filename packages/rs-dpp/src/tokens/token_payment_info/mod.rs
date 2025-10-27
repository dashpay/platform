//! Token payment metadata and helpers.
//!
//! This module defines the versioned `TokenPaymentInfo` wrapper used to describe how a
//! client intends to pay with tokens for an operation (for example, creating,
//! transferring, purchasing, or updating the price of a document/NFT).
//! It captures which token to use, optional price bounds, and who covers gas fees.
//!
//! The enum is versioned to allow future evolution without breaking callers. The
//! current implementation is [`v0::TokenPaymentInfoV0`]. Accessors are provided via
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
//! `$format_version` key. For V0 the map may contain:
//! - `paymentTokenContractId` (`Identifier` as bytes)
//! - `tokenContractPosition` (`u16`)
//! - `minimumTokenCost` (`u64`)
//! - `maximumTokenCost` (`u64`)
//! - `gasFeesPaidBy` (one of: `"DocumentOwner"`, `"ContractOwner"`, `"PreferContractOwner"`)
//!
//! Unknown `$format_version` values yield an `UnknownVersionMismatch` error.
//!
use crate::balances::credits::TokenAmount;
use crate::data_contract::TokenContractPosition;
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::methods::v0::TokenPaymentInfoMethodsV0;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use crate::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use crate::ProtocolError;
use bincode_derive::{Decode, Encode};
use derive_more::{Display, From};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
#[cfg(feature = "state-transition-value-conversion")]
use platform_value::Error;
use platform_value::{Identifier, Value};
#[cfg(any(
    feature = "state-transition-serde-conversion",
    all(
        feature = "document-serde-conversion",
        feature = "data-contract-serde-conversion"
    ),
))]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod methods;
pub mod v0;

#[derive(
    Debug,
    Clone,
    Copy,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PartialEq,
    Display,
    From,
)]
#[cfg_attr(
    any(
        feature = "state-transition-serde-conversion",
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
    ),
    derive(Serialize, Deserialize)
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
    V0(TokenPaymentInfoV0),
}

impl TokenPaymentInfoMethodsV0 for TokenPaymentInfo {}

impl TokenPaymentInfoAccessorsV0 for TokenPaymentInfo {
    // Getters
    fn payment_token_contract_id(&self) -> Option<Identifier> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.payment_token_contract_id(),
        }
    }

    fn payment_token_contract_id_ref(&self) -> &Option<Identifier> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.payment_token_contract_id_ref(),
        }
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        match self {
            TokenPaymentInfo::V0(v0) => v0.token_contract_position(),
        }
    }

    fn minimum_token_cost(&self) -> Option<TokenAmount> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.minimum_token_cost(),
        }
    }

    fn maximum_token_cost(&self) -> Option<TokenAmount> {
        match self {
            TokenPaymentInfo::V0(v0) => v0.maximum_token_cost(),
        }
    }

    fn gas_fees_paid_by(&self) -> GasFeesPaidBy {
        match self {
            TokenPaymentInfo::V0(v0) => v0.gas_fees_paid_by(),
        }
    }

    // Setters
    fn set_payment_token_contract_id(&mut self, id: Option<Identifier>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_payment_token_contract_id(id),
        }
    }

    fn set_token_contract_position(&mut self, position: TokenContractPosition) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_token_contract_position(position),
        }
    }

    fn set_minimum_token_cost(&mut self, cost: Option<TokenAmount>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_minimum_token_cost(cost),
        }
    }

    fn set_maximum_token_cost(&mut self, cost: Option<TokenAmount>) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_maximum_token_cost(cost),
        }
    }

    fn set_gas_fees_paid_by(&mut self, payer: GasFeesPaidBy) {
        match self {
            TokenPaymentInfo::V0(v0) => v0.set_gas_fees_paid_by(payer),
        }
    }
}

impl TryFrom<BTreeMap<String, Value>> for TokenPaymentInfo {
    type Error = ProtocolError;

    fn try_from(map: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        // Expect a `$format_version` discriminator and dispatch to the
        // corresponding versioned structure. This allows backward-compatible
        // support for older serialized payloads.
        let format_version = map.get_str("$format_version")?;
        match format_version {
            "0" => {
                let token_payment_info: TokenPaymentInfoV0 = map.try_into()?;

                Ok(token_payment_info.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenPaymentInfo::from_value".to_string(),
                known_versions: vec![0],
                received: version
                    .parse()
                    .map_err(|_| ProtocolError::Generic("Conversion error".to_string()))?,
            }),
        }
    }
}

#[cfg(feature = "state-transition-value-conversion")]
impl TryFrom<TokenPaymentInfo> for Value {
    type Error = Error;
    /// Serialize the versioned token payment info into a platform `Value`.
    ///
    /// This mirrors the map format accepted by `TryFrom<BTreeMap<String, Value>>`,
    /// including the `$format_version` discriminator.
    fn try_from(value: TokenPaymentInfo) -> Result<Self, Self::Error> {
        platform_value::to_value(value)
    }
}
