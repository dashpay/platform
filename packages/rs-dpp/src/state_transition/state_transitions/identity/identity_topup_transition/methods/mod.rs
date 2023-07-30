mod v0;

pub use v0::*;

use crate::identity::signer::Signer;
use crate::identity::{Identity, KeyID, PartialIdentity};
use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::version::FeatureVersion;
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::{Bytes32, Identifier};

impl IdentityTopUpTransitionMethodsV0 for IdentityTopUpTransition {
    #[cfg(feature = "state-transition-signing")]
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
}
