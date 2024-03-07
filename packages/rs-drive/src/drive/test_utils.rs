use std::cell::RefCell;

use dpp::util::entropy_generator::EntropyGenerator;
use rand::{rngs::SmallRng, Rng, SeedableRng};

/// test entropy generator
pub(crate) struct TestEntropyGenerator {
    rng: RefCell<SmallRng>,
}

impl TestEntropyGenerator {
    /// new test entropy generator
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub(crate) fn new() -> Self {
        Self {
            rng: RefCell::new(SmallRng::seed_from_u64(1337)),
        }
    }
}

impl EntropyGenerator for TestEntropyGenerator {
    fn generate(&self) -> anyhow::Result<[u8; 32]> {
        Ok(self.rng.borrow_mut().gen())
    }
}
