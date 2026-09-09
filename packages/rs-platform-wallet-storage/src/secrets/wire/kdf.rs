//! Bincode-encoded wire image of [`KdfParams`] — the Argon2 parameter
//! header read out of every scheme-1 envelope.
//!
//! Kept as a separate type from [`KdfParams`] (the in-memory + JSON-
//! vault type) so the wire layer owns its own bincode derives and the
//! in-memory type keeps its serde derives for the human-debuggable JSON
//! vault format.

use crate::secrets::error::SecretStoreError;
use crate::secrets::file::crypto::KdfParams;

/// Wire image of [`KdfParams`]: `id ‖ m_kib ‖ t ‖ p`, each a fixed-
/// width integer under the bincode varint config. Encoded once into
/// every scheme-1 envelope's `Payload::Password` body AND into the
/// scheme-1 AAD, so the two cannot disagree without failing the tag.
#[derive(bincode::Encode, bincode::Decode, Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct KdfParamsEncoded {
    /// Argon2 algorithm discriminator (only `KDF_ID_ARGON2ID = 1`
    /// today; enforced by [`KdfParams::enforce_bounds`]).
    pub id: u8,
    /// Argon2 memory cost (KiB). Bounded.
    pub m_kib: u32,
    /// Argon2 time cost (iterations). Bounded.
    pub t: u32,
    /// Argon2 parallelism. Pinned to 1.
    pub p: u32,
}

impl From<KdfParams> for KdfParamsEncoded {
    fn from(k: KdfParams) -> Self {
        Self {
            id: k.id,
            m_kib: k.m_kib,
            t: k.t,
            p: k.p,
        }
    }
}

impl TryFrom<KdfParamsEncoded> for KdfParams {
    type Error = SecretStoreError;

    /// Convert the wire image into the in-memory [`KdfParams`], gated on
    /// [`KdfParams::enforce_bounds`] so an inflated header never
    /// reaches `derive_key`.
    fn try_from(k: KdfParamsEncoded) -> Result<Self, Self::Error> {
        let out = KdfParams {
            id: k.id,
            m_kib: k.m_kib,
            t: k.t,
            p: k.p,
        };
        out.enforce_bounds()?;
        Ok(out)
    }
}
