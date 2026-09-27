use derive_more::From;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::platform_value::Identifier;

use dpp::balances::credits::TokenAmount;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::prelude::IdentityNonce;
use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
use dpp::tokens::token_amount_on_contract_token::DocumentActionTokenEffect;
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

    fn gas_fees_paid_by(&self) -> GasFeesPaidBy {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.gas_fees_paid_by,
        }
    }

    fn contract_gas_fees_paid_by(&self) -> GasFeesPaidBy {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.contract_gas_fees_paid_by,
        }
    }

    fn declared_action_fee_with_agreement(&self) -> Option<DeclaredDocumentActionFee> {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.declared_action_fee.as_deref().copied(),
        }
    }

    fn agrees_to_a_moderators_discount(&self) -> bool {
        let Some(declared) = self.declared_action_fee_with_agreement() else {
            return false;
        };
        declared.agreement.is_some_and(|agreement| {
            agreement.discounts_moderators_of(declared.pricing, declared.fee)
        }) && self
            .data_contract_fetch_info_ref()
            .contract
            .config()
            .moderation()
            .and_then(|moderation| moderation.moderators.elected())
            .is_some_and(|elected| elected.moderates_document_type(self.document_type_name()))
    }
}
