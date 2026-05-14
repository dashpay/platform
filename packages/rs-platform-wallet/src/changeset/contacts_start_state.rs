//! DashPay contact store snapshot restored from storage.
//!
//! Returned as part of [`ClientStartState`](crate::changeset::ClientStartState)
//! by [`PlatformWalletPersistence::load`](crate::changeset::PlatformWalletPersistence::load)
//! so the platform wallet can rebuild one wallet's
//! [`ContactChangeSet`](crate::changeset::ContactChangeSet)-shaped state
//! without replaying a changeset.

use std::collections::BTreeMap;

use crate::changeset::changeset::{
    ContactRequestEntry, ReceivedContactRequestKey, SentContactRequestKey,
};
use crate::wallet::identity::EstablishedContact;

/// Restored DashPay contact store carried in [`ClientStartState`](crate::changeset::ClientStartState).
///
/// Snapshot, not delta — `removed_*` fields are absent because removed
/// rows never reach storage (the writer applies them as `DELETE`s).
/// Mirrors the populated-only subset of
/// [`ContactChangeSet`](crate::changeset::ContactChangeSet).
#[derive(Debug, Default, PartialEq)]
pub struct ContactsStartState {
    /// Sent contact requests keyed by (owner → recipient).
    pub sent_requests: BTreeMap<SentContactRequestKey, ContactRequestEntry>,
    /// Incoming contact requests keyed by (owner ← sender).
    pub incoming_requests: BTreeMap<ReceivedContactRequestKey, ContactRequestEntry>,
    /// Established contacts keyed by (owner → contact).
    pub established: BTreeMap<SentContactRequestKey, EstablishedContact>,
}

impl ContactsStartState {
    /// `true` when no sent / incoming / established rows were restored.
    pub fn is_empty(&self) -> bool {
        self.sent_requests.is_empty()
            && self.incoming_requests.is_empty()
            && self.established.is_empty()
    }
}
