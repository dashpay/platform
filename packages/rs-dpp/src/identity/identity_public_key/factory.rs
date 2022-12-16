use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use rand::rngs::StdRng;
use rand::Rng;
use std::convert::TryFrom;
use std::ops::Range;

pub type KeyCount = KeyID;

impl IdentityPublicKey {
    pub fn random_key_with_rng(id: KeyID, rng: &mut StdRng) -> Self {
        let key_type = KeyType::try_from(rng.gen_range(0..4)).unwrap();
        let purpose = Purpose::try_from(rng.gen_range(0..3)).unwrap();
        let security_level = SecurityLevel::try_from(rng.gen_range(0..4)).unwrap();
        let read_only = false;
        let data = key_type.random_public_key_data(rng);
        IdentityPublicKey {
            id,
            key_type,
            purpose,
            security_level,
            read_only,
            disabled_at: None,
            data,
            signature: vec![],
        }
    }

    pub fn random_keys_with_rng(key_count: KeyCount, rng: &mut StdRng) -> Vec<Self> {
        (0..key_count)
            .map(|i| Self::random_key_with_rng(i, rng))
            .collect()
    }
}
