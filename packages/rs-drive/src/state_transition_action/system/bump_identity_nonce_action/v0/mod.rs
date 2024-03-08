/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::prelude::{UserFeeIncrease, IdentityNonce};

#[derive(Debug, Clone)]
/// Version 0 of the bump identity nonce action
/// This action is performed when we want to pay for validation of the state transition
/// but not execute it
pub struct BumpIdentityNonceActionV0 {
    /// The identity id
    pub identity_id: Identifier,
    /// The identity contract nonce, this is used to stop replay attacks
    pub identity_nonce: IdentityNonce,
    /// fee multiplier
    pub fee_multiplier: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait BumpIdentityNonceActionAccessorsV0 {
    /// The identity id
    fn identity_id(&self) -> Identifier;
    /// Identity contract nonce
    fn identity_nonce(&self) -> IdentityNonce;

    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
}
