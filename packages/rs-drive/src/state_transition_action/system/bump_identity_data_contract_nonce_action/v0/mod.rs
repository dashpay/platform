/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;

#[derive(Debug, Clone)]
/// Version 0 of the bump identity data contract nonce action
/// This action is performed when we want to pay for the state transition
pub struct BumpIdentityDataContractNonceActionV0 {
    /// The identity id
    pub identity_id: Identifier,
    /// The contract id
    pub data_contract_id: Identifier,
    /// The identity contract nonce, this is used to stop replay attacks
    pub identity_contract_nonce: IdentityNonce,
}

/// document base transition action accessors v0
pub trait BumpIdentityDataContractNonceActionAccessorsV0 {
    /// The identity id
    fn identity_id(&self) -> Identifier;
    /// The contract id
    fn data_contract_id(&self) -> Identifier;
    /// Identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;
}
