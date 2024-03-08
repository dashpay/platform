use derive_more::From;
use dpp::platform_value::Identifier;

use dpp::prelude::{UserFeeMultiplier, IdentityNonce};

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

/// bump identity nonce action
#[derive(Debug, Clone, From)]
pub enum BumpIdentityNonceAction {
    /// v0
    V0(BumpIdentityNonceActionV0),
}

impl BumpIdentityNonceActionAccessorsV0 for BumpIdentityNonceAction {
    fn identity_id(&self) -> Identifier {
        match self {
            BumpIdentityNonceAction::V0(v0) => v0.identity_id,
        }
    }

    fn identity_nonce(&self) -> IdentityNonce {
        match self {
            BumpIdentityNonceAction::V0(v0) => v0.identity_nonce,
        }
    }

    fn fee_multiplier(&self) -> UserFeeMultiplier {
        match self {
            BumpIdentityNonceAction::V0(transition) => transition.fee_multiplier,
        }
    }
}
