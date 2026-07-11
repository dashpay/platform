//! An owned snapshot of an address' ban state, decoupled from the
//! lock-guarded [`crate::AddressList`] internals so it can cross crate
//! and FFI boundaries.

use chrono::{DateTime, Utc};

/// An owned, cloned snapshot of a single address' ban state, returned
/// from [`crate::AddressList::ban_info`].
///
/// This is a plain-data mirror of [`crate::AddressStatus`] (plus the
/// address URI) so it can be handed across crate / FFI boundaries
/// without holding the [`crate::AddressList`] lock.
#[derive(Debug, Clone)]
pub struct AddressBanInfo {
    /// The address URI as a string.
    pub uri: String,
    /// Whether the address is *currently effectively* banned, i.e. it
    /// has been banned at least once and the ban period has not yet
    /// expired. Consistent with the filtering used by
    /// [`crate::AddressList::get_live_address`].
    pub banned: bool,
    /// Total number of times the address has been banned (drives the
    /// exponential backoff).
    pub ban_count: usize,
    /// The timestamp until which the address remains banned, if any.
    pub banned_until: Option<DateTime<Utc>>,
    /// Human-readable reason for the most recent ban, if recorded.
    pub reason: Option<String>,
}
