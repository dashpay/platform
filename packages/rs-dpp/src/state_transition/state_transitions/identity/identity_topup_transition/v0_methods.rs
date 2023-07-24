use crate::identity::signer::Signer;
use crate::identity::{Identity, KeyID, PartialIdentity};
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::v0::v0_methods::IdentityTopUpTransitionV0Methods;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::version::FeatureVersion;
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::{Bytes32, Identifier};

impl IdentityTopUpTransitionV0Methods for IdentityTopUpTransition {
    fn try_from_identity(
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        bls: &impl BlsModule,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        match version {
            0 => Ok(IdentityTopUpTransitionV0::try_from_identity(
                identity,
                asset_lock_proof,
                asset_lock_proof_private_key,
                bls,
                version,
            )?
            .into()),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityTopUpTransition version for try_from_identity {v}"
            ))),
        }
    }

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
