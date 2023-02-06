use crate::identifier::Identifier;
use crate::identity::identity_public_key::factory::KeyCount;
use crate::identity::{Identity, IdentityPublicKey};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

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
            asset_lock_proof: None,
            balance,
            public_keys,
            metadata: None,
        }
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
}
