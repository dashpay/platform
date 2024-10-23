use crate::identity::v0::IdentityV0;
use crate::identity::{IdentityPublicKey, KeyID};
use crate::prelude::Identifier;

use crate::ProtocolError;

use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::KeyCount;
use crate::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;
use std::iter::FromIterator;

impl IdentityV0 {
    pub fn random_identity_with_rng(
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let id = Identifier::new(rng.gen::<[u8; 32]>());
        let revision = rng.gen_range(0..100);
        // balance must be in i64 (that would be >> 2)
        // but let's make it smaller
        let balance = rng.gen::<u64>() >> 20; //around 175 Dash as max
        let public_keys = IdentityPublicKey::random_authentication_keys_with_rng(
            key_count,
            rng,
            platform_version,
        )?
        .into_iter()
        .map(|key| (key.id(), key))
        .collect();

        Ok(IdentityV0 {
            id,
            revision,
            balance,
            public_keys,
        })
    }

    pub fn random_identity_with_main_keys_with_private_key<I>(
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, I), ProtocolError>
    where
        I: Default
            + IntoIterator<Item = (IdentityPublicKey, [u8; 32])>
            + Extend<(IdentityPublicKey, [u8; 32])>,
    {
        let id = Identifier::new(rng.gen::<[u8; 32]>());
        let revision = 0;
        // balance must be in i64 (that would be >> 2)
        // but let's make it smaller
        let balance = rng.gen::<u64>() >> 20; //around 175 Dash as max
        let (public_keys, private_keys): (BTreeMap<KeyID, IdentityPublicKey>, I) =
            IdentityPublicKey::main_keys_with_random_authentication_keys_with_private_keys_with_rng(
                key_count,
                rng,
                platform_version,
            )?
                .into_iter()
                .map(|(key, private_key)| ((key.id(), key.clone()), (key, private_key)))
                .unzip();

        Ok((
            IdentityV0 {
                id,
                revision,
                balance,
                public_keys,
            },
            private_keys,
        ))
    }

    pub fn random_identity(
        key_count: KeyCount,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_identity_with_rng(key_count, &mut rng, platform_version)
    }

    pub fn random_identities(
        count: u16,
        key_count: KeyCount,
        seed: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Self>, ProtocolError> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_identities_with_rng(count, key_count, &mut rng, platform_version)
    }

    pub fn random_identities_with_rng(
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Self>, ProtocolError> {
        let mut vec: Vec<IdentityV0> = vec![];
        for _i in 0..count {
            vec.push(Self::random_identity_with_rng(
                key_count,
                rng,
                platform_version,
            )?);
        }
        Ok(vec)
    }

    pub fn random_identities_with_private_keys_with_rng<I>(
        count: u16,
        key_count: KeyCount,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Self>, I), ProtocolError>
    where
        I: Default
            + FromIterator<(IdentityPublicKey, [u8; 32])>
            + Extend<(IdentityPublicKey, [u8; 32])>,
    {
        let mut vec: Vec<IdentityV0> = vec![];
        let mut private_key_map: Vec<(IdentityPublicKey, [u8; 32])> = vec![];
        for _i in 0..count {
            let (identity, mut map) = Self::random_identity_with_main_keys_with_private_key(
                key_count,
                rng,
                platform_version,
            )?;
            vec.push(identity);
            private_key_map.append(&mut map);
        }
        Ok((vec, private_key_map.into_iter().collect()))
    }
}
