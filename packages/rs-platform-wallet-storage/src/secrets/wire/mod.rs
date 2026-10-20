//! Bincode wire format for the Tier-2 envelope and the three AAD
//! constructions used inside `secrets/`.
//!
//! Every byte that crosses the AEAD seam — the on-disk Tier-2 blob and the
//! AAD bound into each ciphertext — is produced by a `#[derive(bincode::
//! Encode)]` (or `Encode + Decode`) struct in this module, against the
//! single [`config::WIRE_CONFIG`] constant. A future bincode-config drift
//! is then caught by the golden vector tests in [`envelope::tests`]
//! instead of silently corrupting every stored blob.
//!
//! Module is `pub(crate)` only — the Tier-2 wire format is an
//! implementation detail of [`SecretStore`](super::store::SecretStore);
//! external callers see the unchanged `set_secret` / `get_secret` API.
//!
//! Audit-readable layout:
//!
//! - [`config`] — the single bincode config + domain-tag / version
//!   constants every encoder uses.
//! - [`kdf`] — `KdfParamsEncoded`, the wire image of [`KdfParams`].
//! - [`aad`] — the three AAD structs (`Tier2Aad` / `EntryAad` /
//!   `VerifyAad`).
//! - [`envelope`] — the `Envelope` + `Payload` structs plus the
//!   `wrap` / `unwrap` API.
//!
//! [`KdfParams`]: super::file::crypto::KdfParams
//!
//! Domain tags include an explicit `-v2` suffix to mark the
//! wire-format break from the pre-bincode hand-rolled layout
//! (`PWSEV-TIER2-AAD-v1` and the implicitly-untagged
//! `secrets/file/format.rs::aad` / `verify_aad` outputs).
#![deny(missing_docs)]

pub(crate) mod aad;
pub(crate) mod config;
pub(crate) mod envelope;
pub(crate) mod kdf;
