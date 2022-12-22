use std::cell::RefCell;

use dpp::data_contract::EntropyGenerator;
use rand::{rngs::SmallRng, Rng, SeedableRng};

pub(crate) struct TestEntropyGenerator {
    rng: RefCell<SmallRng>,
}

impl TestEntropyGenerator {
    pub(crate) fn new() -> Self {
        Self {
            rng: RefCell::new(SmallRng::seed_from_u64(1337)),
        }
    }
}

impl EntropyGenerator for TestEntropyGenerator {
    fn generate(&self) -> [u8; 32] {
        self.rng.borrow_mut().gen()
    }
}
