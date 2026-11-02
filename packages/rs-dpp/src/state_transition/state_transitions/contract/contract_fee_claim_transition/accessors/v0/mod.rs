use crate::data_contract::document_type::action_fees::ContractFeePot;
use crate::prelude::{Identifier, IdentityNonce};

pub trait ContractFeeClaimTransitionAccessorsV0 {
    fn set_owner_id(&mut self, id: Identifier);
    fn set_data_contract_id(&mut self, id: Identifier);
    /// The contract whose pot is paid out
    fn data_contract_id(&self) -> Identifier;
    fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce);
    /// The signer's nonce for the contract
    fn identity_contract_nonce(&self) -> IdentityNonce;
    fn set_pot(&mut self, pot: ContractFeePot);
    /// The pot that is paid out
    fn pot(&self) -> ContractFeePot;
}
