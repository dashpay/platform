use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::DocumentCreateTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_direct_purchase_transition_action::TokenDirectPurchaseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::DocumentEraseTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::DocumentEraseTransitionAction;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::state_transition_action::batch::BatchTransitionAction;
use crate::state_transition_action::batch::v0::BatchTransitionActionV0;
use derive_more::From;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::identity::SecurityLevel;
use dpp::prelude::UserFeeIncrease;
use dpp::ProtocolError;

#[cfg(test)]
mod tests;

/// One item of a batch action in format 1.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, From)]
pub enum BatchedTransitionActionV1 {
    /// An action that batch action format 0 can express: a document, token or
    /// identity contract nonce bump action.
    Batched(BatchedTransitionAction),
    /// The erase of an already deleted keep-history document, which only
    /// batch action format 1 can express.
    DocumentErase(DocumentEraseTransitionAction),
}

impl BatchedTransitionActionV1 {
    /// The data contract the item acts on.
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            BatchedTransitionActionV1::Batched(action) => action.data_contract_id(),
            BatchedTransitionActionV1::DocumentErase(action) => action.base().data_contract_id(),
        }
    }

    /// The item as a format 0 action, when it is one.
    pub fn as_batched_action(&self) -> Option<&BatchedTransitionAction> {
        match self {
            BatchedTransitionActionV1::Batched(action) => Some(action),
            BatchedTransitionActionV1::DocumentErase(_) => None,
        }
    }

    /// The item as an erase action, when it is one.
    pub fn as_document_erase_action(&self) -> Option<&DocumentEraseTransitionAction> {
        match self {
            BatchedTransitionActionV1::Batched(_) => None,
            BatchedTransitionActionV1::DocumentErase(action) => Some(action),
        }
    }
}

/// Batch action format 1: what a batch transition becomes once it has been
/// transformed into an action for a protocol version whose batch wire format
/// can carry an erase.
///
/// It is a sibling of [`BatchTransitionAction`] rather than a variant of it. The
/// typed accessors of [`BatchTransitionAction`] hand out format 0 items, and
/// the validation generations selected by earlier protocol versions consume
/// them; a format that carries an erase cannot satisfy those signatures, so it
/// reaches those consumers through its own state transition action arm instead
/// and is only ever produced by the generations that know it.
#[derive(Default, Debug, Clone)]
pub struct BatchTransitionActionV1 {
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions, in the order the batch listed them
    pub transitions: Vec<BatchedTransitionActionV1>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

impl From<BatchTransitionActionV0> for BatchTransitionActionV1 {
    fn from(value: BatchTransitionActionV0) -> Self {
        let BatchTransitionActionV0 {
            owner_id,
            transitions,
            user_fee_increase,
        } = value;
        Self {
            owner_id,
            transitions: transitions
                .into_iter()
                .map(BatchedTransitionActionV1::Batched)
                .collect(),
            user_fee_increase,
        }
    }
}

impl From<BatchTransitionAction> for BatchTransitionActionV1 {
    fn from(value: BatchTransitionAction) -> Self {
        match value {
            BatchTransitionAction::V0(v0) => v0.into(),
        }
    }
}

impl BatchTransitionActionV1 {
    /// owner id
    pub fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    /// transitions
    pub fn transitions(&self) -> &Vec<BatchedTransitionActionV1> {
        &self.transitions
    }

    /// transitions
    pub fn transitions_mut(&mut self) -> &mut Vec<BatchedTransitionActionV1> {
        &mut self.transitions
    }

    /// transitions
    pub fn transitions_take(&mut self) -> Vec<BatchedTransitionActionV1> {
        std::mem::take(&mut self.transitions)
    }

    /// transitions owned
    pub fn transitions_owned(self) -> Vec<BatchedTransitionActionV1> {
        self.transitions
    }

    /// set transitions
    pub fn set_transitions(&mut self, transitions: Vec<BatchedTransitionActionV1>) {
        self.transitions = transitions
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        self.user_fee_increase
    }

    /// The sum of all purchases amount and all conflicting index collateral voting funds
    pub fn all_used_balances(&self) -> Result<Option<Credits>, ProtocolError> {
        Ok(match (self.all_purchases_amount()?, self.all_conflicting_index_collateral_voting_funds()?) {
            (Some(all_purchases_amount), Some(all_conflicting_index_collateral_voting_funds)) => Some(all_purchases_amount.checked_add(all_conflicting_index_collateral_voting_funds).ok_or(ProtocolError::Overflow("overflow between all_purchases_amount and all_conflicting_index_collateral_voting_funds"))?),
            (Some(all_purchases_amount), None) => Some(all_purchases_amount),
            (None, Some(all_conflicting_index_collateral_voting_funds)) => Some(all_conflicting_index_collateral_voting_funds),
            (None, None) => None,
        })
    }

    /// The sum of all purchases amounts for all purchase transitions in the batch.
    /// An erase purchases nothing.
    pub fn all_purchases_amount(&self) -> Result<Option<Credits>, ProtocolError> {
        let (total, any_purchases): (Option<Credits>, bool) = self
            .transitions
            .iter()
            .filter_map(|transition| match transition {
                BatchedTransitionActionV1::Batched(BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::PurchaseAction(document_purchase),
                )) => Some(document_purchase.price()),
                BatchedTransitionActionV1::Batched(BatchedTransitionAction::TokenAction(
                    TokenTransitionAction::DirectPurchaseAction(token_purchase),
                )) => Some(token_purchase.total_agreed_price()),
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
            (None, true) => Err(ProtocolError::Overflow("overflow in all purchases amount")),
            _ => Ok(None),
        }
    }

    /// The sum of all conflicting index collateral voting funds for all document create transitions in the batch.
    /// An erase prefunds nothing.
    pub fn all_conflicting_index_collateral_voting_funds(
        &self,
    ) -> Result<Option<Credits>, ProtocolError> {
        let (total, any_voting_funds): (Option<Credits>, bool) = self
            .transitions
            .iter()
            .filter_map(|transition| match transition {
                BatchedTransitionActionV1::Batched(BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::CreateAction(document_create_transition_action),
                )) => document_create_transition_action
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
            )),
            _ => Ok(None),
        }
    }

    /// The security levels a key must have to sign this batch: the highest
    /// level any document type or token action in the batch requires.
    ///
    /// An erase acts on a document type like any other document action, so it
    /// contributes that type's requirement.
    pub fn combined_security_level_requirement(&self) -> Result<Vec<SecurityLevel>, ProtocolError> {
        let mut highest_security_level = SecurityLevel::lowest_level();

        for transition in self.transitions.iter() {
            let document_security_level = match transition {
                BatchedTransitionActionV1::Batched(BatchedTransitionAction::DocumentAction(
                    document_transition,
                )) => {
                    let document_type_name = document_transition.base().document_type_name();
                    let data_contract_info = document_transition.base().data_contract_fetch_info();

                    data_contract_info
                        .contract
                        .document_type_for_name(document_type_name)?
                        .security_level_requirement()
                }
                BatchedTransitionActionV1::DocumentErase(erase_transition) => {
                    let document_type_name = erase_transition.base().document_type_name();
                    let data_contract_info = erase_transition.base().data_contract_fetch_info();

                    data_contract_info
                        .contract
                        .document_type_for_name(document_type_name)?
                        .security_level_requirement()
                }
                BatchedTransitionActionV1::Batched(BatchedTransitionAction::TokenAction(_)) => {
                    SecurityLevel::CRITICAL
                }
                BatchedTransitionActionV1::Batched(
                    BatchedTransitionAction::BumpIdentityDataContractNonce(_),
                ) => continue,
            };

            // lower enum representation means higher in security
            if document_security_level < highest_security_level {
                highest_security_level = document_security_level
            }
        }

        Ok(if highest_security_level == SecurityLevel::MASTER {
            vec![SecurityLevel::MASTER]
        } else {
            // this might seem wrong until you realize that master is 0, critical 1, etc
            (SecurityLevel::CRITICAL as u8..=highest_security_level as u8)
                .map(|security_level| {
                    SecurityLevel::try_from(security_level).map_err(|_| {
                        ProtocolError::CorruptedCodeExecution(format!(
                            "security level {security_level} is between two known levels"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        })
    }
}
