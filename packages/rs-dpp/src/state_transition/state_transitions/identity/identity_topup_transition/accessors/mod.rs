mod v0;

pub use v0::*;

use crate::identity::signer::Signer;
use crate::identity::{Identity, KeyID, PartialIdentity};
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::version::FeatureVersion;
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::{Bytes32, Identifier};

impl IdentityTopUpTransitionAccessorsV0 for IdentityTopUpTransition {
    fn set_asset_lock_proof(&mut self, asset_lock_proof: AssetLockProof) {
        match self {
            IdentityTopUpTransition::V0(transition) => {
                transition.set_asset_lock_proof(asset_lock_proof)
            }
        }
    }

    fn asset_lock_proof(&self) -> &AssetLockProof {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.asset_lock_proof(),
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.set_identity_id(identity_id),
        }
    }

    fn identity_id(&self) -> &Identifier {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.identity_id(),
        }
    }
}
