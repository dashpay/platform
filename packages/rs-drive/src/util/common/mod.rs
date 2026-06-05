pub mod decode;
pub mod encode;

/// Builds the 16-byte big-endian compacted-block key `(start_block, end_block)`
/// shared by the shielded-nullifier and address-balance compacted trees.
///
/// This 16-byte boundary-key encoding is part of the chained-proof contract:
/// every prover and its matching verifier MUST construct keys identically, or
/// chained verification silently breaks. It therefore lives in exactly one
/// place (the four compacted prove/verify modules import it) so the encoding
/// cannot drift between them.
pub(crate) fn compacted_key(start_block: u64, end_block: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(16);
    key.extend_from_slice(&start_block.to_be_bytes());
    key.extend_from_slice(&end_block.to_be_bytes());
    key
}
