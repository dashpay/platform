use crate::util::hash::sha;
use std::io::Write;

/// Generate data contract id based on owner id and entropy
pub fn generate_data_contract_id(owner_id: impl AsRef<[u8]>, entropy: impl AsRef<[u8]>) -> Vec<u8> {
    let mut b: Vec<u8> = vec![];
    let _ = b.write(owner_id.as_ref());
    let _ = b.write(entropy.as_ref());
    sha(b)
}
