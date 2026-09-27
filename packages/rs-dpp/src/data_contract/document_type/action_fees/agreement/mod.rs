//! What a document transition agrees to pay in action fees (protocol version 14).
//!
//! A document type's `actionFees` are read off the contract when the action executes, so a
//! transition that said nothing would pay whatever the contract declares by then. The
//! agreement closes that gap: the transition names the two amounts its signer saw declared for
//! the action, and the network refuses the action when they are no longer what the document
//! type declares. For a fee priced by the fee multiplier it also names the multiplier the
//! signer knew and how far above it the multiplier of the executing epoch may be, so a
//! transition signed just before an epoch boundary is not refused for a small move.
//!
//! An action whose document type charges a fee must carry an agreement. One on an action that
//! charges nothing is ignored: its signer pays nothing, which is never more than they agreed to.

use crate::balances::credits::Credits;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::action_fees::{ActionFeePricing, DocumentActionFee};
use crate::data_contract::document_type::DocumentTypeRef;
use crate::prelude::FeeMultiplier;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;
use crate::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::{Display, From};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

pub mod v0;

use v0::DocumentActionFeeAgreementV0;

/// The fee multiplier a transition's signer knew, and how far above it they accept to pay.
#[derive(Debug, Clone, Copy, Encode, Decode, DecodeUntrusted, PartialEq, Eq, Display)]
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[display(
    "Known: {} permille, Increase Tolerance: {}%",
    known_permille,
    increase_tolerance_percent
)]
pub struct AgreedFeeMultiplier {
    /// The fee multiplier, in permille, the signer priced the fee with
    pub known_permille: FeeMultiplier,
    /// How far the multiplier of the executing epoch may be above the known one, in percent of
    /// the known one: 20 accepts a multiplier up to 1.2 times the known one. A lower multiplier
    /// is always accepted.
    pub increase_tolerance_percent: u16,
}

impl AgreedFeeMultiplier {
    /// Whether the signer accepts a fee priced with `current_permille`
    pub fn tolerates(&self, current_permille: FeeMultiplier) -> bool {
        let ceiling =
            (self.known_permille as u128) * (100 + self.increase_tolerance_percent as u128);
        (current_permille as u128) * 100 <= ceiling
    }
}

/// What a document transition agrees to pay in action fees
#[derive(Debug, Clone, Copy, Encode, Decode, DecodeUntrusted, PartialEq, Eq, Display, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum DocumentActionFeeAgreement {
    /// Version 0 of the agreement
    #[display("V0({})", _0)]
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DocumentActionFeeAgreementV0),
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for DocumentActionFeeAgreement {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for DocumentActionFeeAgreement {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for DocumentActionFeeAgreement {}

impl DocumentActionFeeAgreement {
    /// The agreement to `fee` as a document type declares it under `pricing`.
    ///
    /// `fee_multiplier` is only kept for a fee priced by the fee multiplier: a fixed fee does
    /// not follow it, and its agreement says so by naming none.
    pub fn for_declared_fee(
        pricing: ActionFeePricing,
        fee: DocumentActionFee,
        fee_multiplier: AgreedFeeMultiplier,
    ) -> Self {
        DocumentActionFeeAgreementV0 {
            owner: fee.owner,
            moderators: fee.moderators,
            fee_multiplier: match pricing {
                ActionFeePricing::FeeMultiplier => Some(fee_multiplier),
                ActionFeePricing::Fixed => None,
            },
        }
        .into()
    }

    /// The agreement a transition performing `action` on a document of `document_type` must
    /// carry, `None` when the document type charges nothing for it. It names what
    /// `document_type` declares, so it is only as current as the contract it was read from.
    pub fn for_document_type_action(
        document_type: DocumentTypeRef,
        action: DocumentTransitionActionType,
        fee_multiplier: AgreedFeeMultiplier,
    ) -> Option<Self> {
        let fees = document_type.action_fees()?;
        let fee = fees.action_fee(action)?;
        Some(Self::for_declared_fee(fees.pricing(), fee, fee_multiplier))
    }

    /// The owner part the signer saw declared, in credits
    pub fn owner(&self) -> Credits {
        match self {
            DocumentActionFeeAgreement::V0(v0) => v0.owner,
        }
    }

    /// The moderators part the signer saw declared, in credits
    pub fn moderators(&self) -> Credits {
        match self {
            DocumentActionFeeAgreement::V0(v0) => v0.moderators,
        }
    }

    /// The fee multiplier terms, `None` when the signer agreed to a fixed fee
    pub fn fee_multiplier(&self) -> Option<AgreedFeeMultiplier> {
        match self {
            DocumentActionFeeAgreement::V0(v0) => v0.fee_multiplier,
        }
    }

    /// The declared fee the signer agreed to
    pub fn fee(&self) -> DocumentActionFee {
        DocumentActionFee {
            owner: self.owner(),
            moderators: self.moderators(),
        }
    }

    /// The pricing the signer agreed to: the fee multiplier when they named its terms
    pub fn pricing(&self) -> ActionFeePricing {
        match self.fee_multiplier() {
            Some(_) => ActionFeePricing::FeeMultiplier,
            None => ActionFeePricing::Fixed,
        }
    }

    /// Whether the agreement names exactly what the document type declares: both amounts and
    /// the pricing. A lower declared fee does not match either; the signer reads the contract
    /// again and agrees to what it declares now.
    pub fn matches_declared(&self, pricing: ActionFeePricing, fee: DocumentActionFee) -> bool {
        self.pricing() == pricing && self.fee() == fee
    }

    /// Whether the agreement asks for a discount on the moderators part: it names the declared
    /// pricing and owner part, and less than the declared moderators part. Only the seated
    /// moderation charter of an elected contract gives one, on a document type the contract
    /// moderates: its `moderatorsShare` of the declared part
    /// ([`moderators_share_of`](crate::moderation_charter::moderators_share_of)). Judging it
    /// needs that charter, which is state; this only says whether there is anything to judge.
    pub fn discounts_moderators_of(
        &self,
        pricing: ActionFeePricing,
        fee: DocumentActionFee,
    ) -> bool {
        self.pricing() == pricing && self.owner() == fee.owner && self.moderators() < fee.moderators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: AgreedFeeMultiplier = AgreedFeeMultiplier {
        known_permille: 1000,
        increase_tolerance_percent: 20,
    };

    const FEE: DocumentActionFee = DocumentActionFee {
        owner: 10_000_000,
        moderators: 100_000_000,
    };

    #[test]
    fn should_ask_for_a_discount_only_below_the_declared_moderators_part() {
        let agreed = |owner, moderators, pricing| {
            DocumentActionFeeAgreement::for_declared_fee(
                pricing,
                DocumentActionFee { owner, moderators },
                KNOWN,
            )
        };
        let fixed = ActionFeePricing::Fixed;
        assert!(agreed(FEE.owner, FEE.moderators - 1, fixed).discounts_moderators_of(fixed, FEE));
        assert!(agreed(FEE.owner, 0, fixed).discounts_moderators_of(fixed, FEE));
        // The declared amounts are no discount, nor is anything that also moves the owner
        // part or the pricing, nor a higher moderators part.
        for (owner, moderators, pricing) in [
            (FEE.owner, FEE.moderators, fixed),
            (FEE.owner, FEE.moderators + 1, fixed),
            (FEE.owner - 1, FEE.moderators - 1, fixed),
            (
                FEE.owner,
                FEE.moderators - 1,
                ActionFeePricing::FeeMultiplier,
            ),
        ] {
            assert!(!agreed(owner, moderators, pricing).discounts_moderators_of(fixed, FEE));
        }
    }

    #[test]
    fn should_tolerate_a_fee_multiplier_up_to_the_stated_increase() {
        assert!(KNOWN.tolerates(1000));
        assert!(KNOWN.tolerates(1200));
        assert!(!KNOWN.tolerates(1201));
    }

    #[test]
    fn should_tolerate_any_lower_fee_multiplier() {
        assert!(KNOWN.tolerates(999));
        assert!(KNOWN.tolerates(0));
    }

    #[test]
    fn should_tolerate_only_the_known_fee_multiplier_without_a_tolerance() {
        let exact = AgreedFeeMultiplier {
            known_permille: 1500,
            increase_tolerance_percent: 0,
        };
        assert!(exact.tolerates(1500));
        assert!(!exact.tolerates(1501));
    }

    #[test]
    fn should_not_overflow_at_the_largest_terms() {
        let largest = AgreedFeeMultiplier {
            known_permille: FeeMultiplier::MAX,
            increase_tolerance_percent: u16::MAX,
        };
        assert!(largest.tolerates(FeeMultiplier::MAX));
    }

    #[test]
    fn should_name_the_fee_multiplier_only_for_a_fee_priced_by_it() {
        let by_multiplier = DocumentActionFeeAgreement::for_declared_fee(
            ActionFeePricing::FeeMultiplier,
            FEE,
            KNOWN,
        );
        assert_eq!(by_multiplier.fee_multiplier(), Some(KNOWN));
        assert_eq!(by_multiplier.pricing(), ActionFeePricing::FeeMultiplier);

        let fixed =
            DocumentActionFeeAgreement::for_declared_fee(ActionFeePricing::Fixed, FEE, KNOWN);
        assert_eq!(fixed.fee_multiplier(), None);
        assert_eq!(fixed.pricing(), ActionFeePricing::Fixed);
    }

    #[test]
    fn should_match_only_the_declared_amounts_and_pricing() {
        let agreement = DocumentActionFeeAgreement::for_declared_fee(
            ActionFeePricing::FeeMultiplier,
            FEE,
            KNOWN,
        );
        assert!(agreement.matches_declared(ActionFeePricing::FeeMultiplier, FEE));
        assert!(!agreement.matches_declared(ActionFeePricing::Fixed, FEE));
        assert!(!agreement.matches_declared(
            ActionFeePricing::FeeMultiplier,
            DocumentActionFee {
                owner: FEE.owner + 1,
                ..FEE
            }
        ));
        assert!(!agreement.matches_declared(
            ActionFeePricing::FeeMultiplier,
            DocumentActionFee {
                moderators: FEE.moderators - 1,
                ..FEE
            }
        ));
    }

    #[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
    #[test]
    fn should_round_trip_through_json() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;

        let agreement = DocumentActionFeeAgreement::for_declared_fee(
            ActionFeePricing::FeeMultiplier,
            FEE,
            KNOWN,
        );
        let json = JsonConvertible::to_json(&agreement).expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "owner": 10_000_000,
                "moderators": 100_000_000,
                "feeMultiplier": {
                    "knownPermille": 1000,
                    "increaseTolerancePercent": 20,
                },
            })
        );
        let recovered =
            <DocumentActionFeeAgreement as JsonConvertible>::from_json(json).expect("from_json");
        assert_eq!(agreement, recovered);
    }
}
