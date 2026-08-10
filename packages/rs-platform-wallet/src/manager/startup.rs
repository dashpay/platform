//! Ordered wallet bring-up: identity → contacts → contact accounts.
//!
//! A DashPay contact's DIP-15 payment addresses are derived from its contact
//! account, and an address the wallet is not watching when the compact-filter
//! scan passes its funding height produces no transaction at all. So the order
//! in which these come up decides whether a restored wallet has contact
//! payment history, and the client must be able to hold Core SPV back until
//! the addresses exist.
//!
//! That sequencing is a policy decision, which is why it lives here rather
//! than in each client. The iOS app had it in Swift first (a hand-rolled step
//! sequencer with its own budget and poll loops); Android would have had to
//! reproduce it, and the identity-retry half of it regressed once already
//! while it lived there.
//!
//! # The step that is easy to miss
//!
//! After a DashPay sync pass the contact accounts still do not exist. The
//! recurring sweep runs unattended and holds no signer, so it cannot derive
//! the receiving xpub or run the ECDH for a contact's external account — it
//! only *enqueues* the work (see `enqueue_deferred_contact_crypto` in
//! [`crate::wallet::identity::network::contact_requests`]). The accounts come
//! into being when a signer-present drain runs. A sequence that stopped after
//! the sync pass would be correctly ordered and still start SPV with nothing
//! extra to watch.
//!
//! # Key material
//!
//! Per-call, never resident. The caller resolves the master xpriv and the
//! contact-crypto provider for exactly this call and drops them after — the
//! same contract [`crate::wallet::identity::IdentityWallet::discover_from_master`]
//! and the drain already use. Making the unattended sweep self-sufficient
//! instead would turn a narrowly-scoped Keychain capability into a standing
//! one, which is a security-posture change and deliberately not what this is.

use std::time::Duration;

use dpp::prelude::Identifier;

/// Why a bring-up stopped where it did.
///
/// Every variant is a normal return. Budget exhaustion and an unreachable
/// Platform are outcomes the caller acts on, not errors to unwind — Core sync
/// is the wallet's primary function and must never be held hostage to
/// Platform being slow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletStartupStatus {
    /// Identity resolved, the sync pass ran, and no contact-account builds are
    /// left queued. Everything a contact payment needs is in place.
    Ready,
    /// Platform answered that this seed owns no identity. Terminal, and not a
    /// failure: there is nothing to sync and nothing to drain.
    NoIdentity,
    /// The scan never reached Platform within the budget. The wallet may well
    /// own an identity; we do not know yet.
    PartialNoIdentity,
    /// Identity resolved and synced, but contact-account builds are still
    /// queued — the drain did not finish inside the budget.
    PartialAccountsPending,
}

impl WalletStartupStatus {
    /// Whether Platform gave a definitive answer about this seed's identity.
    ///
    /// `false` only for [`Self::PartialNoIdentity`] — the one outcome worth
    /// repeating. This is the distinction that platform#4352 made expressible:
    /// before it, "no identity exists" and "we never got through" both arrived
    /// as an empty success, so clients either retried a proven-empty scan
    /// forever or cached a network failure as fact.
    pub fn identity_is_settled(self) -> bool {
        !matches!(self, Self::PartialNoIdentity)
    }
}

/// What a bring-up did, for the client to log and act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletStartupOutcome {
    pub status: WalletStartupStatus,
    /// The wallet's identity, when one is known by the time this returns.
    pub identity_id: Option<Identifier>,
    /// Discovery scans performed. `0` when a local identity was already known
    /// and no network scan was needed.
    pub discovery_attempts: u32,
    /// Whether the inline DashPay sync pass ran (skipped when there is no
    /// identity to sync for).
    pub dashpay_sync_ran: bool,
    /// Contact-crypto entries completed by the drain.
    pub contact_accounts_drained: usize,
    /// Contact-account builds still queued when this returned.
    pub contact_accounts_pending: usize,
    pub elapsed: Duration,
}

/// Running state of one bring-up, and the verdict it produces.
///
/// Split out from the async body so the classification is testable without an
/// SDK, a wallet, or a network — the same reason [`ScanTally`] exists in
/// `discovery.rs`. The empty-vs-unreachable rule below is the one that has
/// already regressed once in a client implementation, so it is pinned by tests
/// rather than left inline.
///
/// [`ScanTally`]: crate::wallet::identity::network::discovery
#[derive(Debug, Default)]
pub(crate) struct StartupTally {
    /// Identity known locally or found by a scan.
    pub identity_id: Option<Identifier>,
    /// Platform definitively answered "this seed owns no identity".
    pub proven_no_identity: bool,
    /// The budget ran out while discovery was still unreachable.
    pub discovery_unreachable: bool,
    pub discovery_attempts: u32,
    pub dashpay_sync_ran: bool,
    pub contact_accounts_drained: usize,
    pub contact_accounts_pending: usize,
}

impl StartupTally {
    /// A local identity was already on file — no scan needed.
    pub(crate) fn record_local_identity(&mut self, identity_id: Identifier) {
        self.identity_id = Some(identity_id);
    }

    /// A scan reached Platform and found an identity.
    pub(crate) fn record_discovered(&mut self, identity_id: Identifier) {
        self.identity_id = Some(identity_id);
        self.discovery_attempts += 1;
    }

    /// A scan reached Platform and proved there is no identity for this seed.
    ///
    /// Terminal. Rescanning cannot change a proof of absence, and treating it
    /// as retryable is exactly the bug this type exists to prevent: it costs a
    /// wallet that legitimately owns no identity a full backoff schedule of
    /// pointless round trips on every launch.
    pub(crate) fn record_proven_absent(&mut self) {
        self.proven_no_identity = true;
        self.discovery_attempts += 1;
    }

    /// A scan failed to reach Platform. Retryable while budget remains.
    pub(crate) fn record_unreachable(&mut self) {
        self.discovery_attempts += 1;
    }

    /// The budget expired with discovery still unresolved.
    pub(crate) fn record_discovery_gave_up(&mut self) {
        self.discovery_unreachable = true;
    }

    /// Whether there is an identity to sync and drain for.
    pub(crate) fn has_identity(&self) -> bool {
        self.identity_id.is_some()
    }

    pub(crate) fn record_sync_ran(&mut self) {
        self.dashpay_sync_ran = true;
    }

    pub(crate) fn record_drain(&mut self, drained: usize, pending: usize) {
        self.contact_accounts_drained = drained;
        self.contact_accounts_pending = pending;
    }

    /// Classify the run.
    ///
    /// Order matters: an unreachable Platform outranks everything, because
    /// every later step was skipped or ran against incomplete state. A proven
    /// absence outranks the drain counters for the same reason — with no
    /// identity there is nothing to have drained.
    pub(crate) fn status(&self) -> WalletStartupStatus {
        if self.discovery_unreachable {
            return WalletStartupStatus::PartialNoIdentity;
        }
        if self.proven_no_identity && self.identity_id.is_none() {
            return WalletStartupStatus::NoIdentity;
        }
        if self.contact_accounts_pending > 0 {
            return WalletStartupStatus::PartialAccountsPending;
        }
        WalletStartupStatus::Ready
    }

    pub(crate) fn into_outcome(self, elapsed: Duration) -> WalletStartupOutcome {
        WalletStartupOutcome {
            status: self.status(),
            identity_id: self.identity_id,
            discovery_attempts: self.discovery_attempts,
            dashpay_sync_ran: self.dashpay_sync_ran,
            contact_accounts_drained: self.contact_accounts_drained,
            contact_accounts_pending: self.contact_accounts_pending,
            elapsed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> Identifier {
        Identifier::from([7u8; 32])
    }

    /// The regression this type exists to prevent, and the one that already
    /// shipped once in a client: a scan that came back definitively empty must
    /// settle as `NoIdentity`, never as something to retry.
    #[test]
    fn proven_absence_settles_and_is_not_retryable() {
        let mut tally = StartupTally::default();
        tally.record_proven_absent();

        assert_eq!(tally.status(), WalletStartupStatus::NoIdentity);
        assert!(
            tally.status().identity_is_settled(),
            "a proof of absence is an answer; retrying it cannot change it"
        );
    }

    /// The opposite case, and the reason the distinction is expressible at all
    /// (platform#4352): never reaching Platform is not evidence of absence.
    #[test]
    fn unreachable_discovery_is_not_settled() {
        let mut tally = StartupTally::default();
        tally.record_unreachable();
        tally.record_unreachable();
        tally.record_discovery_gave_up();

        assert_eq!(tally.status(), WalletStartupStatus::PartialNoIdentity);
        assert!(!tally.status().identity_is_settled());
        assert_eq!(tally.discovery_attempts, 2);
    }

    /// An unreachable Platform outranks a clean drain: the later steps ran
    /// against state we know to be incomplete.
    #[test]
    fn unreachable_discovery_outranks_a_finished_drain() {
        let mut tally = StartupTally::default();
        tally.record_unreachable();
        tally.record_discovery_gave_up();
        tally.record_drain(0, 0);

        assert_eq!(tally.status(), WalletStartupStatus::PartialNoIdentity);
    }

    #[test]
    fn identity_found_and_drained_is_ready() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        tally.record_sync_ran();
        tally.record_drain(4, 0);

        assert_eq!(tally.status(), WalletStartupStatus::Ready);
        assert_eq!(tally.discovery_attempts, 1);
    }

    /// A warm launch: the identity was already on file, so no scan ran at all.
    #[test]
    fn local_identity_needs_no_discovery_attempt() {
        let mut tally = StartupTally::default();
        tally.record_local_identity(identity());
        tally.record_sync_ran();
        tally.record_drain(0, 0);

        assert_eq!(tally.status(), WalletStartupStatus::Ready);
        assert_eq!(
            tally.discovery_attempts, 0,
            "a known identity must not cost a network scan"
        );
    }

    #[test]
    fn queued_builds_report_as_pending() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        tally.record_sync_ran();
        tally.record_drain(2, 3);

        assert_eq!(tally.status(), WalletStartupStatus::PartialAccountsPending);
        assert!(
            tally.status().identity_is_settled(),
            "the identity question is answered even though the drain is not done"
        );
    }

    /// `has_identity` gates the sync and drain steps, so it must not be fooled
    /// by a proven absence.
    #[test]
    fn proven_absence_has_no_identity_to_sync_for() {
        let mut tally = StartupTally::default();
        tally.record_proven_absent();

        assert!(!tally.has_identity());
    }

    #[test]
    fn outcome_carries_the_tally_through() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        tally.record_sync_ran();
        tally.record_drain(1, 0);

        let outcome = tally.into_outcome(Duration::from_secs(3));
        assert_eq!(outcome.status, WalletStartupStatus::Ready);
        assert_eq!(outcome.identity_id, Some(identity()));
        assert!(outcome.dashpay_sync_ran);
        assert_eq!(outcome.contact_accounts_drained, 1);
        assert_eq!(outcome.elapsed, Duration::from_secs(3));
    }
}
