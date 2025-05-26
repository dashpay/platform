use crate::data_contract::TokenContractPosition;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Identifier;

#[derive(Debug, Clone, Encode, Decode, From, PartialEq)]
/// Token contract info
pub struct TokenContractInfoV0 {
    /// The contract id of the token
    pub contract_id: Identifier,
    /// The token contract position
    pub token_contract_position: TokenContractPosition,
}

/// Accessor trait for `TokenContractInfoV0`
pub trait TokenContractInfoV0Accessors {
    /// Returns a reference to the contract ID.
    fn contract_id(&self) -> Identifier;

    /// Sets the contract ID.
    fn set_contract_id(&mut self, contract_id: Identifier);

    /// Returns the token contract position.
    fn token_contract_position(&self) -> TokenContractPosition;

    /// Sets the token contract position.
    fn set_token_contract_position(&mut self, position: TokenContractPosition);
}

impl TokenContractInfoV0Accessors for TokenContractInfoV0 {
    fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    fn set_contract_id(&mut self, contract_id: Identifier) {
        self.contract_id = contract_id;
    }

    fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }

    fn set_token_contract_position(&mut self, position: TokenContractPosition) {
        self.token_contract_position = position;
    }
}
