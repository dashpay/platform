use dashcore::{InstantLock, Transaction};

use std::collections::BTreeMap;

use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::{Identity, IdentityPublicKey, KeyID};
use crate::prelude::{Identifier, IdentityNonce};

use crate::identity::identity_factory::IdentityFactory;
#[cfg(feature = "state-transitions")]
use crate::state_transition::{
    identity_create_transition::IdentityCreateTransition,
    identity_credit_transfer_transition::IdentityCreditTransferTransition,
    identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    identity_topup_transition::IdentityTopUpTransition,
    identity_update_transition::IdentityUpdateTransition,
    public_key_in_creation::IdentityPublicKeyInCreation,
};

use crate::identity::core_script::CoreScript;
use crate::withdrawal::Pooling;
use crate::ProtocolError;

#[derive(Clone)]
pub struct IdentityFacade {
    factory: IdentityFactory,
}

impl IdentityFacade {
    pub fn new(protocol_version: u32) -> Self {
        Self {
            factory: IdentityFactory::new(protocol_version),
        }
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

    #[cfg(all(feature = "identity-serialization", feature = "client"))]
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        self.factory.create_from_buffer(buffer, skip_validation)
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
        identity: &Identity,
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
        identity: &Identity,
        recipient_id: Identifier,
        amount: u64,
        identity_nonce: IdentityNonce,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        self.factory.create_identity_credit_transfer_transition(
            identity,
            recipient_id,
            amount,
            identity_nonce,
        )
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_credit_withdrawal_transition(
        &self,
        identity_id: Identifier,
        amount: u64,
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        identity_nonce: u64,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        self.factory.create_identity_credit_withdrawal_transition(
            identity_id,
            amount,
            core_fee_per_byte,
            pooling,
            output_script,
            identity_nonce,
        )
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_identity_update_transition(
        &self,
        identity: Identity,
        identity_nonce: u64,
        add_public_keys: Option<Vec<IdentityPublicKeyInCreation>>,
        public_key_ids_to_disable: Option<Vec<KeyID>>,
    ) -> Result<IdentityUpdateTransition, ProtocolError> {
        self.factory.create_identity_update_transition(
            identity,
            identity_nonce,
            add_public_keys,
            public_key_ids_to_disable,
        )
    }
}
