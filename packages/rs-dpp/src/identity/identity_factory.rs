use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Identifier;

use crate::{BlsModule, Convertible, ProtocolError};

use dashcore::{InstantLock, Transaction};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::iter::FromIterator;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::serialization::PlatformDeserializable;

use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::identity::v0::IdentityV0;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
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
use crate::version::PlatformVersion;
use platform_value::Value;
use std::sync::Arc;

pub const IDENTITY_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone)]
pub struct IdentityFactory {
    protocol_version: u32,
}

impl IdentityFactory {
    pub fn new(protocol_version: u32) -> Self {
        IdentityFactory { protocol_version }
    }

    pub fn create(
        &self,
        asset_lock_proof: AssetLockProof,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    ) -> Result<Identity, ProtocolError> {
        Identity::new_with_asset_lock_and_keys(
            asset_lock_proof,
            public_keys,
            PlatformVersion::get(self.protocol_version)?,
        )
    }

    pub fn create_from_object(
        &self,
        raw_identity: Value,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        #[cfg(feature = "validation")]
        if !skip_validation {
            self.validate_identity(&raw_identity)?;
        }

        Identity::try_from_owned_value(raw_identity, PlatformVersion::get(self.protocol_version)?)
    }

    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        let identity: Identity = Identity::deserialize_no_limit(&buffer).map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
        })?;

        #[cfg(feature = "validation")]
        if !skip_validation {
            self.validate_identity(&identity.to_cleaned_object()?)?;
        }

        Ok(identity)
    }

    #[cfg(feature = "validation")]
    pub fn validate_identity(&self, raw_identity: &Value) -> Result<(), ProtocolError> {
        //todo: reenable
        // let result = self
        //     .identity_validator
        //     .validate_identity_object(raw_identity)?;
        //
        // if !result.is_valid() {
        //     return Err(ProtocolError::InvalidIdentityError {
        //         errors: result.errors,
        //         raw_identity: raw_identity.to_owned(),
        //     });
        // }

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

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_create_transition(
        &self,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        let mut identity_create_transition: IdentityCreateTransition = identity.try_into()?;
        Ok(identity_create_transition)
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_with_create_transition(
        &self,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        asset_lock_proof: AssetLockProof,
    ) -> Result<(Identity, IdentityCreateTransition), ProtocolError> {
        let identifier = asset_lock_proof.create_identifier()?;
        let identity = IdentityV0 {
            id: identifier,
            public_keys: public_keys.clone(),
            balance: 0,
            revision: 0,
        };

        let identity = IdentityCreateTransitionV0 {
            public_keys: public_keys.into_iter().map(|(_, values)| values).collect(),
            asset_lock_proof,
            signature: Default::default(),
            identity_id: identifier,
        };

        let mut identity_create_transition: IdentityCreateTransition = identity.try_into()?;
        Ok(identity_create_transition)
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        let mut identity_topup_transition = IdentityTopUpTransition::default();
        identity_topup_transition.set_identity_id(identity_id);

        identity_topup_transition
            .set_asset_lock_proof(asset_lock_proof)
            .map_err(ProtocolError::from)?;

        Ok(identity_topup_transition)
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_credit_transfer_transition(
        &self,
        identity_id: Identifier,
        recipient_id: Identifier,
        amount: u64,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        let mut identity_credit_transfer_transition = IdentityCreditTransferTransition::default();
        identity_credit_transfer_transition.set_protocol_version(self.protocol_version);
        identity_credit_transfer_transition.set_identity_id(identity_id);
        identity_credit_transfer_transition.set_recipient_id(recipient_id);
        identity_credit_transfer_transition.set_amount(amount);

        Ok(identity_credit_transfer_transition)
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
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
