mod v0;

pub use v0::*;

use crate::identity::SecurityLevel;
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition::{
    StateTransitionIdentitySigned, StateTransitionLike, StateTransitionType,
};
use platform_value::Identifier;

impl IdentityCreditTransferTransitionMethodsV0 for IdentityCreditTransferTransition {
    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.security_level_requirement()
            }
        }
    }
}
