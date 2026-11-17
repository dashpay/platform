use dashcore::signer;

/// Generation 0, protocol versions 1 to 14: `dashcore`'s recovery check and nothing else. It
/// accepts a high-S signature and a header naming an uncompressed key. Kept as is for replay.
#[inline(always)]
pub(super) fn verify_hash_signature_v0(
    data_hash: &[u8],
    signature: &[u8],
    public_key_hash: &[u8],
) -> Result<(), anyhow::Error> {
    signer::verify_hash_signature(data_hash, signature, public_key_hash)
}
