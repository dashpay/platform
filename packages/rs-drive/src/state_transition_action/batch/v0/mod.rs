use dpp::fee::Credits;
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
}

impl BatchTransitionActionV0 {
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
