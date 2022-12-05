use crate::identifier::Identifier;
use crate::identity::{Identity, IdentityPublicKey, CURRENT_IDENTITY_VERSION};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

impl Identity {
    pub fn random_identity_with_rng(key_count: u16, rng: &mut StdRng) -> Self {
        let id = Identifier::new(rng.gen::<[u8; 32]>());
        let revision = rng.gen::<u64>();
        let balance = rng.gen::<u64>();
        let public_keys = IdentityPublicKey::random_keys_with_rng(key_count, 96, rng)
            .into_iter()
            .map(|key| (key.id, key))
            .collect();

        Identity {
            version: CURRENT_IDENTITY_VERSION,
            id,
            revision,
            asset_lock_proof: None,
            balance,
            public_keys,
            metadata: None,
        }
    }

    pub fn random_identity(key_count: u16, seed: Option<u64>) -> Self {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        Self::random_identity_with_rng(key_count, &mut rng)
    }

    pub fn random_identities(count: u16, key_count: u16, seed: Option<u64>) -> Vec<Self> {
        let mut rng = match seed {
            None => StdRng::from_entropy(),
            Some(seed_value) => StdRng::seed_from_u64(seed_value),
        };
        let mut vec: Vec<Identity> = vec![];
        for _i in 0..count {
            vec.push(Self::random_identity_with_rng(key_count, &mut rng));
        }
        vec
    }
}
