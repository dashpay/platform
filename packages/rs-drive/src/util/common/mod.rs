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

/// Path to the compacted address-balance subtree under `SavedBlockTransactions`.
///
/// Lives here (a `verify`-available module) so the **server-side** storage/fetch
/// path and the **verify-side** proof verifier share one definition — the
/// subtree location is part of the proof contract and must not drift between
/// them. The byte must stay identical to
/// `saved_block_transactions::queries::COMPACTED_ADDRESS_BALANCES_KEY_U8`
/// (which is `server`-gated and so not referenceable from the verifier).
pub(crate) fn compacted_address_balances_path() -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::SavedBlockTransactions as u8],
        vec![b'c'], // COMPACTED_ADDRESS_BALANCES_KEY_U8
    ]
}
