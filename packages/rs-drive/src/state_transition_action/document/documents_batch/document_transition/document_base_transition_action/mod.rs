use derive_more::From;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Identifier;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::prelude::IdentityNonce;
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

    fn document_type_field_is_required(&self, field: &str) -> Result<bool, ProtocolError> {
        Ok(self
            .data_contract_fetch_info()
            .contract
            .document_type_for_name(self.document_type_name())?
            .required_fields()
            .contains(field))
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
}
