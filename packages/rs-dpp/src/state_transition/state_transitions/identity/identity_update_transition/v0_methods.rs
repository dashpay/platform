use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey, KeyID, PartialIdentity};
use crate::prelude::{AssetLockProof, Revision, TimestampMillis};
use crate::state_transition::identity_update_transition::v0::v0_methods::IdentityUpdateTransitionV0Methods;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::version::FeatureVersion;
use crate::{BlsModule, NonConsensusError, ProtocolError};
use platform_value::{Bytes32, Identifier};

impl IdentityUpdateTransitionV0Methods for IdentityUpdateTransition {
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        match version {
            0 => Ok(IdentityUpdateTransitionV0::try_from_identity_with_signer(
                identity,
                master_public_key_id,
                add_public_keys,
                disable_public_keys,
                public_keys_disabled_at,
                signer,
                version,
            )?
            .into()),
            v => Err(ProtocolError::UnknownVersionError(format!(
                "Unknown IdentityUpdateTransition version for try_from_identity_with_signer {v}"
            ))),
        }
    }

    fn set_identity_id(&mut self, id: Identifier) {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.set_identity_id(id),
        }
    }

    fn get_identity_id(&self) -> &Identifier {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_identity_id(),
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.set_revision(revision),
        }
    }

    fn get_revision(&self) -> Revision {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_revision(),
        }
    }

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        match self {
            IdentityUpdateTransition::V0(transition) => {
                transition.set_public_keys_to_add(add_public_keys)
            }
        }
    }

    fn get_public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_public_keys_to_add(),
        }
    }

    fn get_public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_public_keys_to_add_mut(),
        }
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        match self {
            IdentityUpdateTransition::V0(transition) => {
                transition.set_public_key_ids_to_disable(disable_public_keys)
            }
        }
    }

    fn get_public_key_ids_to_disable(&self) -> &[KeyID] {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_public_key_ids_to_disable(),
        }
    }

    fn set_public_keys_disabled_at(&mut self, public_keys_disabled_at: Option<TimestampMillis>) {
        match self {
            IdentityUpdateTransition::V0(transition) => {
                transition.set_public_keys_disabled_at(public_keys_disabled_at)
            }
        }
    }

    fn get_public_keys_disabled_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_public_keys_disabled_at(),
        }
    }

    fn get_owner_id(&self) -> &Identifier {
        match self {
            IdentityUpdateTransition::V0(transition) => transition.get_owner_id(),
        }
    }
}
