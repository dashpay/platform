use anyhow::Context;
use getrandom::getrandom;

pub fn generate() -> Result<[u8; 32], anyhow::Error> {
    let mut buffer = [0u8; 32];
    getrandom(&mut buffer).context("generating entropy failed")?;
    Ok(buffer)
}
