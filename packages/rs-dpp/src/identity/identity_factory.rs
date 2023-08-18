use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::{Identity, IdentityPublicKey, IdentityV0, KeyID, TimestampMillis};

use crate::ProtocolError;

use dashcore::{InstantLock, Transaction};
use std::collections::BTreeMap;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::accessors::IdentityGettersV0;
#[cfg(all(feature = "validation", feature = "identity-value-conversion"))]
use crate::identity::conversion::platform_value::IdentityPlatformValueConversionMethodsV0;
use crate::serialization::PlatformDeserializable;
use crate::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::version::PlatformVersion;
use platform_value::{Identifier, Value};
use platform_version::TryIntoPlatformVersioned;

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

    #[cfg(feature = "identity-value-conversion")]
    pub fn create_from_object(
        &self,
        raw_identity: Value,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        #[cfg(feature = "validation")]
        if !skip_validation {
            self.validate_identity(&raw_identity)?;
        }
        raw_identity.try_into_platform_versioned(PlatformVersion::get(self.protocol_version)?)
    }

    #[cfg(all(feature = "identity-serialization", feature = "client"))]
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(all(feature = "validation", feature = "identity-value-conversion"))] skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        let identity: Identity = Identity::deserialize_no_limit(&buffer).map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
        })?;

        #[cfg(all(feature = "validation", feature = "identity-value-conversion"))]
        if !skip_validation {
            self.validate_identity(&identity.to_cleaned_object()?)?;
        }

        Ok(identity)
    }

    //todo: this should be changed into identity.validate()
    #[cfg(feature = "validation")]
    pub fn validate_identity(&self, _raw_identity: &Value) -> Result<(), ProtocolError> {
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
        identity: Identity,
        asset_lock_proof: AssetLockProof,
        platform_version: &PlatformVersion,
    ) -> Result<IdentityCreateTransition, ProtocolError> {
        let transition = IdentityCreateTransitionV0::try_from_identity(
            identity,
            asset_lock_proof,
            platform_version,
        )?;

        Ok(IdentityCreateTransition::V0(transition))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_with_create_transition(
        &self,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        asset_lock_proof: AssetLockProof,
        platform_version: &PlatformVersion,
    ) -> Result<(Identity, IdentityCreateTransition), ProtocolError> {
        let identifier = asset_lock_proof.create_identifier()?;
        let identity = Identity::V0(IdentityV0 {
            id: identifier,
            public_keys: public_keys.clone(),
            balance: 0,
            revision: 0,
        });

        let mut identity_create_transition =
            IdentityCreateTransition::V0(IdentityCreateTransitionV0::try_from_identity(
                identity.clone(),
                asset_lock_proof,
                platform_version,
            )?);
        Ok((identity, identity_create_transition))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_topup_transition(
        &self,
        identity_id: Identifier,
        asset_lock_proof: AssetLockProof,
    ) -> Result<IdentityTopUpTransition, ProtocolError> {
        let mut identity_topup_transition = IdentityTopUpTransitionV0::default();

        identity_topup_transition.set_identity_id(identity_id);
        identity_topup_transition.set_asset_lock_proof(asset_lock_proof);

        Ok(IdentityTopUpTransition::V0(identity_topup_transition))
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    pub fn create_identity_credit_transfer_transition(
        &self,
        identity_id: Identifier,
        recipient_id: Identifier,
        amount: u64,
    ) -> Result<IdentityCreditTransferTransition, ProtocolError> {
        let mut identity_credit_transfer_transition = IdentityCreditTransferTransitionV0::default();
        identity_credit_transfer_transition.identity_id = identity_id;
        identity_credit_transfer_transition.recipient_id = recipient_id;
        identity_credit_transfer_transition.amount = amount;

        Ok(IdentityCreditTransferTransition::from(
            identity_credit_transfer_transition,
        ))
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
        let mut identity_update_transition = IdentityUpdateTransitionV0::default();
        identity_update_transition.set_identity_id(identity.id().to_owned());
        identity_update_transition.set_revision(identity.revision() + 1);

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

        Ok(IdentityUpdateTransition::V0(identity_update_transition))
    }
}
