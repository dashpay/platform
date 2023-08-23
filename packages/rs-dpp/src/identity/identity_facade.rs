use dashcore::{InstantLock, Transaction};
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Identifier;

use crate::identity::identity_factory::IdentityFactory;
#[cfg(feature = "state-transitions")]
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::validation::SimpleConsensusValidationResult;
use crate::{BlsModule, DashPlatformProtocolInitError, NonConsensusError, ProtocolError};

#[derive(Clone)]
pub struct IdentityFacade {
    factory: IdentityFactory,
}

impl IdentityFacade {
    pub fn new(protocol_version: u32) -> Result<Self, DashPlatformProtocolInitError> {
        Ok(Self {
            factory: IdentityFactory::new(protocol_version),
        })
    }

    pub fn create(
        &self,
        id: Identifier,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create(id, public_keys)
    }

    // TODO(versioning): not used anymore?
    // pub fn create_from_object(
    //     &self,
    //     raw_identity: Value,
    //     skip_validation: bool,
    // ) -> Result<Identity, ProtocolError> {
    //     self.factory
    //         .create_from_object(raw_identity)
    // }

    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create_from_buffer(buffer, false)
    }

    pub fn create_instant_lock_proof(
        instant_lock: InstantLock,
        asset_lock_transaction: Transaction,
        output_index: u32,
    ) -> InstantAssetLockProof {
        IdentityFactory::create_instant_lock_proof(
            instant_lock,
            asset_lock_transaction,
            output_index,
        )
    }

    pub fn create_chain_asset_lock_proof(
        core_chain_locked_height: u32,
        out_point: [u8; 36],
    ) -> ChainAssetLockProof {
        IdentityFactory::create_chain_asset_lock_proof(core_chain_locked_height, out_point)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_create_transition(
        &self,
        identity: Identity,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        self.factory
            .create_identity_create_transition(identity, asset_lock_proof)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        self.factory
            .create_identity_topup_transition(identity_id, asset_lock_proof)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_credit_transfer_transition(
        &self,
        identity_id: Identifier,
        recipient_id: Identifier,
        amount: u64,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        self.factory
            .create_identity_credit_transfer_transition(identity_id, recipient_id, amount)
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_update_transition(
        &self,
        identity: Identity,
        add_public_keys: Option<Vec<IdentityPublicKeyInCreation>>,
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
