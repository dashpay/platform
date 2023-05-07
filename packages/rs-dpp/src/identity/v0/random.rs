use crate::identity::v0::identity::IdentityV0;
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

impl IdentityV0 {
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
            feature_version: LATEST_PLATFORM_VERSION.identity.default_current_version,
            id,
            revision,
            asset_lock_proof: Default::default(),
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
        let revision = 0;
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
                feature_version: LATEST_PLATFORM_VERSION.identity.default_current_version,
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
        let mut vec: Vec<IdentityV0> = vec![];
        for _i in 0..count {
            vec.push(Self::random_identity_with_rng(key_count, rng));
        }
        vec
    }

    // TODO: Move to a separate module under a feature
    pub fn random_identities_with_private_keys_with_rng<I>(
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
    ) -> Result<(Vec<Self>, I), ProtocolError>
        where
            I: Default
            + FromIterator<(IdentityPublicKey, Vec<u8>)>
            + Extend<(IdentityPublicKey, Vec<u8>)>,
    {
        let mut vec: Vec<IdentityV0> = vec![];
        let mut private_key_map: Vec<(IdentityPublicKey, Vec<u8>)> = vec![];
        for _i in 0..count {
            let (identity, mut map) =
                Self::random_identity_with_main_keys_with_private_key(key_count, rng)?;
            vec.push(identity);
            private_key_map.append(&mut map);
        }
        Ok((vec, private_key_map.into_iter().collect()))
    }
}
