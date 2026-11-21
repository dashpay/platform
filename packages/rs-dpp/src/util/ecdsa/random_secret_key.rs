use dashcore::crypto::ecdsa::EcdsaSecretKey;
use dashcore::key::rand::RngCore;

/// Draws a secret key from `rng` exactly as `secp256k1::SecretKey::new` does: 32 bytes at a time
/// until they form a valid scalar. Seeded callers get the same keys, and leave `rng` in the same
/// state, as before the typed key migration.
// TODO: replace with Core's RNG-parameterized constructor once rust-dashcore exposes one
// (dash-pkc already has `EcdsaSecretKey::generate(rng, compressed)`).
pub fn random_secret_key<R: RngCore + ?Sized>(rng: &mut R) -> EcdsaSecretKey {
    loop {
        let mut bytes = [0; 32];
        rng.fill_bytes(&mut bytes);
        if let Ok(key) = EcdsaSecretKey::from_bytes(&bytes) {
            return key;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::key::rand::rngs::StdRng;
    use dashcore::key::rand::SeedableRng;

    #[test]
    fn should_draw_the_same_key_and_rng_state_as_secp256k1() {
        for seed in 0..32u64 {
            let mut typed_rng = StdRng::seed_from_u64(seed);
            let mut raw_rng = StdRng::seed_from_u64(seed);
            let typed = random_secret_key(&mut typed_rng);
            let raw = secp256k1::SecretKey::new(&mut raw_rng);
            assert_eq!(*typed.to_bytes(), raw.to_secret_bytes(), "seed {seed}");
            assert_eq!(typed_rng.next_u64(), raw_rng.next_u64(), "seed {seed}");
        }
    }
}
