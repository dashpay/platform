/// transformer
pub mod transformer;

use std::collections::BTreeMap;

use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::group::Group;
use dpp::data_contract::{
    DataContract, DocumentName, GroupContractPosition, TokenContractPosition,
};
use dpp::platform_value::Value;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// Data contract update transition action v1.
/// This version is used for V1 state transitions that contain partial update information
/// and require fetching the old contract from state.
#[derive(Debug, Clone)]
pub struct DataContractUpdateTransitionActionV1 {
    /// The existing data contract before the update (fetched from state).
    pub old_data_contract: DataContract,
    /// The new data contract after applying the update.
    pub data_contract: DataContract,
    /// Identity contract nonce.
    pub identity_contract_nonce: IdentityNonce,
    /// Fee multiplier.
    pub user_fee_increase: UserFeeIncrease,
    /// Updated document schemas (for V1 transitions).
    /// These are schemas for existing document types that are being modified.
    pub updated_document_schemas: BTreeMap<DocumentName, Value>,
    /// New document schemas (for V1 transitions).
    /// These are schemas for new document types being added.
    pub new_document_schemas: BTreeMap<DocumentName, Value>,
    /// New groups being added (for V1 transitions).
    pub new_groups: BTreeMap<GroupContractPosition, Group>,
    /// New tokens being added (for V1 transitions).
    pub new_tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,
    /// Keywords to remove (for V1 transitions).
    pub remove_keywords: Vec<String>,
    /// Keywords to add (for V1 transitions).
    pub add_keywords: Vec<String>,
    /// Updated description (for V1 transitions).
    /// None = don't update, Some(None) = clear description, Some(Some(value)) = set new description.
    pub update_description: Option<Option<String>>,
}
