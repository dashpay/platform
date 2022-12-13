use crate::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use rand::rngs::StdRng;
use rand::Rng;
use std::convert::TryFrom;
use std::ops::Range;

pub type KeyCount = KeyID;

impl IdentityPublicKey {
    pub fn random_key_with_rng(id: KeyID, key_size: u16, rng: &mut StdRng) -> Self {
        let key_type = KeyType::try_from(rng.gen_range(0..2)).unwrap();
        let purpose = Purpose::try_from(rng.gen_range(0..3)).unwrap();
        let security_level =
            SecurityLevel::try_from(rng.gen_range::<u32, Range<u32>>(0..4)).unwrap();
        let read_only = false;
        let data = (0..key_size).map(|_| rng.gen::<u8>()).collect();
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

    pub fn random_keys_with_rng(key_count: KeyCount, key_size: u16, rng: &mut StdRng) -> Vec<Self> {
        (0..key_count)
            .map(|i| Self::random_key_with_rng(i, key_size, rng))
            .collect()
    }
}
