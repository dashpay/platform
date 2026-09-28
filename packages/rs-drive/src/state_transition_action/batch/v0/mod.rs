use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::{
    GasPayer, ResolvedContractGroupMemberships, ResolvedGasSponsor,
};
use crate::state_transition_action::contract::moderators_pot_settlement::ModeratorsPotSettlement;
use dpp::prelude::FeeMultiplier;
use dpp::consensus::state::document::document_action_fee_agreement_mismatch_error::DocumentActionFeeAgreementMismatchError;
use dpp::consensus::state::document::document_action_fee_agreement_not_set_error::DocumentActionFeeAgreementNotSetError;
use dpp::consensus::state::document::document_action_fee_moderators_share_mismatch_error::DocumentActionFeeModeratorsShareMismatchError;
use dpp::moderation_charter::moderators_share_of;
use dpp::consensus::state::document::document_action_fee_multiplier_not_tolerated_error::DocumentActionFeeMultiplierNotToleratedError;
use dpp::consensus::state::token::{GasFeesPaidByNotAllowedError, InconsistentGasFeesPaidByInBatchError};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::fee::Credits;
use std::collections::{BTreeMap, BTreeSet};
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use dpp::identifier::Identifier;
use dpp::prelude::UserFeeIncrease;
use dpp::ProtocolError;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::TokenDirectPurchaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct BatchTransitionActionV0 {
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<BatchedTransitionAction>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// The contract group memberships of every contract the batch touches, keyed by contract
    /// id, as the transformer read them from state (protocol version 14). Resolved only when
    /// the batch is signed by a key bound to a contract group, so that key can be judged from
    /// the action alone; empty for every other batch and under earlier transformer versions.
    pub contract_group_memberships: BTreeMap<Identifier, ResolvedContractGroupMemberships>,
    /// The contract owner sponsoring the batch's gas, resolved by the batch transformer from
    /// protocol version 14 (see `ResolvedGasSponsor`)
    pub gas_sponsor: Option<ResolvedGasSponsor>,
    /// The fee multiplier, in permille, of the epoch the batch executes in, read by the batch
    /// transformer from protocol version 14 when some document transition declares an action
    /// fee priced by it. The fees themselves are not kept: they are read off the transitions
    /// when the batch executes, after state validation had its say on each of them.
    pub action_fee_multiplier_permille: Option<FeeMultiplier>,
    /// The moderators share of the seated moderation charter of each elected contract on whose
    /// moderated document types some document transition of the batch agrees to a discounted
    /// moderators part, read by the batch transformer from protocol version 14: `None` when no
    /// charter is seated on the contract. Empty for a batch that asks for no discount, which
    /// reads nothing.
    pub seated_moderators_shares: BTreeMap<Identifier, Option<u8>>,

    /// The contracts, among those the batch touches, on which the transformer found the batch
    /// owner's suspension lapsed (protocol version 14). Each such suspension is deleted when
    /// the batch executes: the first document transition after a suspension lapses sweeps it.
    /// Only ever the owner's own: the identity is not stored, so nothing can queue another's.
    pub lapsed_suspensions: BTreeSet<Identifier>,

    /// The settles of elected contracts' moderators pots the batch forces before it changes a
    /// seated team (protocol version 14): an `addedModerator` or `removedModerator` of the
    /// moderation charters contract created or deleted pays the pot out to the team as it was
    /// before the change, by its proposal's reward split, and resets the action counts. Set
    /// by the batch's state validation, which reads the team, the pot and the counts; empty
    /// for every other batch.
    pub moderators_pot_settlements: Vec<ModeratorsPotSettlement>,
}

impl BatchTransitionActionV0 {
    /// See `BatchTransitionAction::validate_action_fee_agreements`
    pub(in crate::state_transition_action) fn validate_action_fee_agreements(
        &self,
    ) -> Result<Result<(), ConsensusError>, ProtocolError> {
        for transition in &self.transitions {
            let BatchedTransitionAction::DocumentAction(document_action) = transition else {
                continue;
            };
            let base = document_action.base();
            let Some(declared) = base.declared_action_fee_with_agreement() else {
                continue;
            };
            let document_type_name = || base.document_type_name().clone();
            let action = || document_action.action_name().to_string();
            let Some(agreement) = declared.agreement else {
                return Ok(Err(DocumentActionFeeAgreementNotSetError::new(
                    document_type_name(),
                    action(),
                    declared.pricing,
                    declared.fee,
                )
                .into()));
            };
            if base.agrees_to_a_moderators_discount() {
                // Less than the declared moderators part, on a type an elected contract
                // moderates: exactly the share the contract's seated charter takes of it, and
                // nothing without a seated charter.
                let moderators_share = *self
                    .seated_moderators_shares
                    .get(&base.data_contract_id())
                    .ok_or(ProtocolError::CorruptedCodeExecution(
                        "the batch transformer reads the seated moderators share of every \
                         contract a document transition agrees to a discount on"
                            .to_string(),
                    ))?;
                let offered = moderators_share
                    .map(|share| moderators_share_of(declared.fee.moderators, share));
                if offered != Some(agreement.moderators()) {
                    return Ok(Err(DocumentActionFeeModeratorsShareMismatchError::new(
                        document_type_name(),
                        action(),
                        declared.fee.moderators,
                        agreement.moderators(),
                        moderators_share,
                    )
                    .into()));
                }
            } else if !agreement.matches_declared(declared.pricing, declared.fee) {
                return Ok(Err(DocumentActionFeeAgreementMismatchError::new(
                    document_type_name(),
                    action(),
                    declared.pricing,
                    declared.fee,
                    &agreement,
                )
                .into()));
            }
            // A matching agreement names the fee multiplier exactly when the fee follows it.
            if let Some(agreed_fee_multiplier) = agreement.fee_multiplier() {
                let current = self.action_fee_multiplier_permille.ok_or(
                    ProtocolError::CorruptedCodeExecution(
                        "the batch transformer reads the fee multiplier of every batch that \
                         declares an action fee priced by it"
                            .to_string(),
                    ),
                )?;
                if !agreed_fee_multiplier.tolerates(current) {
                    return Ok(Err(DocumentActionFeeMultiplierNotToleratedError::new(
                        document_type_name(),
                        action(),
                        agreed_fee_multiplier,
                        current,
                    )
                    .into()));
                }
            }
        }
        Ok(Ok(()))
    }

    /// See `BatchTransitionAction::resolve_gas_payer`
    pub(in crate::state_transition_action) fn resolve_gas_payer(
        &self,
    ) -> Result<GasPayer, ConsensusError> {
        // `None` is the document owner; `Some` a contract owner.
        let mut payer: Option<Option<Identifier>> = None;
        let mut strict = false;
        for transition in &self.transitions {
            let transition_payer = match transition {
                BatchedTransitionAction::DocumentAction(document_action) => {
                    let base = document_action.base();
                    let requested = base.gas_fees_paid_by();
                    let offered = base.contract_gas_fees_paid_by();
                    match GasFeesPaidBy::resolve(offered, requested) {
                        None => {
                            return Err(GasFeesPaidByNotAllowedError::new(
                                base.document_type_name().clone(),
                                document_action.action_name().to_string(),
                                requested,
                                offered,
                            )
                            .into())
                        }
                        Some(GasFeesPaidBy::DocumentOwner) => None,
                        Some(GasFeesPaidBy::PreferContractOwner) => {
                            Some(base.data_contract_fetch_info_ref().contract.owner_id())
                        }
                        Some(GasFeesPaidBy::ContractOwner) => {
                            strict = true;
                            Some(base.data_contract_fetch_info_ref().contract.owner_id())
                        }
                    }
                }
                // Token transitions are never sponsored.
                BatchedTransitionAction::TokenAction(_) => None,
                // A transition that already failed names no payer of its own: the execution
                // event drops the sponsor of a batch that carries one, so its signer pays, and
                // the error that replaced it must not be masked by an inconsistency here.
                BatchedTransitionAction::BumpIdentityDataContractNonce(_) => continue,
            };
            match payer {
                None => payer = Some(transition_payer),
                Some(expected) if expected != transition_payer => {
                    return Err(InconsistentGasFeesPaidByInBatchError::new(
                        expected,
                        transition_payer,
                    )
                    .into())
                }
                Some(_) => {}
            }
        }
        Ok(match payer.flatten() {
            None => GasPayer::DocumentOwner,
            Some(identity_id) => GasPayer::ContractOwner {
                identity_id,
                strict,
            },
        })
    }

    pub(in crate::state_transition_action) fn all_used_balances(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        Ok(match (self.all_purchases_amount()?, self.all_conflicting_index_collateral_voting_funds()?) {
            (Some(all_purchases_amount), Some(all_conflicting_index_collateral_voting_funds)) => Some(all_purchases_amount.checked_add(all_conflicting_index_collateral_voting_funds).ok_or(ProtocolError::Overflow("overflow between all_purchases_amount and all_conflicting_index_collateral_voting_funds"))?),
            (Some(all_purchases_amount), None) => Some(all_purchases_amount),
            (None, Some(all_conflicting_index_collateral_voting_funds)) => Some(all_conflicting_index_collateral_voting_funds),
            (None, None) => None,
        })
    }
    pub(in crate::state_transition_action) fn all_purchases_amount(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        let (total, any_purchases): (Option<Credits>, bool) = self
            .transitions
            .iter()
            .filter_map(|transition| match transition {
                BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::PurchaseAction(document_purchase),
                ) => Some(document_purchase.price()),
                BatchedTransitionAction::TokenAction(
                    TokenTransitionAction::DirectPurchaseAction(token_purchase),
                ) => Some(token_purchase.total_agreed_price()),
                _ => None,
            })
            .fold((None, false), |(acc, _), price| match acc {
                Some(acc_val) => acc_val
                    .checked_add(price)
                    .map_or((None, true), |sum| (Some(sum), true)),
                None => (Some(price), true),
            });

        match (total, any_purchases) {
            (Some(total), _) => Ok(Some(total)),
            (None, true) => Err(ProtocolError::Overflow("overflow in all purchases amount")), // Overflow occurred
            _ => Ok(None), // No purchases were found
        }
    }

    pub(in crate::state_transition_action) fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        let (total, any_voting_funds): (Option<Credits>, bool) = self
            .transitions
            .iter()
            .filter_map(|transition| match transition {
                BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::CreateAction(document_create_transition_action),
                ) => document_create_transition_action
                    .prefunded_voting_balance()
                    .iter()
                    .try_fold(0u64, |acc, &(_, val)| acc.checked_add(val)),
                _ => None,
            })
            .fold((None, false), |(acc, _), price| match acc {
                Some(acc_val) => acc_val
                    .checked_add(price)
                    .map_or((None, true), |sum| (Some(sum), true)),
                None => (Some(price), true),
            });

        match (total, any_voting_funds) {
            (Some(total), _) => Ok(Some(total)),
            (None, true) => Err(ProtocolError::Overflow(
                "overflow in all voting funds amount",
            )), // Overflow occurred
            _ => Ok(None),
        }
    }
}
