//! DashPay profile model.
//!
//! A `DashPayProfile` is the per-identity user-facing metadata
//! published via the DashPay data contract: display name, bio, avatar
//! URL + bytes, public message. The platform-wallet stores it on
//! [`ManagedIdentity`](crate::wallet::identity::ManagedIdentity)
//! alongside the identity's other persistable fields, and emits it
//! through [`IdentityEntry`](crate::changeset::IdentityEntry) so the
//! persister can round-trip it.
//!
//! The fields mirror what evo-tool's `dashpay_profiles` table stores:
//! `(display_name, bio, avatar_url, avatar_bytes, public_message,
//! created_at, updated_at)`. Avatar bytes are an `Option<Vec<u8>>`
//! because the bytes are fetched lazily from the URL after the
//! profile metadata lands.

/// User-facing DashPay profile data published via the DashPay data
/// contract.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DashPayProfile {
    /// Display name (publicly visible).
    pub display_name: Option<String>,
    /// Biography / about-me text.
    pub bio: Option<String>,
    /// URL of the avatar image (HTTPS, IPFS, etc.).
    pub avatar_url: Option<String>,
    /// Raw avatar bytes, fetched lazily from `avatar_url`.
    /// `None` until the bytes have been downloaded.
    pub avatar_bytes: Option<Vec<u8>>,
    /// Public message broadcast to contacts.
    pub public_message: Option<String>,
}
