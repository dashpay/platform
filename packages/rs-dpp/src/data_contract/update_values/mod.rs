use std::collections::BTreeMap;

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::{GroupContractPosition, TokenContractPosition};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use platform_value::Value;

/// Represents the values to apply to a data contract during an update.
#[derive(Debug)]
pub struct DataContractUpdateValues<'a> {
    /// The new revision number for the contract.
    pub revision: u32,
    /// Updated schema definitions (must be compatible with existing ones).
    pub updated_schema_defs: &'a BTreeMap<String, Value>,
    /// New schema definitions to add.
    pub new_schema_defs: &'a BTreeMap<String, Value>,
    /// Updates to existing document schemas (keyed by document type name).
    pub updated_document_schemas: &'a BTreeMap<String, Value>,
    /// New document schemas to add (keyed by document type name).
    pub new_document_schemas: &'a BTreeMap<String, Value>,
    /// New groups to add to the contract.
    pub new_groups: &'a BTreeMap<GroupContractPosition, Group>,
    /// New tokens to add to the contract.
    pub new_tokens: &'a BTreeMap<TokenContractPosition, TokenConfiguration>,
    /// Keywords to remove from the contract.
    pub remove_keywords: &'a [String],
    /// Keywords to add to the contract.
    pub add_keywords: &'a [String],
    /// Update to the contract description. None means no change, Some(None) clears it,
    /// Some(Some(desc)) sets a new description.
    pub update_description: &'a Option<Option<String>>,
}

impl<'a> From<&'a DataContractUpdateTransitionV1> for DataContractUpdateValues<'a> {
    fn from(value: &'a DataContractUpdateTransitionV1) -> Self {
        DataContractUpdateValues {
            revision: value.revision,
            updated_schema_defs: &value.updated_schema_defs,
            new_schema_defs: &value.new_schema_defs,
            updated_document_schemas: &value.updated_document_schemas,
            new_document_schemas: &value.new_document_schemas,
            new_groups: &value.new_groups,
            new_tokens: &value.new_tokens,
            remove_keywords: &value.remove_keywords,
            add_keywords: &value.add_keywords,
            update_description: &value.update_description,
        }
    }
}
