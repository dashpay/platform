use bincode::{Decode, Encode};
use derive_more::From;
use dpp::platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod transformer;
mod v0;

use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::DataContract;
pub use v0::*;

#[derive(Debug, Clone, From)]
pub enum DocumentBaseTransitionAction {
    V0(DocumentBaseTransitionActionV0),
}

impl DocumentBaseTransitionActionAccessorsV0 for DocumentBaseTransitionAction {
    fn id(&self) -> Identifier {
        match self {
            DocumentBaseTransitionAction::V0(v0) => v0.id,
        }
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
}
