//! Document action fees: a fixed amount of credits a document type charges, on top of the gas,
//! for an action on one of its documents (the `actionFees` keyword, protocol version 14).
//!
//! Each fee has two parts. The `owner` part accumulates in the contract's owner pot and the
//! `moderators` part in its moderators pot; a `ContractFeeClaim` state transition pays a pot
//! out. Whoever pays the gas of the action pays its fee. The amounts are fixed when the
//! document type is published and never change.

use crate::balances::credits::{Credits, MAX_CREDITS};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod v0;

use v0::DocumentActionFeesV0;

/// The pricing values of the `actionFees.pricing` schema keyword
pub mod pricing_names {
    /// The declared amounts are scaled by the epoch's fee multiplier (the default)
    pub const FEE_MULTIPLIER: &str = "feeMultiplier";
    /// The declared amounts are charged as written
    pub const FIXED: &str = "fixed";
}

/// One thousand permille: a fee multiplier that changes nothing
pub const FEE_MULTIPLIER_PERMILLE_BASE: u64 = 1000;

/// How the declared amounts of a document type's action fees become the amounts charged.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Hash)]
pub enum ActionFeePricing {
    /// The declared amounts are scaled by the fee multiplier of the epoch the action executes
    /// in, so they follow the network's fees.
    #[default]
    FeeMultiplier,
    /// The declared amounts are charged as written.
    Fixed,
}

impl ActionFeePricing {
    /// The schema value of the pricing
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionFeePricing::FeeMultiplier => pricing_names::FEE_MULTIPLIER,
            ActionFeePricing::Fixed => pricing_names::FIXED,
        }
    }

    /// The pricing a schema value names
    pub fn from_schema_value(value: &str) -> Option<Self> {
        match value {
            pricing_names::FEE_MULTIPLIER => Some(ActionFeePricing::FeeMultiplier),
            pricing_names::FIXED => Some(ActionFeePricing::Fixed),
            _ => None,
        }
    }
}

/// The two pots a contract's action fees accumulate in. A `ContractFeeClaim` state transition
/// names the one it pays out.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    DecodeUntrusted,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum ContractFeePot {
    /// The pot the contract owner claims
    Owner,
    /// The pot the contract's moderation team shares equally
    Moderators,
}

impl fmt::Display for ContractFeePot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractFeePot::Owner => write!(f, "owner"),
            ContractFeePot::Moderators => write!(f, "moderators"),
        }
    }
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for ContractFeePot {}

/// The fee of one document action, in credits.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Hash)]
pub struct DocumentActionFee {
    /// The credits that go to the contract's owner pot
    pub owner: Credits,
    /// The credits that go to the contract's moderators pot
    pub moderators: Credits,
}

impl DocumentActionFee {
    /// The fee, `None` when both parts are zero
    pub fn non_zero(self) -> Option<Self> {
        (self.owner != 0 || self.moderators != 0).then_some(self)
    }

    /// The sum of both parts
    pub fn total(&self) -> Result<Credits, ProtocolError> {
        self.owner
            .checked_add(self.moderators)
            .filter(|total| *total <= MAX_CREDITS)
            .ok_or(ProtocolError::Overflow(
                "document action fee parts overflow the maximum credits",
            ))
    }

    /// The part that goes to `pot`
    pub fn part(&self, pot: ContractFeePot) -> Credits {
        match pot {
            ContractFeePot::Owner => self.owner,
            ContractFeePot::Moderators => self.moderators,
        }
    }

    /// The amounts charged for the declared fee under `pricing`.
    ///
    /// `fee_multiplier_permille` is the fee multiplier of the epoch the action executes in. It
    /// is only read under [`ActionFeePricing::FeeMultiplier`].
    pub fn charged(
        &self,
        pricing: ActionFeePricing,
        fee_multiplier_permille: u64,
    ) -> Result<Self, ProtocolError> {
        match pricing {
            ActionFeePricing::Fixed => Ok(*self),
            ActionFeePricing::FeeMultiplier => Ok(DocumentActionFee {
                owner: scale(self.owner, fee_multiplier_permille)?,
                moderators: scale(self.moderators, fee_multiplier_permille)?,
            }),
        }
    }
}

fn scale(amount: Credits, fee_multiplier_permille: u64) -> Result<Credits, ProtocolError> {
    let scaled = (amount as u128)
        .checked_mul(fee_multiplier_permille as u128)
        .map(|product| product / FEE_MULTIPLIER_PERMILLE_BASE as u128)
        .ok_or(ProtocolError::Overflow(
            "document action fee overflows when scaled by the fee multiplier",
        ))?;
    Credits::try_from(scaled)
        .ok()
        .filter(|scaled| *scaled <= MAX_CREDITS)
        .ok_or(ProtocolError::Overflow(
            "document action fee scaled by the fee multiplier overflows the maximum credits",
        ))
}

/// The action fees of a document type
#[derive(Debug, PartialEq, Eq, Clone, From)]
pub enum DocumentActionFees {
    /// Version 0 of the action fees
    V0(DocumentActionFeesV0),
}

impl DocumentActionFees {
    /// How the declared amounts become the amounts charged
    pub fn pricing(&self) -> ActionFeePricing {
        match self {
            DocumentActionFees::V0(v0) => v0.pricing,
        }
    }

    /// The fee of creating a document
    pub fn document_creation_action_fee(&self) -> Option<DocumentActionFee> {
        match self {
            DocumentActionFees::V0(v0) => v0.create,
        }
    }

    /// The fee of replacing a document
    pub fn document_replacement_action_fee(&self) -> Option<DocumentActionFee> {
        match self {
            DocumentActionFees::V0(v0) => v0.replace,
        }
    }

    /// The fee of deleting a document
    pub fn document_deletion_action_fee(&self) -> Option<DocumentActionFee> {
        match self {
            DocumentActionFees::V0(v0) => v0.delete,
        }
    }

    /// The fee of transferring a document
    pub fn document_transfer_action_fee(&self) -> Option<DocumentActionFee> {
        match self {
            DocumentActionFees::V0(v0) => v0.transfer,
        }
    }

    /// The fee of updating the price of a document
    pub fn document_price_update_action_fee(&self) -> Option<DocumentActionFee> {
        match self {
            DocumentActionFees::V0(v0) => v0.update_price,
        }
    }

    /// The fee of purchasing a document
    pub fn document_purchase_action_fee(&self) -> Option<DocumentActionFee> {
        match self {
            DocumentActionFees::V0(v0) => v0.purchase,
        }
    }

    /// Every declared fee, in the order of the schema's action keys
    pub fn all(&self) -> impl Iterator<Item = DocumentActionFee> + '_ {
        match self {
            DocumentActionFees::V0(v0) => [
                v0.create,
                v0.replace,
                v0.delete,
                v0.transfer,
                v0.update_price,
                v0.purchase,
            ]
            .into_iter()
            .flatten(),
        }
    }

    /// Whether any action charges a non-zero moderators part
    pub fn charges_moderators_part(&self) -> bool {
        self.all().any(|fee| fee.moderators != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
    use crate::data_contract::document_type::DocumentType;
    use platform_value::{platform_value, Identifier, Value};
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// A document type with one string property and the given `actionFees`.
    fn parse_with_action_fees(
        action_fees: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentType, ProtocolError> {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0, "maxLength": 60_u32},
            },
            "actionFees": action_fees,
            "additionalProperties": false,
        });
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create a default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "post",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut Vec::new(),
            platform_version,
        )
    }

    #[test]
    fn should_parse_action_fees_with_the_fee_multiplier_pricing_by_default() {
        let document_type = parse_with_action_fees(
            platform_value!({
                "create": {"moderators": 100_000_000_u64, "owner": 10_000_000_u64},
                "delete": {"owner": 5_u64},
            }),
            true,
            PlatformVersion::latest(),
        )
        .expect("expected the action fees to parse");

        let fees = document_type.action_fees().expect("expected action fees");
        assert_eq!(fees.pricing(), ActionFeePricing::FeeMultiplier);
        assert_eq!(
            fees.document_creation_action_fee(),
            Some(DocumentActionFee {
                owner: 10_000_000,
                moderators: 100_000_000,
            })
        );
        assert_eq!(
            fees.document_deletion_action_fee(),
            Some(DocumentActionFee {
                owner: 5,
                moderators: 0,
            })
        );
        assert_eq!(fees.document_replacement_action_fee(), None);
        assert!(fees.charges_moderators_part());
    }

    #[test]
    fn should_parse_the_fixed_pricing() {
        let document_type = parse_with_action_fees(
            platform_value!({"pricing": "fixed", "create": {"owner": 1_u64}}),
            true,
            PlatformVersion::latest(),
        )
        .expect("expected the action fees to parse");
        let fees = document_type.action_fees().expect("expected action fees");
        assert_eq!(fees.pricing(), ActionFeePricing::Fixed);
        assert!(!fees.charges_moderators_part());
    }

    #[test]
    fn should_refuse_malformed_action_fees_on_a_contract_entering_the_chain() {
        for action_fees in [
            // Prices no action
            platform_value!({"pricing": "fixed"}),
            // A priced action that charges nothing
            platform_value!({"create": {"owner": 0_u64, "moderators": 0_u64}}),
            // An unknown pricing
            platform_value!({"pricing": "percent", "create": {"owner": 1_u64}}),
            // An unknown action
            platform_value!({"archive": {"owner": 1_u64}}),
            // An unknown part
            platform_value!({"create": {"treasury": 1_u64}}),
            // Over the maximum credits
            platform_value!({"create": {"owner": u64::MAX}}),
        ] {
            assert!(
                parse_with_action_fees(action_fees.clone(), true, PlatformVersion::latest())
                    .is_err(),
                "expected {action_fees:?} to be refused"
            );
        }
    }

    #[test]
    fn should_read_malformed_action_fees_of_a_stored_contract_as_none() {
        // A contract admitted by the frozen v0 meta-schema may carry a stray `actionFees` key
        // of any shape; it must stay loadable, and charge nothing.
        for action_fees in [
            platform_value!("free"),
            platform_value!({"pricing": "percent", "create": {"owner": 1_u64}}),
            platform_value!({"create": {"owner": 0_u64}}),
        ] {
            let document_type =
                parse_with_action_fees(action_fees, false, PlatformVersion::latest())
                    .expect("expected a stored contract to stay loadable");
            assert!(document_type.action_fees().is_none());
        }
    }

    #[test]
    fn should_read_action_fees_from_protocol_version_14_only() {
        let action_fees = platform_value!({"create": {"owner": 1_u64}});
        // The frozen v0 meta-schema (protocol versions 1 to 11) does not forbid unknown
        // top-level keys, and the parsers of those versions never look at this one: the key
        // is carried and charges nothing.
        for protocol_version in 1..=11 {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected the protocol version");
            let document_type = parse_with_action_fees(action_fees.clone(), true, platform_version)
                .expect("expected the v0 meta-schema to admit the unknown key");
            assert!(
                document_type.action_fees().is_none(),
                "expected protocol version {protocol_version} to ignore `actionFees`"
            );
        }
        // The v1 and v2 meta-schemas forbid unknown top-level keys.
        for protocol_version in 12..=13 {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected the protocol version");
            assert!(
                parse_with_action_fees(action_fees.clone(), true, platform_version).is_err(),
                "expected protocol version {protocol_version} to refuse `actionFees`"
            );
        }
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let document_type = parse_with_action_fees(action_fees, true, platform_version)
            .expect("expected protocol version 14 to admit `actionFees`");
        assert!(document_type.action_fees().is_some());
    }

    #[test]
    fn should_charge_the_declared_amounts_when_fixed() {
        let fee = DocumentActionFee {
            owner: 10_000_000,
            moderators: 100_000_000,
        };
        assert_eq!(
            fee.charged(ActionFeePricing::Fixed, 2_500)
                .expect("charged"),
            fee
        );
    }

    #[test]
    fn should_scale_the_declared_amounts_by_the_fee_multiplier() {
        let fee = DocumentActionFee {
            owner: 10_000_000,
            moderators: 100_000_001,
        };
        assert_eq!(
            fee.charged(ActionFeePricing::FeeMultiplier, 1_000)
                .expect("charged"),
            fee
        );
        assert_eq!(
            fee.charged(ActionFeePricing::FeeMultiplier, 1_500)
                .expect("charged"),
            DocumentActionFee {
                owner: 15_000_000,
                // floor(100_000_001 * 1.5)
                moderators: 150_000_001,
            }
        );
    }

    #[test]
    fn should_refuse_a_scaled_amount_over_the_maximum_credits() {
        let fee = DocumentActionFee {
            owner: MAX_CREDITS,
            moderators: 0,
        };
        assert!(fee.charged(ActionFeePricing::FeeMultiplier, 1_001).is_err());
        assert!(fee.charged(ActionFeePricing::FeeMultiplier, 1_000).is_ok());
    }

    #[test]
    fn should_refuse_a_total_over_the_maximum_credits() {
        let fee = DocumentActionFee {
            owner: MAX_CREDITS,
            moderators: 1,
        };
        assert!(fee.total().is_err());
    }

    #[test]
    fn should_round_trip_the_pricing_schema_values() {
        for pricing in [ActionFeePricing::FeeMultiplier, ActionFeePricing::Fixed] {
            assert_eq!(
                ActionFeePricing::from_schema_value(pricing.as_str()),
                Some(pricing)
            );
        }
        assert_eq!(ActionFeePricing::from_schema_value("percent"), None);
        assert_eq!(ActionFeePricing::default(), ActionFeePricing::FeeMultiplier);
    }
}
