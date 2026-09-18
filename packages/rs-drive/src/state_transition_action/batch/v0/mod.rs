use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::{GasPayer, ResolvedContractGroupMemberships, ResolvedGasSponsor};
use dpp::consensus::state::token::{GasFeesPaidByNotAllowedError, InconsistentGasFeesPaidByInBatchError};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::fee::Credits;
use std::collections::BTreeMap;
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
}

impl BatchTransitionActionV0 {
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
                // Token transitions are never sponsored, and a transition that already failed
                // is paid for by its signer.
                BatchedTransitionAction::TokenAction(_)
                | BatchedTransitionAction::BumpIdentityDataContractNonce(_) => None,
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
