mod v0;

pub use v0::*;

use crate::shielded::SerializedAction;
use crate::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_value::Identifier;

impl IdentityCreateFromShieldedPoolTransitionAccessorsV0
    for IdentityCreateFromShieldedPoolTransition
{
    fn actions(&self) -> &[SerializedAction] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => &v0.actions,
        }
    }

    fn public_keys(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => &v0.public_keys,
        }
    }

    fn denomination(&self) -> u64 {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.denomination,
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.identity_id,
        }
    }
}
