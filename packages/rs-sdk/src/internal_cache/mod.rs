use crate::platform::Identifier;
use crate::sdk::LastQueryTimestamp;
use dpp::prelude::IdentityNonce;
use std::collections::BTreeMap;
use tokio::sync::Mutex;

/// Cached nonce state: `(current_nonce, last_fetch_timestamp, last_platform_nonce)`.
///
/// - `current_nonce` — the nonce value the SDK will use next (may be bumped
///   ahead of what Platform reported).
/// - `last_fetch_timestamp` — when we last fetched from Platform.
/// - `last_platform_nonce` — the value Platform returned on the last fetch
///   (after stripping upper bits). Used to detect when local bumps have
///   drifted too far ahead, triggering a re-fetch.
pub(crate) type NonceCacheEntry = (IdentityNonce, LastQueryTimestamp, IdentityNonce);

/// This is a cache that is internal to the SDK that the user does not have to worry about
pub struct InternalSdkCache {
    /// This is the identity nonce counter for the sdk
    /// The sdk will automatically manage this counter for the user.
    /// When the sdk user requests to update identities, withdraw or transfer
    /// this will be automatically updated
    /// This update can involve querying Platform for the current identity nonce
    /// If the sdk user requests to put a state transition the counter is checked and either
    /// returns an error or is updated.
    pub(crate) identity_nonce_counter: tokio::sync::Mutex<BTreeMap<Identifier, NonceCacheEntry>>,

    /// This is the identity contract nonce counter for the sdk
    /// The sdk will automatically manage this counter for the user.
    /// When the sdk user requests to put documents this will be automatically updated
    /// This update can involve querying Platform for the current identity contract nonce
    /// If the sdk user requests to put a state transition the counter is checked and either
    /// returns an error or is updated.
    pub(crate) identity_contract_nonce_counter:
        tokio::sync::Mutex<BTreeMap<(Identifier, Identifier), NonceCacheEntry>>,
}

impl Default for InternalSdkCache {
    fn default() -> Self {
        InternalSdkCache {
            identity_nonce_counter: Mutex::new(BTreeMap::<Identifier, NonceCacheEntry>::new()),
            identity_contract_nonce_counter: Mutex::new(BTreeMap::<
                (Identifier, Identifier),
                NonceCacheEntry,
            >::new()),
        }
    }
}
