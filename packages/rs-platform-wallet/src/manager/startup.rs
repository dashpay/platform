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

use std::time::{Duration, Instant};

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::Identifier;
use key_wallet::bip32::ExtendedPrivKey;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::manager::PlatformWalletManager;
use crate::wallet::identity::network::{ContactCryptoProvider, IdentityDiscoveryOptions};
use crate::wallet::platform_wallet::WalletId;

/// Whole-sequence budget when the caller does not name one.
///
/// Core sync is the wallet's primary function and Platform is not: a Platform
/// outage must never be able to leave the user without a balance. Twenty
/// seconds is roughly the window in which a cold quorum endpoint either
/// answers or has already failed.
pub const DEFAULT_STARTUP_BUDGET: Duration = Duration::from_secs(20);

/// Backoff between discovery attempts when Platform could not be reached.
///
/// Retrying the scan is what helps here, not retrying inside DAPI: every probe
/// verifies its proof against quorum keys fetched from a single endpoint whose
/// cache is cold at process start, so a scan that begins before that endpoint
/// answers fails whole rather than per-node. A wallet that owns no identity
/// never reaches this schedule — a proof of absence settles on the first pass.
const DISCOVERY_BACKOFF: [Duration; 2] = [Duration::from_secs(3), Duration::from_secs(8)];

/// Run `future` with whatever is left of the budget, or not at all.
///
/// `None` means the deadline passed — either before the step started or while
/// it ran. Every step in the sequence is abandonable: the work it did not
/// finish stays queued for the next attempt, and the caller needs Core SPV to
/// start far more than it needs any one of them to complete.
async fn within_budget<F: std::future::Future>(deadline: Instant, future: F) -> Option<F::Output> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return None;
    }
    tokio::time::timeout(remaining, future).await.ok()
}

/// Knobs for [`PlatformWalletManager::start_wallet_subsystems`].
#[derive(Debug, Clone, Copy)]
pub struct WalletStartupOptions {
    /// Ceiling for the whole sequence. Expiry is reported, never an error.
    pub budget: Duration,
    /// Gap limit for identity discovery; `None` uses the crate default.
    pub gap_limit: Option<u32>,
}

impl Default for WalletStartupOptions {
    fn default() -> Self {
        Self {
            budget: DEFAULT_STARTUP_BUDGET,
            gap_limit: None,
        }
    }
}

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
    /// own an identity; we do not know yet, and asking again may answer it.
    PartialNoIdentity,
    /// Discovery failed locally — a wallet-manager or persistence error, not a
    /// reachability problem. Like [`Self::PartialNoIdentity`] the identity
    /// question is unanswered, but unlike it another scan will not answer it:
    /// the same local fault is still there. Terminal for this launch.
    DiscoveryFailed,
    /// Identity resolved and synced, but contact-account builds are still
    /// queued. The budget may have run out, the drain may have failed on some
    /// entries, or no contact-crypto provider was supplied — the count is what
    /// is certain, not the reason. Either way those contacts' payments wait on
    /// the DIP-15 rescan.
    PartialAccountsPending,
}

impl WalletStartupStatus {
    /// Whether another discovery scan could change the answer.
    ///
    /// True only for [`Self::PartialNoIdentity`]. The other three are terminal
    /// for different reasons — an identity was found, absence was proven, or
    /// the failure is local and will still be there next time — and only an
    /// unreachable Platform is worth asking again.
    ///
    /// This is the distinction platform#4352 made expressible: before it, "no
    /// identity exists" and "we never got through" both arrived as an empty
    /// success, so clients either retried a proven-empty scan forever or cached
    /// a network failure as fact.
    pub fn discovery_worth_retrying(self) -> bool {
        matches!(self, Self::PartialNoIdentity)
    }

    /// Whether the identity question has an answer.
    ///
    /// Note this is NOT the inverse of [`Self::discovery_worth_retrying`]:
    /// [`Self::DiscoveryFailed`] leaves the question open *and* is not worth
    /// retrying. Use this to decide what to display, and
    /// `discovery_worth_retrying` to decide whether to scan again.
    pub fn identity_is_settled(self) -> bool {
        !matches!(self, Self::PartialNoIdentity | Self::DiscoveryFailed)
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
    /// Discovery hit a local fault (wallet manager, persistence) rather than a
    /// reachability problem. Tracked apart from `discovery_unreachable`
    /// because retrying cannot clear it.
    pub discovery_failed_locally: bool,
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

    /// The budget expired with discovery still unresolved. Retryable.
    pub(crate) fn record_discovery_gave_up(&mut self) {
        self.discovery_unreachable = true;
    }

    /// Discovery failed on a local fault. Not retryable — the wallet-manager
    /// or persistence problem behind it is still there on the next attempt, so
    /// telling the client to rescan would only waste a round trip.
    pub(crate) fn record_discovery_failed_locally(&mut self) {
        self.discovery_failed_locally = true;
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
        // A local fault outranks unreachability: both leave the question open,
        // but only this one tells the client not to bother asking again.
        if self.discovery_failed_locally {
            return WalletStartupStatus::DiscoveryFailed;
        }
        if self.discovery_unreachable {
            return WalletStartupStatus::PartialNoIdentity;
        }
        if self.proven_no_identity && self.identity_id.is_none() {
            return WalletStartupStatus::NoIdentity;
        }
        if self.contact_accounts_pending > 0 {
            return WalletStartupStatus::PartialAccountsPending;
        }
        // An empty queue only means "nothing left to build" if a contact pass
        // actually completed. Without one there may be undiscovered contact
        // requests whose account builds were never enqueued, and calling that
        // `Ready` would promise addresses this call never prepared.
        if !self.dashpay_sync_ran {
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

impl<P: PlatformWalletPersistence + Send + Sync + 'static> PlatformWalletManager<P> {
    /// Bring one wallet's DashPay state up in dependency order, so the caller
    /// can start Core SPV against a complete contact-address set.
    ///
    /// Runs identity discovery (only when no identity is known locally), then
    /// one DashPay sync pass, then the signer-present drain that turns queued
    /// contact-crypto into real accounts. See the module docs for why the
    /// drain is not optional.
    ///
    /// # Errors
    ///
    /// Two, both about the request rather than its execution:
    /// [`PlatformWalletError::WalletNotFound`] for an unknown `wallet_id`, and
    /// [`PlatformWalletError::InvalidParameter`] for a budget whose deadline
    /// would not be representable.
    ///
    /// Everything about the run itself — an unreachable Platform, a failed
    /// sync pass, a drain that did not finish inside the budget — is reported
    /// in [`WalletStartupOutcome`], so a client can start Core SPV regardless
    /// and let the DIP-15 rescan repair whatever is missing. Failing loudly
    /// there would trade a data gap for a wallet with no balance, which is the
    /// worse of the two.
    ///
    /// # Key material
    ///
    /// `master` and `contact_crypto` are borrowed for this call only and must
    /// not be retained by the caller afterwards. `master` is `None` for a
    /// wallet holding resident keys; `identity_signer` is `None` to skip the
    /// DIP-15 auto-accept pass.
    pub async fn start_wallet_subsystems<C, S>(
        &self,
        wallet_id: &WalletId,
        master: Option<&ExtendedPrivKey>,
        contact_crypto: Option<&C>,
        identity_signer: Option<&S>,
        opts: WalletStartupOptions,
    ) -> Result<WalletStartupOutcome, PlatformWalletError>
    where
        C: ContactCryptoProvider + Sync,
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let started = Instant::now();
        // `Instant + Duration` panics when the sum is not representable, and
        // this runs under an `extern "C"` caller whose thread wrapper re-raises
        // a panic as an abort — so an absurd budget would take the host process
        // down rather than return an error.
        let deadline = started.checked_add(opts.budget).ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "startup budget {:?} exceeds the supported duration range",
                opts.budget
            ))
        })?;
        let mut tally = StartupTally::default();

        let wallet = self
            .get_wallet(wallet_id)
            .await
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
        let identity_wallet = wallet.identity();

        // 1. Local identities first. A warm launch must not pay for a network
        //    scan it does not need.
        if let Some(known) = self.local_identity_id(wallet_id).await {
            tally.record_local_identity(known);
        } else {
            self.discover_identity_with_backoff(
                wallet_id,
                identity_wallet,
                master,
                opts.gap_limit,
                deadline,
                &mut tally,
            )
            .await;
        }

        // With no identity there is nothing to sync and nothing to drain, and
        // that is true whether Platform proved absence or never answered.
        if !tally.has_identity() {
            return Ok(tally.into_outcome(started.elapsed()));
        }

        // 2. One contact-request pass, so the deferred builds exist to drain.
        //    Log-and-continue: a prior session may already have queued work
        //    that this call can still complete.
        match within_budget(deadline, identity_wallet.dashpay().sync_contact_requests()).await {
            Some(Ok(requests)) => {
                tally.record_sync_ran();
                tracing::debug!(
                    wallet_id = %hex::encode(wallet_id),
                    requests = requests.len(),
                    "startup: contact-request pass complete"
                );
            }
            Some(Err(e)) => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    error = %e,
                    "startup: contact-request pass failed; continuing to the drain"
                );
            }
            None => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    "startup: budget spent before the contact-request pass finished"
                );
            }
        }

        // 3. The step that actually creates the addresses.
        //
        // Both drains hit the network, so both are bounded. An abandoned drain
        // is safe to walk away from: entries it did not complete stay queued
        // and the next signer-present action retries them.
        //
        // Without a provider there is nothing to drain WITH. A caller that
        // passes `None` gets the sequence's other steps and an honest
        // `contact_accounts_pending`, rather than a drain that reports zero
        // because every crypto operation failed.
        let (drained, accepted) = match contact_crypto {
            Some(contact_crypto) => {
                let drained = within_budget(
                    deadline,
                    identity_wallet
                        .dashpay()
                        .drain_pending_contact_crypto(contact_crypto),
                )
                .await
                .unwrap_or(0);
                let accepted = match identity_signer {
                    Some(signer) => within_budget(
                        deadline,
                        identity_wallet
                            .dashpay()
                            .drain_auto_accepts(signer, contact_crypto),
                    )
                    .await
                    .unwrap_or(0),
                    None => 0,
                };
                (drained, accepted)
            }
            None => {
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    "startup: no contact-crypto provider; skipping the drain"
                );
                (0, 0)
            }
        };
        // Not budgeted: a local queue-length read with no I/O. Leaving it
        // unbounded keeps the reported `pending` truthful even when the steps
        // above ran out of time — which is exactly when it matters most.
        let pending = identity_wallet
            .dashpay()
            .pending_contact_crypto_count()
            .await;
        tally.record_drain(drained + accepted, pending);

        if pending > 0 {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                pending,
                "startup: contact-account builds still queued; the DIP-15 rescan will backfill"
            );
        }

        Ok(tally.into_outcome(started.elapsed()))
    }

    /// The first identity this wallet already owns locally, if any.
    async fn local_identity_id(&self, wallet_id: &WalletId) -> Option<Identifier> {
        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(wallet_id)?;
        info.identity_manager
            .wallet_identity_ids(wallet_id)
            .into_iter()
            .next()
    }

    /// Scan for an identity, retrying only while Platform stays unreachable.
    ///
    /// An `Ok` result ends the loop whether or not it found anything: Platform
    /// answered, and an empty answer is a proof of absence that rescanning
    /// cannot overturn. Only [`PlatformWalletError::IdentityDiscoveryIncomplete`]
    /// — the error platform#4352 introduced for a scan that never got through
    /// — is worth another attempt.
    async fn discover_identity_with_backoff(
        &self,
        wallet_id: &WalletId,
        identity_wallet: &crate::wallet::identity::IdentityWallet,
        master: Option<&ExtendedPrivKey>,
        gap_limit: Option<u32>,
        deadline: Instant,
        tally: &mut StartupTally,
    ) {
        let opts = IdentityDiscoveryOptions {
            start_index: Some(0),
            gap_limit: gap_limit.unwrap_or(IdentityDiscoveryOptions::default().gap_limit),
        };

        for (attempt, backoff) in DISCOVERY_BACKOFF.iter().map(Some).chain([None]).enumerate() {
            // A single scan walks up to `gap_limit` indices, each a Platform
            // fetch plus a DPNS lookup, so it needs the same ceiling the other
            // steps have — otherwise one attempt can outlast the whole budget
            // this call promises.
            let attempt_future = async {
                match master {
                    Some(master) => identity_wallet.discover_from_master(opts, master).await,
                    None => identity_wallet.discover(opts).await,
                }
            };
            let Some(result) = within_budget(deadline, attempt_future).await else {
                // Sightings persist incrementally, so an abandoned scan may
                // still have folded an identity in before it was cut off.
                if let Some(known) = self.local_identity_id(wallet_id).await {
                    tally.record_local_identity(known);
                    return;
                }
                break;
            };

            match result {
                Ok(found) => {
                    match found.first() {
                        Some(identity) => tally.record_discovered(identity.id()),
                        // An empty return is not proof on its own: `discover`
                        // reports only identities THIS call inserted, so a
                        // concurrent startup that inserted one first leaves us
                        // seeing it as already-managed and returning nothing.
                        // Consult local state before calling it absence.
                        None => match self.local_identity_id(wallet_id).await {
                            Some(known) => tally.record_discovered(known),
                            None => tally.record_proven_absent(),
                        },
                    }
                    return;
                }
                Err(PlatformWalletError::IdentityDiscoveryIncomplete { .. }) => {
                    tally.record_unreachable();
                }
                Err(e) => {
                    // Not a reachability question — a wallet/persistence
                    // failure will not fix itself on the next attempt, so this
                    // is recorded as terminal rather than as "try again".
                    tracing::warn!(
                        error = %e,
                        "startup: identity discovery failed for a non-network reason"
                    );
                    tally.record_discovery_failed_locally();
                    return;
                }
            }

            let Some(backoff) = backoff else { break };
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            tracing::info!(
                attempt = attempt + 1,
                "startup: identity discovery could not reach Platform; retrying"
            );
            tokio::time::sleep((*backoff).min(remaining)).await;
        }

        tally.record_discovery_gave_up();
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

    /// A local discovery fault is terminal, unlike an unreachable Platform.
    /// The branch that produces it says the failure will not fix itself, so
    /// reporting it as retryable would send clients on a futile rescan.
    #[test]
    fn a_local_discovery_failure_is_terminal() {
        let mut tally = StartupTally::default();
        tally.record_discovery_failed_locally();

        assert_eq!(tally.status(), WalletStartupStatus::DiscoveryFailed);
        assert!(
            !tally.status().discovery_worth_retrying(),
            "the same local fault will still be there next time"
        );
        assert!(
            !tally.status().identity_is_settled(),
            "terminal is not the same as answered — we still do not know"
        );
    }

    /// Both leave the identity question open, but only one is worth asking
    /// again. Keeping that asymmetry visible is the point of the two methods.
    #[test]
    fn only_an_unreachable_platform_is_worth_retrying() {
        let mut unreachable = StartupTally::default();
        unreachable.record_unreachable();
        unreachable.record_discovery_gave_up();
        assert!(unreachable.status().discovery_worth_retrying());

        for terminal in [
            WalletStartupStatus::Ready,
            WalletStartupStatus::NoIdentity,
            WalletStartupStatus::PartialAccountsPending,
            WalletStartupStatus::DiscoveryFailed,
        ] {
            assert!(
                !terminal.discovery_worth_retrying(),
                "{terminal:?} must not ask the client to rescan"
            );
        }
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

    /// An empty drain queue is not evidence of readiness on its own. Without a
    /// completed contact pass there may be requests nobody has looked at, whose
    /// account builds were therefore never enqueued — reporting `Ready` would
    /// promise addresses this call never prepared.
    #[test]
    fn an_empty_queue_without_a_contact_pass_is_not_ready() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        tally.record_drain(0, 0);

        assert!(!tally.dashpay_sync_ran);
        assert_eq!(tally.status(), WalletStartupStatus::PartialAccountsPending);
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

    /// Every network step is abandonable, so `within_budget` must return
    /// `None` rather than run a future past the deadline. This is the guard for
    /// the gap review found: bounding only the discovery retries let a stalled
    /// sync or drain hold Core SPV well past `budget`.
    #[tokio::test(start_paused = true)]
    async fn within_budget_abandons_a_step_that_outlasts_the_deadline() {
        let deadline = Instant::now() + Duration::from_secs(2);
        let slow = async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            "finished"
        };
        assert_eq!(within_budget(deadline, slow).await, None);
    }

    #[tokio::test(start_paused = true)]
    async fn within_budget_returns_a_step_that_fits() {
        let deadline = Instant::now() + Duration::from_secs(10);
        let quick = async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            "finished"
        };
        assert_eq!(within_budget(deadline, quick).await, Some("finished"));
    }

    /// A deadline already in the past must not start the step at all.
    #[tokio::test(start_paused = true)]
    async fn within_budget_skips_once_the_deadline_has_passed() {
        let deadline = Instant::now();
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(within_budget(deadline, async { "ran" }).await, None);
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
