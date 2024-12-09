use std::sync::Arc;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use crate::drive::contract::DataContractFetchInfo;

/// transformer
pub mod transformer;

/// Token base transition action v0
#[derive(Debug, Clone)]
pub struct TokenBaseTransitionActionV0 {
    /// The token transition ID
    pub id: Identifier,
    /// The identity contract nonce, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// The token ID within the data contract
    pub token_id: u16,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
}

/// Token base transition action accessors v0
pub trait TokenBaseTransitionActionAccessorsV0 {
    /// Returns the token transition ID
    fn id(&self) -> Identifier;

    /// The token ID within the data contract
    fn token_id(&self) -> u16;

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier;

    /// Returns a reference to the data contract fetch info, without cloning
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo>;

    /// Returns the data contract fetch info (cloned Arc)
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo>;

    /// Returns the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;
}

impl TokenBaseTransitionActionAccessorsV0 for TokenBaseTransitionActionV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn token_id(&self) -> u16 {
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
}