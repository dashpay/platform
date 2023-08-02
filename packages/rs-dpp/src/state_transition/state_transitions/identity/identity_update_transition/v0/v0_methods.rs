use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};
use platform_version::version::PlatformVersion;

use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, SignatureError,
};
use crate::consensus::ConsensusError;
use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey};

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::{StateTransition, StateTransitionIdentitySigned};
use crate::version::FeatureVersion;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::{Identifier, Revision, TimestampMillis},
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    version::LATEST_VERSION,
    ProtocolError,
};

impl IdentityUpdateTransitionMethodsV0 for IdentityUpdateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        signer: &S,
        _platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let add_public_keys_in_creation = add_public_keys
            .iter()
            .map(|public_key| public_key.into())
            .collect();

        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: identity.revision(),
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
                if public_key.key_type().is_unique_key_type() {
                    let signature = signer.sign(public_key, &key_signable_bytes)?;
                    public_key_with_witness.set_signature(signature);
                }

                Ok::<(), ProtocolError>(())
            })?;

        let master_public_key = identity
            .public_keys()
            .get(master_public_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(
                    *master_public_key_id,
                ))
                .into(),
            )?;
        if master_public_key.security_level() != SecurityLevel::MASTER {
            Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    master_public_key.security_level(),
                    vec![SecurityLevel::MASTER],
                ),
            ))
        } else {
            let mut state_transition: StateTransition = identity_update_transition.into();
            state_transition.sign_external(master_public_key, signer)?;
            Ok(state_transition)
        }
    }
}

impl IdentityUpdateTransitionAccessorsV0 for IdentityUpdateTransitionV0 {
    fn set_identity_id(&mut self, id: Identifier) {
        self.identity_id = id;
    }

    fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.add_public_keys = add_public_keys;
    }

    fn public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        &self.add_public_keys
    }

    fn public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        &mut self.add_public_keys
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        self.disable_public_keys = disable_public_keys;
    }

    fn public_key_ids_to_disable(&self) -> &[KeyID] {
        &self.disable_public_keys
    }

    fn set_public_keys_disabled_at(&mut self, public_keys_disabled_at: Option<TimestampMillis>) {
        self.public_keys_disabled_at = public_keys_disabled_at;
    }

    fn public_keys_disabled_at(&self) -> Option<TimestampMillis> {
        self.public_keys_disabled_at
    }

    fn owner_id(&self) -> Identifier {
        self.identity_id
    }
}
