use crate::drive::contract::DataContractFetchInfo;
use crate::error::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::ProtocolError;
use std::sync::Arc;

/// transformer
pub mod transformer;

/// A type for the original group action if one exists
pub type OriginalGroupAction = Option<GroupAction>;

/// Token base transition action v0
#[derive(Debug, Clone)]
pub struct TokenBaseTransitionActionV0 {
    /// Token Id
    pub token_id: Identifier,
    /// The identity contract nonce, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// The token position within the data contract
    pub token_contract_position: u16,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
    /// Using group multi party rules for authentication
    /// If this is set we should store in group
    pub store_in_group: Option<(GroupStateTransitionResolvedInfo, OriginalGroupAction)>,
    /// Should the action be performed.
    /// This is true if we don't store in group.
    /// And also true if we store in group and with this have enough signatures to perform the action
    pub perform_action: bool,
}

/// Token base transition action accessors v0
pub trait TokenBaseTransitionActionAccessorsV0 {
    /// The token position within the data contract
    fn token_position(&self) -> TokenContractPosition;

    /// The token id
    fn token_id(&self) -> Identifier;

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier;

    /// Returns a reference to the data contract fetch info, without cloning
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo>;

    /// Returns the data contract fetch info (cloned Arc)
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo>;

    /// Returns the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;

    /// Gets the token configuration associated to the action
    fn token_configuration(&self) -> Result<&TokenConfiguration, Error>;

    /// Gets the store_in_group field (optional)
    fn store_in_group(&self) -> Option<&GroupStateTransitionResolvedInfo>;

    /// Gets the original group action if we are in a group action, and we are not the proposer
    fn original_group_action(&self) -> Option<&GroupAction>;

    /// Gets the perform_action field
    fn perform_action(&self) -> bool;
}

impl TokenBaseTransitionActionAccessorsV0 for TokenBaseTransitionActionV0 {
    fn token_position(&self) -> u16 {
        self.token_contract_position
    }

    fn token_id(&self) -> Identifier {
        self.token_id
    }

    fn data_contract_id(&self) -> Identifier {
        self.data_contract.contract.id()
    }

    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        &self.data_contract
    }

    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.data_contract.clone()
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.identity_contract_nonce
    }

    fn token_configuration(&self) -> Result<&TokenConfiguration, Error> {
        self.data_contract
            .as_ref()
            .contract
            .tokens()
            .get(&self.token_contract_position)
            .ok_or(Error::Protocol(ProtocolError::CorruptedCodeExecution(
                format!(
                    "data contract does not have a token at position {}",
                    self.token_contract_position
                ),
            )))
    }

    fn store_in_group(&self) -> Option<&GroupStateTransitionResolvedInfo> {
        self.store_in_group.as_ref().map(|(s, _)| s)
    }

    fn original_group_action(&self) -> Option<&GroupAction> {
        self.store_in_group
            .as_ref()
            .map(|(_, group_action)| group_action.as_ref())
            .flatten()
    }

    fn perform_action(&self) -> bool {
        self.perform_action
    }
}
