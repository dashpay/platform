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

/// Canonical subtree key for the compacted address-balance tree under
/// `SavedBlockTransactions`.
///
/// Defined here (a `verify`-available module) rather than in the `server`-gated
/// `saved_block_transactions::queries` so the **verify-side** proof verifier and
/// the **server-side** storage/fetch path reference one definition — the subtree
/// location is part of the proof contract and must not drift between them.
/// `saved_block_transactions::queries` re-exports this constant for its
/// server-side callers, so the `b'c'` byte is written in exactly one place.
pub const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';

/// Path to the compacted address-balance subtree under `SavedBlockTransactions`.
///
/// Shared by the server-side storage/fetch path and the verify-side proof
/// verifier; both sides reach it through this one helper. See
/// [`COMPACTED_ADDRESS_BALANCES_KEY_U8`] for why the byte lives in this module.
pub(crate) fn compacted_address_balances_path() -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::SavedBlockTransactions as u8],
        vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
    ]
}
