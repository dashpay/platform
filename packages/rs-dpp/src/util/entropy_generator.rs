use anyhow::Context;
use getrandom::getrandom;

/// A way to provide external entropy generator.
pub trait EntropyGenerator {
    fn generate(&self) -> anyhow::Result<[u8; 32]>;
}

pub struct DefaultEntropyGenerator;

impl EntropyGenerator for DefaultEntropyGenerator {
    fn generate(&self) -> anyhow::Result<[u8; 32]> {
        let mut buffer = [0u8; 32];
        getrandom(&mut buffer).context("generating entropy failed")?;
        Ok(buffer)
    }
}
