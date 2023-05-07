use crate::identity::identity_public_key::factory::KeyCount;
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Identifier;

use crate::{BlsModule, Convertible, ProtocolError};

use dashcore::{InstantLock, Transaction};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::iter::FromIterator;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::serialization_traits::PlatformDeserializable;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::version::LATEST_PLATFORM_VERSION;

use platform_value::Value;
use std::sync::Arc;
use crate::identity::v0::identity::Identity;

pub const IDENTITY_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone)]
pub struct IdentityFactory<T: BlsModule> {
    protocol_version: u32,
    identity_validator: Arc<IdentityValidator<PublicKeysValidator<T>>>,
}

impl<T> IdentityFactory<T>
where
    T: BlsModule,
{
    pub fn new(
        protocol_version: u32,
        identity_validator: Arc<IdentityValidator<PublicKeysValidator<T>>>,
    ) -> Self {
        IdentityFactory {
            protocol_version,
            identity_validator,
        }
    }

    pub fn create(
        &self,
        asset_lock_proof: AssetLockProof,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Identity, ProtocolError> {
        let identity = Identity {
            feature_version: LATEST_PLATFORM_VERSION.identity.default_current_version,
            id: asset_lock_proof.create_identifier()?,
            balance: 0,
            public_keys,
            revision: 0,
            asset_lock_proof: Some(asset_lock_proof),
            metadata: None,
        };

        Ok(identity)
    }

    pub fn create_from_object(
        &self,
        raw_identity: Value,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        if !skip_validation {
            self.validate_identity(&raw_identity)?;
        }

        Identity::from_object(raw_identity)
    }

    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        let identity: Identity = Identity::deserialize(&buffer).map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
        })?;

        if !skip_validation {
            self.validate_identity(&identity.to_cleaned_object()?)?;
        }

        Ok(identity)
    }

    pub fn validate_identity(&self, raw_identity: &Value) -> Result<(), ProtocolError> {
        let result = self
            .identity_validator
            .validate_identity_object(raw_identity)?;

        if !result.is_valid() {
            return Err(ProtocolError::InvalidIdentityError {
                errors: result.errors,
                raw_identity: raw_identity.to_owned(),
            });
        }

        Ok(())
    }

    pub fn create_instant_lock_proof(
        instant_lock: InstantLock,
        asset_lock_transaction: Transaction,
        output_index: u32,
    ) -> InstantAssetLockProof {
        InstantAssetLockProof::new(instant_lock, asset_lock_transaction, output_index)
    }

    pub fn create_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    ) -> ChainAssetLockProof {
        ChainAssetLockProof::new(core_chain_locked_height, out_point)
    }

    pub fn create_identity_create_transition(
        &self,
        identity: Identity,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        let mut identity_create_transition: IdentityCreateTransition = identity.try_into()?;
        identity_create_transition.set_protocol_version(self.protocol_version);
        Ok(identity_create_transition)
    }

    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        let mut identity_topup_transition = IdentityTopUpTransition::default();
        identity_topup_transition.set_protocol_version(self.protocol_version);
        identity_topup_transition.set_identity_id(identity_id);

        identity_topup_transition
            .set_asset_lock_proof(asset_lock_proof)
            .map_err(ProtocolError::from)?;

        Ok(identity_topup_transition)
    }

    pub fn create_identity_update_transition(
        &self,
        identity: Identity,
        add_public_keys: Option<Vec<IdentityPublicKeyInCreation>>,
        public_key_ids_to_disable: Option<Vec<KeyID>>,
        // Pass disable time as argument because SystemTime::now() does not work for wasm target
        // https://github.com/rust-lang/rust/issues/48564
        disable_time: Option<TimestampMillis>,
    ) -> Result<IdentityUpdateTransition, ProtocolError> {
        let mut identity_update_transition = IdentityUpdateTransition::default();
        identity_update_transition.set_protocol_version(self.protocol_version);
        identity_update_transition.set_identity_id(identity.get_id().to_owned());
        identity_update_transition.set_revision(identity.get_revision() + 1);

        if let Some(add_public_keys) = add_public_keys {
            identity_update_transition.set_public_keys_to_add(add_public_keys);
        }

        if let Some(public_key_ids_to_disable) = public_key_ids_to_disable {
            if disable_time.is_none() {
                return Err(ProtocolError::Generic(
                    "Public keys disabled at must be present".to_string(),
                ));
            }

            identity_update_transition.set_public_key_ids_to_disable(public_key_ids_to_disable);
            identity_update_transition.set_public_keys_disabled_at(disable_time);
        }

        Ok(identity_update_transition)
    }
}
