//! Tier-2 opt-in per-object password envelope (backend-independent).
//!
//! Sits ABOVE [`SecretStore`](crate::secrets::SecretStore), over both
//! the `File` vault and `Os` keyring arms: the backend stores opaque
//! bytes, and a chosen critical object (a seed wallet, a single
//! privkey) can be wrapped under an extra, user-supplied **object
//! password** before it ever reaches the backend. Reading a protected
//! object then needs BOTH backend access AND the password — the first
//! control that survives a full backend compromise.
//!
//! The wire format lives in [`super::wire::envelope`]: every byte that
//! crosses the AEAD seam is bincode-encoded against a single
//! `WIRE_CONFIG`, so a future bincode-config drift is caught by the
//! golden-vector tests instead of silently corrupting every stored
//! blob. This module is a thin re-export hub for the call sites in
//! [`super::store`].

pub use super::wire::envelope::MAX_PLAINTEXT_LEN;
pub(crate) use super::wire::envelope::{unwrap, wrap, MAX_ENVELOPE_OVERHEAD};

#[cfg(test)]
pub(crate) use super::wire::envelope::wrap_with_params;
