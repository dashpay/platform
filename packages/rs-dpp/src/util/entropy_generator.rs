use getrandom::getrandom;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub fn generate(seed: Option<u64>) -> [u8; 32] {
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };
    rng.gen::<[u8; 32]>()
}
