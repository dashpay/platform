use derive_more::From;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Identifier;

use crate::state_transition_action::shielded::ShieldedActionNote;
use crate::util::batch::drive_op_batch::TokenOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::TokenOperation;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::prelude::IdentityNonce;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
use dpp::tokens::token_payment_info::v1::TokenShieldedPayment;
use dpp::ProtocolError;
use std::sync::Arc;

/// transformer module
pub mod transformer;
mod v0;

use crate::drive::contract::DataContractFetchInfo;

pub use v0::*;

/// document base transition action
#[derive(Debug, Clone, From)]
pub enum DocumentBaseTransitionAction {
    /// v0
    V0(DocumentBaseTransitionActionV0),
}

impl DocumentBaseTransitionActionAccessorsV0 for DocumentBaseTransitionAction {
    fn id(&self) -> Identifier {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.id,
        }
    }

    fn document_type(&self) -> Result<DocumentTypeRef<'_>, ProtocolError> {
        Ok(self
            .data_contract_fetch_info_ref()
            .contract
            .document_type_for_name(self.document_type_name())?)
    }

    fn document_type_field_is_required(&self, field: &str) -> Result<bool, ProtocolError> {
        Ok(self.document_type()?.required_fields().contains(field))
    }

    fn document_type_name(&self) -> &String {
        match self {
            DocumentBaseTransitionAction::V0(v0) => &v0.document_type_name,
        }
    }

    fn document_type_name_owned(self) -> String {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.document_type_name,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.data_contract.contract.id(),
        }
    }

    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            DocumentBaseTransitionAction::V0(v0) => &v0.data_contract,
        }
    }
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.data_contract.clone(),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.identity_contract_nonce,
        }
    }

    fn token_cost(&self) -> Option<(Identifier, DocumentActionTokenEffect, TokenAmount)> {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.token_cost,
        }
    }

    fn shielded_token_payment(&self) -> Option<&TokenShieldedPayment> {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.shielded_token_payment.as_deref(),
        }
    }
}

impl DocumentBaseTransitionAction {
    /// The token operations that pay the document action's token cost, if it has one.
    ///
    /// Paid from the owner's balance: a transfer to the contract owner (skipped when the owner
    /// is the contract owner) or a burn. Paid from the token's shielded pool (`TokenPaymentInfo::V1`):
    /// the bundle's spent notes leave the pool and the cost goes into the contract owner's balance
    /// or out of the supply; the change notes join the pool.
    pub fn token_cost_operations<'b>(&self, owner_id: Identifier) -> Vec<DriveOperation<'b>> {
        let Some((token_id, effect, cost)) = self.token_cost() else {
            return vec![];
        };
        let contract_owner_id = self.data_contract_fetch_info_ref().contract.owner_id();
        if let Some(payment) = self.shielded_token_payment() {
            let nullifiers = payment
                .actions
                .iter()
                .map(|action| action.nullifier)
                .collect();
            let notes = payment
                .actions
                .iter()
                .map(ShieldedActionNote::from)
                .collect();
            return match effect {
                DocumentActionTokenEffect::TransferTokenToContractOwner => {
                    vec![TokenOperation(TokenOperationType::TokenUnshield {
                        token_id,
                        recipient_id: contract_owner_id,
                        amount: cost,
                        nullifiers,
                        notes,
                    })]
                }
                DocumentActionTokenEffect::BurnToken => {
                    vec![TokenOperation(TokenOperationType::TokenBurnFromPool {
                        token_id,
                        amount: cost,
                        nullifiers,
                        notes,
                    })]
                }
            };
        }
        match effect {
            DocumentActionTokenEffect::TransferTokenToContractOwner => {
                // If we are the owner, no need to send anything
                if owner_id != contract_owner_id {
                    vec![TokenOperation(TokenOperationType::TokenTransfer {
                        token_id,
                        sender_id: owner_id,
                        recipient_id: contract_owner_id,
                        amount: cost,
                    })]
                } else {
                    vec![]
                }
            }
            DocumentActionTokenEffect::BurnToken => {
                vec![TokenOperation(TokenOperationType::TokenBurn {
                    token_id,
                    identity_balance_holder_id: owner_id,
                    burn_amount: cost,
                })]
            }
        }
    }
}
