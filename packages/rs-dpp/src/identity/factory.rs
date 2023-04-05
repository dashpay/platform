use crate::decode_protocol_entity_factory::DecodeProtocolEntity;
use crate::identity::identity_public_key::factory::KeyCount;
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::state_transition::asset_lock_proof::{AssetLockProof, InstantAssetLockProof};
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitness;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use crate::identity::validation::{IdentityValidator, PublicKeysValidator};
use crate::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use crate::prelude::Identifier;

use crate::{BlsModule, ProtocolError};

use dashcore::{InstantLock, Transaction};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;
use std::convert::TryInto;

use platform_value::Value;
use std::sync::Arc;

pub const IDENTITY_PROTOCOL_VERSION: u32 = 1;

impl Identity {
    // TODO: Move to a separate module under a feature
    pub fn random_identity_with_rng(key_count: KeyCount, rng: &mut StdRng) -> Self {
        let id = Identifier::new(rng.gen::<[u8; 32]>());
        let revision = rng.gen_range(0..100);
        // balance must be in i64 (that would be >> 2)
        // but let's make it smaller
        let balance = rng.gen::<u64>() >> 20; //around 175 Dash as max
        let public_keys = IdentityPublicKey::random_authentication_keys_with_rng(key_count, rng)
            .into_iter()
            .map(|key| (key.id, key))
            .collect();

        Identity {
            protocol_version: IDENTITY_PROTOCOL_VERSION,
            id,
            revision,
            asset_lock_proof: Some(AssetLockProof::Instant(InstantAssetLockProof::default())),
            balance,
            public_keys,
            metadata: None,
        }
    }

    // TODO: Move to a separate module under a feature
    pub fn random_identity_with_main_keys_with_private_key<I>(
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Result<(Self, I), ProtocolError>
    where
        I: Default
            + IntoIterator<Item = (IdentityPublicKey, Vec<u8>)>
            + Extend<(IdentityPublicKey, Vec<u8>)>,
    {
        let id = Identifier::new(rng.gen::<[u8; 32]>());
        let revision = rng.gen_range(0..100);
        // balance must be in i64 (that would be >> 2)
        // but let's make it smaller
        let balance = rng.gen::<u64>() >> 20; //around 175 Dash as max
        let (public_keys, private_keys): (BTreeMap<KeyID, IdentityPublicKey>, I) =
            IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
                key_count, rng,
            )?
            .into_iter()
            .map(|(key, private_key)| ((key.id, key.clone()), (key, private_key)))
            .unzip();

        Ok((
            Identity {
                protocol_version: IDENTITY_PROTOCOL_VERSION,
                id,
                revision,
                asset_lock_proof: Some(AssetLockProof::Instant(InstantAssetLockProof::default())),
                balance,
                public_keys,
                metadata: None,
            },
            private_keys,
        ))
    }

    // TODO: Move to a separate module under a feature
    pub fn random_identity(key_count: KeyCount, seed: Option<u64>) -> Self {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_identity_with_rng(key_count, &mut rng)
    }

    // TODO: Move to a separate module under a feature
    pub fn random_identities(count: u16, key_count: KeyCount, seed: Option<u64>) -> Vec<Self> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_identities_with_rng(count, key_count, &mut rng)
    }

    // TODO: Move to a separate module under a feature
    pub fn random_identities_with_rng(
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Vec<Self> {
        let mut vec: Vec<Identity> = vec![];
        for _i in 0..count {
            vec.push(Self::random_identity_with_rng(key_count, rng));
        }
        vec
    }

    // TODO: Move to a separate module under a feature
    pub fn random_identities_with_private_keys_with_rng(
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Result<(Vec<Self>, Vec<(IdentityPublicKey, Vec<u8>)>), ProtocolError> {
        let mut vec: Vec<Identity> = vec![];
        let mut private_key_map: Vec<(IdentityPublicKey, Vec<u8>)> = vec![];
        for _i in 0..count {
            let (identity, mut map) =
                Self::random_identity_with_main_keys_with_private_key(key_count, rng)?;
            vec.push(identity);
            private_key_map.append(&mut map);
        }
        Ok((vec, private_key_map))
    }
}

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
            protocol_version: self.protocol_version,
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
            let result = self
                .identity_validator
                .validate_identity_object(&raw_identity)?;

            if !result.is_valid() {
                return Err(ProtocolError::InvalidIdentityError {
                    errors: result.errors,
                    raw_identity,
                });
            }
        }

        Identity::from_object(raw_identity)
    }

    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<Identity, ProtocolError> {
        let (protocol_version, mut raw_identity) =
            DecodeProtocolEntity::decode_protocol_entity(buffer)?;
        raw_identity
            .set_value("protocolVersion", Value::U32(protocol_version))
            .map_err(ProtocolError::ValueError)?;

        // TODO: the error originates here due to id having a wrong type - should be a base58 for the schema

        self.create_from_object(raw_identity, skip_validation)
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
        add_public_keys: Option<Vec<IdentityPublicKeyWithWitness>>,
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
