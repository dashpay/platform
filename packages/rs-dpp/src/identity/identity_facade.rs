use dashcore::{InstantLock, Transaction};
use platform_value::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::identity::factory::IdentityFactory;
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyCreateTransition;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Identifier;

use crate::validation::ValidationResult;
use crate::version::ProtocolVersionValidator;
use crate::{BlsModule, DashPlatformProtocolInitError, NonConsensusError, ProtocolError};

#[derive(Clone)]
pub struct IdentityFacade<T: BlsModule> {
    identity_validator: Arc<IdentityValidator<PublicKeysValidator<T>>>,
    factory: IdentityFactory<T>,
}

impl<T> IdentityFacade<T>
where
    T: BlsModule,
{
    pub fn new(
        protocol_version: u32,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        public_keys_validator: Arc<PublicKeysValidator<T>>,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let identity_validator = Arc::new(IdentityValidator::new(
            protocol_version_validator,
            public_keys_validator,
        )?);

        Ok(Self {
            identity_validator: identity_validator.clone(),
            factory: IdentityFactory::new(protocol_version, identity_validator),
        })
    }

    pub fn create(
        &self,
        asset_lock_proof: AssetLockProof,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create(asset_lock_proof, public_keys)
    }

    pub fn create_from_object(
        &self,
        raw_identity: Value,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        self.factory
            .create_from_object(raw_identity, skip_validation)
    }

    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create_from_buffer(buffer, skip_validation)
    }

    pub fn validate(
        &self,
        identity_object: &Value,
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        self.identity_validator.validate_identity(identity_object)
    }

    pub fn create_instant_lock_proof(
        instant_lock: InstantLock,
        asset_lock_transaction: Transaction,
        output_index: u32,
    ) -> InstantAssetLockProof {
        IdentityFactory::<T>::create_instant_lock_proof(
            instant_lock,
            asset_lock_transaction,
            output_index,
        )
    }

    pub fn create_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    ) -> ChainAssetLockProof {
        IdentityFactory::<T>::create_chain_asset_lock_proof(core_chain_locked_height, out_point)
    }

    pub fn create_identity_create_transition(
        &self,
        identity: Identity,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        self.factory.create_identity_create_transition(identity)
    }

    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        self.factory
            .create_identity_topup_transition(identity_id, asset_lock_proof)
    }

    pub fn create_identity_update_transition(
        &self,
        identity: Identity,
        add_public_keys: Option<Vec<IdentityPublicKeyCreateTransition>>,
        public_key_ids_to_disable: Option<Vec<KeyID>>,
        // Pass disable time as argument because SystemTime::now() does not work for wasm target
        // https://github.com/rust-lang/rust/issues/48564
        disable_time: Option<TimestampMillis>,
    ) -> Result<IdentityUpdateTransition, ProtocolError> {
        self.factory.create_identity_update_transition(
            identity,
            add_public_keys,
            public_key_ids_to_disable,
            disable_time,
        )
    }
}
