//! Asset lock tracking and synchronization.
//!
//! Contains methods for tracking asset locks, advancing their lifecycle status,
//! recovering locks, waiting for proofs and chain locks, resuming interrupted
//! locks, and re-deriving private keys.

mod proof;
pub(crate) mod reconstruction;
mod recovery;
/// `pub(super)` so the create path in `build.rs` — the other
/// `Built` → `Broadcast` writer — can name
/// [`BuiltPromotion`](tracking::BuiltPromotion) and share the same
/// compare-and-set instead of writing the status unconditionally.
pub(super) mod tracking;
