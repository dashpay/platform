use crate::drive::contract::DataContractFetchInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::util::hash::hash_double;
use std::sync::Arc;

/// transformer
pub mod transformer;

/// Token base transition action v0
#[derive(Debug, Clone)]
pub struct TokenBaseTransitionActionV0 {
    /// The identity contract nonce, used to prevent replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// The token position within the data contract
    pub token_position: u16,
    /// A potential data contract
    pub data_contract: Arc<DataContractFetchInfo>,
}

/// Token base transition action accessors v0
pub trait TokenBaseTransitionActionAccessorsV0 {
    /// The token position within the data contract
    fn token_position(&self) -> u16;

    /// The token id
    fn token_id(&self) -> Identifier {
        // Prepare the data for hashing
        let mut bytes = b"token".to_vec();
        bytes.extend_from_slice(self.data_contract_id().as_bytes());
        bytes.extend_from_slice(&self.token_position().to_be_bytes());
        hash_double(bytes).into()
    }

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
    fn token_position(&self) -> u16 {
        self.token_position
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
