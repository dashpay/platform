use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};

use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, SignatureError,
};
use crate::consensus::ConsensusError;
use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey};

use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::version::FeatureVersion;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::{Identifier, Revision, TimestampMillis},
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    version::LATEST_VERSION,
    ProtocolError,
};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

pub trait IdentityUpdateTransitionV0Methods {
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityUpdate
    }
    fn set_identity_id(&mut self, id: Identifier);
    fn get_identity_id(&self) -> &Identifier;
    fn set_revision(&mut self, revision: Revision);
    fn get_revision(&self) -> Revision;
    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>);
    fn get_public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation];
    fn get_public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation];
    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>);
    fn get_public_key_ids_to_disable(&self) -> &[KeyID];
    fn set_public_keys_disabled_at(&mut self, public_keys_disabled_at: Option<TimestampMillis>);
    fn get_public_keys_disabled_at(&self) -> Option<TimestampMillis>;
    fn get_owner_id(&self) -> &Identifier;
}

impl IdentityUpdateTransitionV0Methods for IdentityUpdateTransitionV0 {
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        signer: &S,
        _version: FeatureVersion,
    ) -> Result<Self, ProtocolError> {
        let add_public_keys_in_creation = add_public_keys
            .iter()
            .map(|public_key| public_key.into())
            .collect();

        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id,
            revision: identity.revision,
            add_public_keys: add_public_keys_in_creation,
            disable_public_keys,
            public_keys_disabled_at,
        };

        let key_signable_bytes = identity_update_transition.signable_bytes()?;

        // Sign all the keys
        identity_update_transition
            .add_public_keys
            .iter_mut()
            .zip(add_public_keys.iter())
            .try_for_each(|(public_key_with_witness, public_key)| {
                if public_key.key_type.is_unique_key_type() {
                    let signature = signer.sign(public_key, &key_signable_bytes)?;
                    public_key_with_witness.signature = signature;
                }

                Ok::<(), ProtocolError>(())
            })?;

        let master_public_key = identity
            .public_keys
            .get(master_public_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(
                    *master_public_key_id,
                ))
                .into(),
            )?;
        if master_public_key.security_level != SecurityLevel::MASTER {
            Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    master_public_key.security_level,
                    vec![SecurityLevel::MASTER],
                ),
            ))
        } else {
            identity_update_transition.sign_external(master_public_key, signer)?;
            Ok(identity_update_transition)
        }
    }

    fn set_identity_id(&mut self, id: Identifier) {
        self.identity_id = id;
    }

    fn get_identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn get_revision(&self) -> Revision {
        self.revision
    }

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.add_public_keys = add_public_keys;
    }

    fn get_public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        &self.add_public_keys
    }

    fn get_public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        &mut self.add_public_keys
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        self.disable_public_keys = disable_public_keys;
    }

    fn get_public_key_ids_to_disable(&self) -> &[KeyID] {
        &self.disable_public_keys
    }

    fn set_public_keys_disabled_at(&mut self, public_keys_disabled_at: Option<TimestampMillis>) {
        self.public_keys_disabled_at = public_keys_disabled_at;
    }

    fn get_public_keys_disabled_at(&self) -> Option<TimestampMillis> {
        self.public_keys_disabled_at
    }

    fn get_owner_id(&self) -> &Identifier {
        &self.identity_id
    }
}
