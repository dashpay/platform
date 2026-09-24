//! Document action fees: a fixed amount of credits a document type charges, on top of the gas,
//! for an action on one of its documents (the `actionFees` keyword, protocol version 14).
//!
//! Each fee has two parts. The `owner` part accumulates in the contract's owner pot and the
//! `moderators` part in its moderators pot; a `ContractFeeClaim` state transition pays a pot
//! out. Whoever pays the gas of the action pays its fee. The amounts are fixed when the
//! document type is published, and a transition names the ones it agrees to pay (see
//! [`agreement`]).

use crate::balances::credits::{Credits, MAX_CREDITS};
use crate::block::epoch::EpochIndex;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::v2::DataContractConfigGettersV2;
use crate::data_contract::document_type::class_methods::{
    consensus_or_protocol_data_contract_error, consensus_or_protocol_value_error,
};
use crate::data_contract::document_type::property_names::ACTION_FEES;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::DataContract;
use crate::prelude::TimestampMillis;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

pub mod agreement;
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
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Hash, Encode, Decode, DecodeUntrusted)]
pub enum ActionFeePricing {
    /// The declared amounts are scaled by the fee multiplier of the epoch the action executes
    /// in, so they follow the network's fees.
    #[default]
    FeeMultiplier,
    /// The declared amounts are charged as written.
    Fixed,
}

impl fmt::Display for ActionFeePricing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
    /// The pot the contract's moderation team shares: equally for a declared team, by its
    /// proposal's reward split for a seated elected team
    Moderators,
}

impl ContractFeePot {
    /// The identities a payout of this pot of `contract` goes to: the contract owner for the
    /// owner pot, the contract's moderation team for the moderators pot, which is empty for a
    /// contract that declares no moderation. Only a recipient may claim the pot.
    ///
    /// For an elected contract this is the interim's team, which may claim the moderators pot
    /// only until a charter is seated on the contract; the claim reads whether one is.
    pub fn recipients(&self, contract: &DataContract) -> BTreeSet<Identifier> {
        let owner_id = contract.owner_id();
        match self {
            ContractFeePot::Owner => BTreeSet::from([owner_id]),
            ContractFeePot::Moderators => contract
                .config()
                .moderation()
                .map(|moderation| moderation.team(&owner_id))
                .unwrap_or_default(),
        }
    }

    /// The identities whose balances the proof of a claim of this pot of `contract` by
    /// `claimant_id` shows: the recipients the contract names, when the claimant is one of
    /// them, as it is for every claim of the owner pot, of a declared team's moderators pot and
    /// of an elected contract's interim team before a charter is seated. Otherwise the claimant
    /// alone: the claimant of an elected contract's moderators pot is then on the seated team,
    /// which the charter contract names and the contract does not, so neither the prover nor
    /// the verifier could list the other payees from the contract.
    pub fn claim_proof_identities(
        &self,
        contract: &DataContract,
        claimant_id: Identifier,
    ) -> BTreeSet<Identifier> {
        let recipients = self.recipients(contract);
        if recipients.contains(&claimant_id) {
            recipients
        } else {
            BTreeSet::from([claimant_id])
        }
    }
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

/// The last payout of a contract fee pot, which the claim that made it leaves in state. The
/// next claim is judged against its epoch, and the rest tells the recipients of the pot who
/// paid them and when.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode, DecodeUntrusted, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct ContractFeePotLastClaim {
    /// The epoch the pot was paid out in. A pot is paid out at most once per epoch.
    pub epoch_index: EpochIndex,
    /// The time of the block that paid the pot out, in milliseconds.
    pub time_ms: TimestampMillis,
    /// The identity that signed the claim: the contract owner for the owner pot, and for the
    /// moderators pot the member of the team that claimed it for all of them.
    pub claimant_id: Identifier,
}

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

    /// The sum of both parts, held at `MAX_CREDITS`
    pub fn saturating_total(&self) -> Credits {
        self.owner.saturating_add(self.moderators).min(MAX_CREDITS)
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

/// `amount` scaled by the multiplier, held at `MAX_CREDITS`. The multiplier is the network's and
/// has no upper bound, so a declared amount that fits today may not tomorrow. A fee held at the
/// maximum is one nobody can pay: the action is refused for an insufficient balance, a consensus
/// error a client can read, where an overflow would have failed every transition on the action
/// with an internal one.
fn scale(amount: Credits, fee_multiplier_permille: u64) -> Result<Credits, ProtocolError> {
    let scaled = (amount as u128).saturating_mul(fee_multiplier_permille as u128)
        / FEE_MULTIPLIER_PERMILLE_BASE as u128;
    Ok(Credits::try_from(scaled)
        .unwrap_or(MAX_CREDITS)
        .min(MAX_CREDITS))
}

/// The action fees of a document type
#[derive(Debug, PartialEq, Eq, Clone, From)]
pub enum DocumentActionFees {
    /// Version 0 of the action fees
    V0(DocumentActionFeesV0),
}

impl DocumentActionFees {
    /// Reads the `actionFees` keyword of a document type's schema, `None` when the schema
    /// declares none.
    ///
    /// A declaration must price at least one action, every priced action must charge
    /// something, and no amount (nor the sum of an action's two parts) may exceed
    /// `MAX_CREDITS`.
    pub fn try_from_document_schema(
        schema: &Value,
        name: &str,
    ) -> Result<Option<Self>, ProtocolError> {
        let structure_error = |message: String| {
            consensus_or_protocol_data_contract_error(DataContractError::InvalidContractStructure(
                message,
            ))
        };

        let Ok(schema_map) = schema.to_map() else {
            return Ok(None);
        };
        let Some(action_fees) = Value::get_optional_from_map(schema_map, ACTION_FEES) else {
            return Ok(None);
        };
        if !action_fees.is_map() {
            return Err(structure_error(format!(
                "document type \"{name}\": `{ACTION_FEES}` must be an object"
            )));
        }

        let pricing = match action_fees
            .get_optional_str("pricing")
            .map_err(consensus_or_protocol_value_error)?
        {
            None => ActionFeePricing::default(),
            Some(value) => ActionFeePricing::from_schema_value(value).ok_or_else(|| {
                structure_error(format!(
                    "document type \"{name}\": `{ACTION_FEES}.pricing` must be \"{}\" or \"{}\", \
                     got \"{value}\"",
                    pricing_names::FEE_MULTIPLIER,
                    pricing_names::FIXED,
                ))
            })?,
        };

        let extract_fee = |action: &str| -> Result<Option<DocumentActionFee>, ProtocolError> {
            let Some(entry) = action_fees
                .get_optional_value(action)
                .map_err(consensus_or_protocol_value_error)?
            else {
                return Ok(None);
            };
            if !entry.is_map() {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{ACTION_FEES}.{action}` must be an object"
                )));
            }
            let part = |key: &str| -> Result<Credits, ProtocolError> {
                let amount = entry
                    .get_optional_integer::<Credits>(key)
                    .map_err(consensus_or_protocol_value_error)?
                    .unwrap_or_default();
                if amount > MAX_CREDITS {
                    return Err(structure_error(format!(
                        "document type \"{name}\": `{ACTION_FEES}.{action}.{key}` of {amount} \
                         credits is over the maximum of {MAX_CREDITS}"
                    )));
                }
                Ok(amount)
            };
            let fee = DocumentActionFee {
                owner: part("owner")?,
                moderators: part("moderators")?,
            };
            if fee.non_zero().is_none() {
                return Err(structure_error(format!(
                    "document type \"{name}\": `{ACTION_FEES}.{action}` charges nothing; leave the \
                     action out instead"
                )));
            }
            if fee.total().is_err() {
                return Err(structure_error(format!(
                    "document type \"{name}\": the two parts of `{ACTION_FEES}.{action}` add up to \
                     more than the maximum of {MAX_CREDITS} credits"
                )));
            }
            Ok(Some(fee))
        };

        let fees = DocumentActionFeesV0 {
            pricing,
            create: extract_fee("create")?,
            replace: extract_fee("replace")?,
            delete: extract_fee("delete")?,
            transfer: extract_fee("transfer")?,
            update_price: extract_fee("update_price")?,
            purchase: extract_fee("purchase")?,
        };
        let fees: DocumentActionFees = fees.into();
        if fees.all().next().is_none() {
            return Err(structure_error(format!(
                "document type \"{name}\": `{ACTION_FEES}` prices no action"
            )));
        }
        Ok(Some(fees))
    }

    /// The name of the first document type, in name order, whose schema charges a moderators
    /// part. A schema whose declaration is malformed is skipped: refusing it is the job of the
    /// document type parser.
    pub fn first_document_type_charging_moderators<'a>(
        document_schemas: impl IntoIterator<Item = (&'a String, &'a Value)>,
    ) -> Option<&'a String> {
        document_schemas
            .into_iter()
            .find(|(name, schema)| {
                matches!(
                    Self::try_from_document_schema(schema, name),
                    Ok(Some(fees)) if fees.charges_moderators_part()
                )
            })
            .map(|(name, _)| name)
    }
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

    /// The fee of `action`, `None` when the document type charges nothing for it. Deleting an
    /// index-only document is a deletion.
    pub fn action_fee(&self, action: DocumentTransitionActionType) -> Option<DocumentActionFee> {
        match action {
            DocumentTransitionActionType::Create => self.document_creation_action_fee(),
            DocumentTransitionActionType::Replace => self.document_replacement_action_fee(),
            DocumentTransitionActionType::Delete
            | DocumentTransitionActionType::IndexOnlyDelete => self.document_deletion_action_fee(),
            DocumentTransitionActionType::Transfer => self.document_transfer_action_fee(),
            DocumentTransitionActionType::UpdatePrice => self.document_price_update_action_fee(),
            DocumentTransitionActionType::Purchase => self.document_purchase_action_fee(),
            DocumentTransitionActionType::IgnoreWhileBumpingRevision => None,
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
    fn should_refuse_malformed_action_fees_on_both_paths() {
        // The shape of a doctype-level keyword is enforced on the stored path as on the
        // validating one: every declaration a node reads from state was validated, so a
        // malformed one is a fault to surface, never a contract without fees.
        for full_validation in [false, true] {
            for action_fees in [
                platform_value!("free"),
                platform_value!({"pricing": "percent", "create": {"owner": 1_u64}}),
                platform_value!({"create": {"owner": 0_u64}}),
            ] {
                assert!(
                    parse_with_action_fees(
                        action_fees.clone(),
                        full_validation,
                        PlatformVersion::latest()
                    )
                    .is_err(),
                    "expected {action_fees:?} to be refused with full_validation {full_validation}"
                );
            }
        }
    }

    #[test]
    fn should_parse_a_moderators_fee_without_moderation_on_both_paths() {
        // The document type does not know whether its contract declares moderation. It parses
        // on both paths, and the contract is what refuses the combination when it enters the
        // chain (`DocumentActionFeesWithoutModerationError`, 10902). Moderation is never turned
        // off, so no validated contract is in that state.
        let action_fees = platform_value!({"create": {"owner": 5_u64, "moderators": 10_u64}});
        for full_validation in [false, true] {
            let document_type = parse_with_action_fees(
                action_fees.clone(),
                full_validation,
                PlatformVersion::latest(),
            )
            .expect("expected the document type to parse");
            let fees = document_type.action_fees().expect("expected action fees");
            assert!(fees.charges_moderators_part());
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
    fn should_hold_a_scaled_amount_at_the_maximum_credits() {
        let fee = DocumentActionFee {
            owner: MAX_CREDITS,
            moderators: 7,
        };
        let charged = fee
            .charged(ActionFeePricing::FeeMultiplier, u64::MAX)
            .expect("charged");
        assert_eq!(charged.owner, MAX_CREDITS);
        // 7 credits at that multiplier still fit, and are scaled as usual.
        assert_eq!(charged.moderators, 129_127_208_515_966_861);
        assert_eq!(charged.saturating_total(), MAX_CREDITS);
        assert_eq!(
            fee.charged(ActionFeePricing::FeeMultiplier, 1_000)
                .expect("charged"),
            fee
        );
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
