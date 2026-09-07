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
//! Per-call, never resident. The contact-crypto provider is built for exactly
//! this call and dropped after — the same contract the drain already uses. The
//! master xpriv is not even that: the caller hands over a [`ScanKeyResolver`]
//! and this module invokes it only on the branch that scans, so a launch with
//! nothing to discover never touches the Keychain at all, and what is resolved
//! is erased when its guard drops rather than on the paths that happen to
//! reach an explicit call. Making the unattended sweep self-sufficient instead
//! would turn a narrowly-scoped Keychain capability into a standing one, which
//! is a security-posture change and deliberately not what this is.

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
///
/// Only for steps that are **safe to drop mid-await**. This drops the future,
/// so any step that commits side effects before recording that it did must
/// take the deadline as a parameter and end itself between units of work
/// instead — that is why the two drains have `_until` variants rather than
/// being wrapped here.
async fn within_budget<F: std::future::Future>(deadline: Instant, future: F) -> Option<F::Output> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return None;
    }
    tokio::time::timeout(remaining, future).await.ok()
}

/// Produces the master xpriv an identity scan needs, on demand.
///
/// **Lazy is the point.** Which branches scan is this module's decision — a
/// wallet with an identity on file does not — and a caller that resolves up
/// front has to predict that decision to avoid paying for it. On iOS resolving
/// means a Keychain round trip, at worst behind a biometric prompt, so a warm
/// launch would pay for a key it never uses, and pay outside the budget it is
/// about to be measured against. Handing the sequence a closure instead keeps
/// the rule in one crate: the FFI marshals, the JNI bridge inherits the
/// behaviour rather than reimplementing it, and a future change to when a scan
/// happens cannot silently starve one of them of key material.
///
/// `None` means the wallet holds resident keys and discovery derives
/// in-process. An `Err` is classified by [`ScanKeyError`] rather than assumed.
pub type ScanKeyResolver<'a> =
    &'a (dyn Fn() -> Result<ExtendedPrivKey, ScanKeyError> + Send + Sync);

/// Why a [`ScanKeyResolver`] could not produce a key — and, the part that
/// matters, whether asking again could ever change the answer.
///
/// Flattening these two would misreport whichever one it did not pick. Both
/// have a plausible-looking home in the status vocabulary, and they are
/// opposites: one says "try on the next launch", the other says "stop".
#[derive(Debug)]
pub enum ScanKeyError {
    /// The key exists but could not be read right now — a locked device, a
    /// denied Keychain read, a resolver that was not ready yet. Reported as
    /// retryable, and the next launch may well succeed.
    Unavailable(String),
    /// The key material itself is wrong: a mnemonic that is not valid BIP-39,
    /// bytes that are not UTF-8, a master key that cannot be built from the
    /// seed. Reported as terminal — no number of retries edits what is stored.
    Invalid(String),
}

impl std::fmt::Display for ScanKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(detail) => write!(f, "scan key unavailable: {detail}"),
            Self::Invalid(detail) => write!(f, "scan key invalid: {detail}"),
        }
    }
}

/// Owns a resolved master xpriv and erases it when it drops.
///
/// `ExtendedPrivKey` has no erasing `Drop`, so an explicit
/// `non_secure_erase` at the end of the scan only covers the paths that reach
/// it. A caller that drops the whole `start_wallet_subsystems` future while
/// discovery is awaiting Platform reaches none of them, and the scalar stays
/// in the freed future. Tying the erase to the value's lifetime instead of to
/// a code path makes cancellation — which this call now invites, since it is
/// budget-bounded and abandonable by design — a non-event.
struct ScanKeyGuard(ExtendedPrivKey);

impl Drop for ScanKeyGuard {
    fn drop(&mut self) {
        self.0.private_key.non_secure_erase();
    }
}

impl std::ops::Deref for ScanKeyGuard {
    type Target = ExtendedPrivKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
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
    /// The contact-crypto provider does not resolve the seed that owns this
    /// wallet, so the drain was skipped without deriving anything.
    ///
    /// Not a slow-Platform outcome like the other partials — it says the host
    /// handed this call a signer for a different wallet, and the only safe
    /// response was to do nothing. Deriving anyway would write contact
    /// receiving xpubs from the wrong seed, and because
    /// `register_contact_account` keys its existence check on the contact pair
    /// rather than on the xpub, those wrong addresses would be written once
    /// and never revisited by a later correct-seed pass. The wallet would then
    /// watch addresses nobody pays to, with no symptom but payments that never
    /// arrive.
    SeedBindingUnverified,
    /// An identity is known and every later step ran, but the wallet's
    /// gap-limit identity scan is still on record as having left indices
    /// unanswered — the rescan this launch forced did not close the gap.
    ///
    /// The distinction from [`Self::Ready`] is the whole point: an identity
    /// hiding at an unanswered index is invisible to everything that consults
    /// local state, so calling this launch `Ready` promises an identity set
    /// that was never established. The lost-second-identity shape recurs one
    /// level up: the wallet has *an* identity, so the warm-launch shortcut
    /// and every tally signal read clean while a second identity stays lost.
    ///
    /// Not terminal: the verdict stays on record, so the next launch re-opens
    /// the question instead of taking the shortcut. Nothing about the contact
    /// state is in doubt here — the sync and the drain both ran for the
    /// identity that *is* known.
    IdentityScanIncomplete,
}

impl WalletStartupStatus {
    /// Whether another discovery scan could change the answer.
    ///
    /// True for [`Self::PartialNoIdentity`] (Platform was never reached) and
    /// [`Self::IdentityScanIncomplete`] (it was reached, but some indices were
    /// not). The rest are terminal for different reasons — absence was proven,
    /// the failure is local and will still be there next time, or the scan
    /// answered everything it probed.
    ///
    /// The distinction matters: if "no identity exists" and "we never got
    /// through" both arrived as an empty success, clients would either retry a
    /// proven-empty scan forever or cache a network failure as fact.
    pub fn discovery_worth_retrying(self) -> bool {
        matches!(self, Self::PartialNoIdentity | Self::IdentityScanIncomplete)
    }

    /// Whether the identity question has an answer.
    ///
    /// Note this is NOT the inverse of [`Self::discovery_worth_retrying`]:
    /// [`Self::DiscoveryFailed`] leaves the question open *and* is not worth
    /// retrying, while [`Self::IdentityScanIncomplete`] has an answer that is
    /// merely known to be partial — an identity was found, so there is
    /// something to display. Use this to decide what to display, and
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
    /// Whether the inline DashPay sync pass ran **to completion**. `false`
    /// when it was skipped, failed, ran out of budget, or came back degraded —
    /// a pass that could not read some identities' contact documents leaves
    /// their account builds unenqueued, so it is not a pass the caller may
    /// rely on.
    pub dashpay_sync_ran: bool,
    /// The contact-account drain was skipped because the supplied
    /// contact-crypto provider does not resolve this wallet's seed. Nothing
    /// was derived and nothing was written; the queue is intact.
    pub seed_binding_unverified: bool,
    /// The wallet's gap-limit identity scan is on record as having left
    /// indices unanswered, and this launch's scan did not close the gap. The
    /// identities reported here are real; they may not be all of them.
    ///
    /// Carried separately from `status` because the status can only report one
    /// thing and a pending contact queue outranks this — a client that wants
    /// to surface "still looking for your other identities" reads the flag.
    pub identity_scan_incomplete: bool,
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
    /// The drain was skipped because the contact-crypto provider could not be
    /// shown to resolve this wallet's seed.
    pub seed_binding_unverified: bool,
    /// The recorded identity-scan verdict still says indices were left
    /// unanswered once discovery was done for this launch. Independent of
    /// `identity_id`: the gap is about the identities that were NOT found.
    pub identity_scan_incomplete: bool,
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

    /// The seed behind the contact-crypto provider could not be shown to own
    /// this wallet, so the drain never ran.
    pub(crate) fn record_seed_binding_unverified(&mut self) {
        self.seed_binding_unverified = true;
    }

    /// Discovery is done for this launch and the recorded scan verdict still
    /// says indices went unanswered.
    ///
    /// Read from the persisted verdict rather than inferred from the
    /// discovery counters, because the two are not the same question. The
    /// counters describe what *this* call did; the verdict describes what the
    /// wallet's identity set is known to be missing, and it survives a launch
    /// that never scanned at all. Only positive evidence sets it — an absent
    /// verdict is "unknown", never "incomplete".
    pub(crate) fn record_identity_scan_incomplete(&mut self) {
        self.identity_scan_incomplete = true;
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
        // Both of these say "the identity question is still open", so neither
        // may decide the verdict once an identity is known. A rescan forced
        // by an incomplete prior scan reaches them with an identity already
        // recorded, and reporting *that* launch as `DiscoveryFailed` would
        // hide a sync and drain that both ran.
        //
        // A local fault outranks unreachability: both leave the question open,
        // but only this one tells the client not to bother asking again.
        if self.discovery_failed_locally && self.identity_id.is_none() {
            return WalletStartupStatus::DiscoveryFailed;
        }
        if self.discovery_unreachable && self.identity_id.is_none() {
            return WalletStartupStatus::PartialNoIdentity;
        }
        if self.proven_no_identity && self.identity_id.is_none() {
            return WalletStartupStatus::NoIdentity;
        }
        // Outranks the queue counters, and must: a wrong-seed provider is why
        // the queue was not drained, and it is the one ending here that points
        // at a host misconfiguration rather than at Platform being slow. It
        // also has to outrank `Ready` — with an empty queue every other signal
        // would read as a clean run.
        if self.seed_binding_unverified {
            return WalletStartupStatus::SeedBindingUnverified;
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
        // Last, and deliberately so: every check above describes work this
        // launch did, while this one describes an identity set the wallet is
        // on record as not having fully established. Ranking it here is what
        // keeps the check additive — the only run it reclassifies is the one
        // that would otherwise come back `Ready`, which is precisely the run
        // that would be lying. Everything else keeps the status a client
        // already handles, and reads `identity_scan_incomplete` on the outcome
        // if it cares.
        //
        // `Ready` is the promise that a contact payment has everything it
        // needs. An unanswered index can hide a whole identity from every
        // consumer of local state, so a launch that knows its scan was partial
        // has not earned that word.
        if self.identity_scan_incomplete {
            return WalletStartupStatus::IdentityScanIncomplete;
        }
        WalletStartupStatus::Ready
    }

    pub(crate) fn into_outcome(self, elapsed: Duration) -> WalletStartupOutcome {
        WalletStartupOutcome {
            status: self.status(),
            identity_id: self.identity_id,
            discovery_attempts: self.discovery_attempts,
            dashpay_sync_ran: self.dashpay_sync_ran,
            seed_binding_unverified: self.seed_binding_unverified,
            identity_scan_incomplete: self.identity_scan_incomplete,
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
    /// `scan_key` and `contact_crypto` are borrowed for this call only and must
    /// not be retained by the caller afterwards. `scan_key` is invoked at most
    /// once, and only on the branch that actually scans — see
    /// [`ScanKeyResolver`] for why the caller must not resolve it up front.
    /// `identity_signer` is `None` to skip the DIP-15 auto-accept pass.
    ///
    /// The resolved key never outlives this call: it is erased before
    /// discovery returns.
    pub async fn start_wallet_subsystems<C, S>(
        &self,
        wallet_id: &WalletId,
        scan_key: Option<ScanKeyResolver<'_>>,
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
        //    scan it does not need — unless the scan that produced those
        //    identities is on record as having left indices unanswered, in
        //    which case "we already have one" is not evidence that we have
        //    them all. Without that exception a wallet whose second identity
        //    was hidden by a failed probe stays that way for the life of the
        //    installation, because this shortcut is the only thing that would
        //    look again.
        //
        //    Only a recorded incomplete scan re-opens the question. An absent
        //    verdict keeps the shortcut, so hosts that do not persist it are
        //    exactly where they were rather than paying for a scan every
        //    launch.
        let scan_incomplete = self.identity_scan_is_incomplete(wallet_id).await;
        match self.local_identity_id(wallet_id).await {
            Some(known) if !scan_incomplete => tally.record_local_identity(known),
            Some(known) => {
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    "startup: the last identity scan left indices unanswered; rescanning \
                     rather than trusting the identities already on file"
                );
                tally.record_local_identity(known);
                self.discover_identity_with_backoff(
                    wallet_id,
                    identity_wallet,
                    scan_key,
                    opts.gap_limit,
                    deadline,
                    &mut tally,
                )
                .await;
            }
            None => {
                self.discover_identity_with_backoff(
                    wallet_id,
                    identity_wallet,
                    scan_key,
                    opts.gap_limit,
                    deadline,
                    &mut tally,
                )
                .await;
            }
        }

        // Discovery is done for this launch; re-read the verdict it leaves
        // behind. Re-reading rather than inferring from the branch above is
        // what makes this correct in every ending: a rescan that closed the
        // gap publishes a complete verdict and this reads `false`, a rescan
        // that could not publishes (or leaves) an incomplete one, and a fresh
        // scan that came back partial without ever having a prior verdict is
        // caught too — it is the same defect, reached from the other side.
        //
        // Without this the tally has no way to express "an identity is known
        // and the set it belongs to is not", so a launch whose rescan was
        // unreachable arrived at `Ready`: the guard in `status()` requires
        // `identity_id.is_none()` before a discovery signal may decide the
        // verdict, and here an identity IS on file.
        if self.identity_scan_is_incomplete(wallet_id).await {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                "startup: the identity scan is still on record as incomplete; this launch \
                 cannot report a settled identity set"
            );
            tally.record_identity_scan_incomplete();
        }

        // With no identity there is nothing to sync and nothing to drain, and
        // that is true whether Platform proved absence or never answered.
        if !tally.has_identity() {
            return Ok(tally.into_outcome(started.elapsed()));
        }

        // 2. One contact-request pass, so the deferred builds exist to drain.
        //    Log-and-continue: a prior session may already have queued work
        //    that this call can still complete.
        match within_budget(
            deadline,
            identity_wallet.dashpay().sync_contact_requests_reporting(),
        )
        .await
        {
            Some(Ok(report)) if report.is_complete() => {
                tally.record_sync_ran();
                tracing::debug!(
                    wallet_id = %hex::encode(wallet_id),
                    requests = report.requests.len(),
                    identities = report.identities_attempted,
                    "startup: contact-request pass complete"
                );
            }
            // Reached Platform for some identities and not others (or for none
            // at all), or reached them all and could not write what came back.
            // The requests it did fetch AND persist are real, but the
            // identities it missed have contact requests nobody has looked at,
            // whose account builds were therefore never enqueued — so the
            // queue being empty below proves nothing. Not recording the pass
            // keeps `status()` off `Ready`, which is the promise that every
            // contact's DIP-15 addresses exist before Core SPV starts.
            //
            // The failures retry themselves whichever door they came through:
            // a fetch that errored and an ingest that could not persist BOTH
            // leave that direction's high-water cursor unadvanced, so the next
            // sweep re-requests exactly the range this pass missed.
            Some(Ok(report)) => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    requests = report.requests.len(),
                    identities = report.identities_attempted,
                    failed = report.failed_identities.len(),
                    degraded = report.degraded_identities.len(),
                    unpersisted = report.unpersisted_identities.len(),
                    "startup: contact-request pass was degraded; not recording it as a \
                     completed sync"
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
        // Both drains hit the network, so both are bounded — but by the
        // deadline they take as a parameter, NOT by `within_budget`. They
        // commit per-entry side effects as they go and apply the dequeue list
        // once at the end, so dropping one mid-loop would strand work that
        // really happened and report it as zero. Bounding them from the inside
        // stops the loop between entries instead: what they return and what
        // they dequeue always describe work that completed, and entries they
        // never reached stay queued for the next signer-present action.
        //
        // Without a provider there is nothing to drain WITH. A caller that
        // passes `None` gets the sequence's other steps and an honest
        // `contact_accounts_pending`, rather than a drain that reports zero
        // because every crypto operation failed.
        //
        // The seed-binding gate in front of both drains is NOT applied here:
        // it lives inside
        // [`PlatformWallet::drain_pending_contact_crypto_verified`], the one
        // primitive this sequence and the FFI drain entry point share. Keeping
        // it there rather than in each caller is the whole point — a client
        // that has to remember to gate the call is a client that will
        // eventually forget, which is exactly how the FFI entry point came to
        // have no gate while iOS enforced one in its Swift wrapper. The only
        // error it can return is a failed verification (the drains themselves
        // report counts, never errors), so an `Err` here means precisely "the
        // provider was not shown to own this wallet".
        let drained = match contact_crypto {
            Some(contact_crypto) => match wallet
                .drain_pending_contact_crypto_verified(
                    contact_crypto,
                    identity_signer,
                    Some(deadline),
                )
                .await
            {
                Ok(drained) => drained,
                Err(e) => {
                    tally.record_seed_binding_unverified();
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        error = %e,
                        "startup: the contact-crypto drain was refused; the supplied provider \
                         does not bind to this wallet's seed"
                    );
                    0
                }
            },
            None => {
                tracing::info!(
                    wallet_id = %hex::encode(wallet_id),
                    "startup: no contact-crypto provider; skipping the drain"
                );
                0
            }
        };
        // Not budgeted: a local queue-length read with no I/O. Leaving it
        // unbounded keeps the reported `pending` truthful even when the steps
        // above ran out of time — which is exactly when it matters most.
        let pending = identity_wallet
            .dashpay()
            .pending_contact_crypto_count()
            .await;
        tally.record_drain(drained, pending);

        if pending > 0 {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                pending,
                "startup: contact-account builds still queued; the DIP-15 rescan will backfill"
            );
        }

        Ok(tally.into_outcome(started.elapsed()))
    }

    /// Whether this wallet's last gap-limit scan is on record as having left
    /// indices unanswered.
    ///
    /// `false` when no verdict is known — see
    /// [`IdentityManager::identity_scan_is_incomplete`] for why "unknown" must
    /// not read as "incomplete".
    ///
    /// [`IdentityManager::identity_scan_is_incomplete`]: crate::wallet::identity::IdentityManager::identity_scan_is_incomplete
    async fn identity_scan_is_incomplete(&self, wallet_id: &WalletId) -> bool {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(wallet_id)
            .is_some_and(|info| info.identity_manager.identity_scan_is_incomplete(wallet_id))
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
    /// — the error for a scan that never got through — is worth another
    /// attempt.
    async fn discover_identity_with_backoff(
        &self,
        wallet_id: &WalletId,
        identity_wallet: &crate::wallet::identity::IdentityWallet,
        scan_key: Option<ScanKeyResolver<'_>>,
        gap_limit: Option<u32>,
        deadline: Instant,
        tally: &mut StartupTally,
    ) {
        // This is the branch that scans, so this is where the key gets
        // resolved — not one moment earlier. A failure here is "come back
        // later", never a verdict: the realistic causes are a locked device or
        // a denied Keychain read, and deriving anyway against a wallet that
        // holds no private key would fail locally and be classified terminal,
        // telling the client to stop trying over a condition that clears
        // itself on the next unlock.
        let master = match scan_key {
            None => None,
            Some(resolve) => match resolve() {
                Ok(master) => Some(ScanKeyGuard(master)),
                // Retryable: the key is there, this moment was wrong.
                Err(e @ ScanKeyError::Unavailable(_)) => {
                    tracing::warn!(
                        error = %e,
                        "startup: scan key unavailable; deferring discovery to a later start"
                    );
                    tally.record_unreachable();
                    tally.record_discovery_gave_up();
                    return;
                }
                // Terminal: retrying re-reads the same bad material. Reported
                // like any other local fault, because that is what it is.
                Err(e @ ScanKeyError::Invalid(_)) => {
                    tracing::warn!(
                        error = %e,
                        "startup: scan key material is not usable; no later scan can fix it"
                    );
                    tally.record_discovery_failed_locally();
                    return;
                }
            },
        };

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
                match master.as_ref() {
                    // `Some(deadline)` bounds the DPNS enrichment tail only,
                    // never the scan. The outer `within_budget` below still
                    // owns the scan's ceiling; what this adds is a way for
                    // enrichment to stop on its own, so a slow DPNS pass no
                    // longer gets the whole call cancelled out from under a
                    // scan that already published a complete verdict — which
                    // arrives here as a scan that recorded nothing, sending
                    // `record_identity_scan_cut_off` to overwrite it and set a
                    // sticky `unlocated_gap`.
                    Some(master) => {
                        identity_wallet
                            .discover_from_master_until(opts, master, Some(deadline))
                            .await
                    }
                    None => identity_wallet.discover_until(opts, Some(deadline)).await,
                }
            };
            let Some(result) = within_budget(deadline, attempt_future).await else {
                // Dropped mid-await, so the scan recorded no verdict of its
                // own. Record one here: an abandoned scan probed an unknown
                // prefix of the index space and answered the rest of it not at
                // all, which is exactly the state a later launch must not
                // mistake for a settled identity set. Without this the
                // budget-expiry path hides a second identity in its own right
                // — it consults local state, finds the sighting that was
                // persisted before cancellation, and records a warm launch.
                self.record_identity_scan_cut_off(wallet_id).await;
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
                    let identity = match found.first() {
                        Some(identity) => Some(identity.id()),
                        // An empty return is not proof on its own: `discover`
                        // reports only identities THIS call inserted, so a
                        // concurrent startup that inserted one first leaves us
                        // seeing it as already-managed and returning nothing.
                        // Consult local state before calling it absence.
                        None => self.local_identity_id(wallet_id).await,
                    };
                    let Some(identity) = identity else {
                        tally.record_proven_absent();
                        return;
                    };
                    tally.record_discovered(identity);
                    // `Ok` does not mean "every index was answered": a scan
                    // that saw an identity is reported as trustworthy even
                    // when a later probe went unanswered, and an identity
                    // hiding at that index is invisible until something scans
                    // again. Retry it here, inside the budget the caller
                    // already granted and with the scan key already resolved,
                    // rather than leaving it to a launch that may never come.
                    if !self.identity_scan_is_incomplete(wallet_id).await {
                        return;
                    }
                    if backoff.is_none() {
                        // Out of attempts. The verdict stays on record, so the
                        // next launch re-opens the question instead of taking
                        // the warm shortcut.
                        tracing::warn!(
                            "startup: identity discovery still has unanswered indices after \
                             every attempt; the recorded verdict will force a rescan"
                        );
                        return;
                    }
                    tracing::info!(
                        attempt = attempt + 1,
                        "startup: identity discovery left indices unanswered; rescanning"
                    );
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
                    // The scan folds each identity in as it finds it, so a
                    // fault raised after one was inserted leaves that identity
                    // resident and its keys in memory. The failure says
                    // persistence broke, NOT that the wallet owns nothing —
                    // reporting it as identity-less would exit at the
                    // no-identity guard and skip the contact sync and drain
                    // for an identity that is right there. Both signals are
                    // kept: the identity is reported and the local fault stays
                    // on record.
                    if let Some(known) = self.local_identity_id(wallet_id).await {
                        tally.record_local_identity(known);
                    }
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

        // Only meaningful while the identity question is still open. The
        // partial-scan retry above can exhaust the loop with an identity
        // already recorded, and that launch is not an unreachable-Platform
        // launch — it found something, it just could not prove it found
        // everything.
        if !tally.has_identity() {
            tally.record_discovery_gave_up();
        }
    }

    /// Record that a scan was abandoned before it could answer every index.
    ///
    /// Mirrors what `discover` publishes for itself; needed separately because
    /// a scan dropped mid-await never reaches its own bookkeeping.
    async fn record_identity_scan_cut_off(&self, wallet_id: &WalletId) {
        // Coverage of nothing: the scan was dropped mid-await, so it answered
        // no index and may not clear one an earlier scan left open.
        let recorded = {
            let mut wm = self.wallet_manager.write().await;
            match wm.get_wallet_info_mut(wallet_id) {
                Some(info) => info.identity_manager.record_identity_scan(
                    *wallet_id,
                    crate::changeset::IdentityScanStateEntry::incomplete(0, 0, Vec::new()),
                ),
                None => return,
            }
        };
        let changeset = crate::changeset::PlatformWalletChangeSet {
            identity_scan_state: Some(recorded),
            ..Default::default()
        };
        if let Err(e) = self.persister.store(*wallet_id, changeset) {
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error = %e,
                "failed to persist an abandoned scan's verdict; the next launch may take the \
                 warm shortcut over an incomplete identity set"
            );
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

    /// The opposite case, and the reason the distinction exists at all: never
    /// reaching Platform is not evidence of absence.
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
            WalletStartupStatus::SeedBindingUnverified,
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

    /// A contact pass that could not read some identities' documents is not a
    /// completed pass, and the whole point of tracking that is to keep it off
    /// `Ready`. `Ready` promises the DIP-15 addresses exist before Core SPV
    /// starts; a degraded pass never enqueued the account builds for the
    /// identities it missed, so the queue being empty proves nothing.
    #[test]
    fn a_degraded_contact_pass_is_not_ready_even_with_an_empty_queue() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        // Deliberately no `record_sync_ran` — this is what startup does when
        // the report comes back degraded.
        tally.record_drain(0, 0);

        assert_eq!(tally.status(), WalletStartupStatus::PartialAccountsPending);
        assert!(!tally.dashpay_sync_ran);
    }

    /// The wrong-seed ending outranks every other non-discovery verdict,
    /// including a clean-looking drain. With an empty queue the run is
    /// otherwise indistinguishable from a healthy one, and reporting it as
    /// `Ready` would hide the single condition here that points at a host
    /// misconfiguration rather than at Platform being slow.
    #[test]
    fn an_unverified_seed_binding_outranks_a_clean_drain() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        tally.record_sync_ran();
        tally.record_seed_binding_unverified();
        tally.record_drain(0, 0);

        assert_eq!(tally.status(), WalletStartupStatus::SeedBindingUnverified);
        assert!(
            tally.status().identity_is_settled(),
            "the identity was found; it is the drain that did not run"
        );
        assert!(!tally.status().discovery_worth_retrying());
    }

    /// The outcome carries the flag so a client can tell "nothing was queued"
    /// from "we refused to derive".
    #[test]
    fn an_unverified_seed_binding_reaches_the_outcome() {
        let mut tally = StartupTally::default();
        tally.record_discovered(identity());
        tally.record_sync_ran();
        tally.record_seed_binding_unverified();

        let outcome = tally.into_outcome(Duration::from_secs(1));
        assert!(outcome.seed_binding_unverified);
        assert_eq!(outcome.status, WalletStartupStatus::SeedBindingUnverified);
    }

    /// A rescan forced by an incomplete prior scan can reach the
    /// discovery-failure branches with an identity already on file. Those
    /// statuses say "the identity question is still open", which would be a
    /// lie here — and it would also hide a sync and drain that both ran. But
    /// the rescan failing is not nothing either: it means the scan gap that
    /// forced it is still there.
    ///
    /// Asserting `Ready` for the unreachable half would pin the very defect
    /// the `identity_scan_incomplete` signal exists to close — a launch that
    /// knows its identity set is partial reporting the status that promises it
    /// is complete. Both halves keep their real subject (the identity must not
    /// be re-opened) and assert the gap is reported.
    #[test]
    fn a_failed_rescan_reports_the_scan_gap_without_reopening_the_identity() {
        // The scenario the name describes: the prior verdict said incomplete,
        // which is the only reason a rescan ran at all, and it is still
        // incomplete afterwards.
        let mut unreachable = StartupTally::default();
        unreachable.record_local_identity(identity());
        unreachable.record_unreachable();
        unreachable.record_discovery_gave_up();
        unreachable.record_identity_scan_incomplete();
        unreachable.record_sync_ran();
        unreachable.record_drain(1, 0);
        assert_eq!(
            unreachable.status(),
            WalletStartupStatus::IdentityScanIncomplete,
            "a launch whose rescan never closed the gap has not established the identity set"
        );
        assert!(
            unreachable.status().identity_is_settled(),
            "the identity that WAS found is real; only the set around it is open"
        );
        assert!(
            unreachable.status().discovery_worth_retrying(),
            "the unanswered indices are exactly what another scan could answer"
        );

        let mut local_fault = StartupTally::default();
        local_fault.record_local_identity(identity());
        local_fault.record_discovery_failed_locally();
        local_fault.record_identity_scan_incomplete();
        local_fault.record_sync_ran();
        local_fault.record_drain(0, 2);
        assert_eq!(
            local_fault.status(),
            WalletStartupStatus::PartialAccountsPending,
            "a pending contact queue still outranks the scan gap in the status"
        );
        assert!(
            local_fault.status().identity_is_settled(),
            "a local discovery fault must not re-open an identity that is on file"
        );
    }

    /// The corrected verdict, isolated: an otherwise perfectly clean run — an
    /// identity, a completed contact pass, an empty queue — is still not
    /// `Ready` while the scan that produced that identity is on record as
    /// having left indices unanswered. `Ready` promises a settled identity
    /// set, and this run cannot promise one.
    #[test]
    fn an_incomplete_scan_keeps_an_otherwise_clean_run_off_ready() {
        let mut tally = StartupTally::default();
        tally.record_local_identity(identity());
        tally.record_sync_ran();
        tally.record_identity_scan_incomplete();
        tally.record_drain(1, 0);

        assert_eq!(tally.status(), WalletStartupStatus::IdentityScanIncomplete);

        let outcome = tally.into_outcome(Duration::from_secs(1));
        assert!(
            outcome.identity_scan_incomplete,
            "the flag must reach the client even where the status is outranked"
        );
        assert_eq!(outcome.identity_id, Some(identity()));
    }

    /// The other direction, and the reason the check reads the recorded
    /// verdict rather than the discovery counters: the identical run with a
    /// scan that answered every index it probed IS `Ready`. Without this the
    /// test above would keep passing if the signal were stuck on.
    #[test]
    fn a_complete_scan_reaches_ready() {
        let mut tally = StartupTally::default();
        tally.record_local_identity(identity());
        tally.record_sync_ran();
        tally.record_drain(1, 0);

        assert_eq!(tally.status(), WalletStartupStatus::Ready);

        let outcome = tally.into_outcome(Duration::from_secs(1));
        assert!(!outcome.identity_scan_incomplete);
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

    // ---------------------------------------------------------------------
    // End-to-end: the seed-binding gate in front of the drain.
    //
    // Driven through the real `start_wallet_subsystems` over a mock SDK, so
    // what is asserted is the sequence's actual behaviour rather than a
    // restatement of the tally rules above.
    // ---------------------------------------------------------------------

    /// Canonical all-`abandon` BIP-39 vector — the seed
    /// `test_platform_wallet_manager` builds its wallet from.
    const OWNING_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";
    /// A different valid BIP-39 vector: the mis-mapped Keychain slot.
    const FOREIGN_MNEMONIC: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";

    /// Only ever passed as `None`, so the sequence skips the DIP-15
    /// auto-accept pass — but the generic still has to be named.
    #[derive(Debug)]
    struct UnusedSigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for UnusedSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("the auto-accept pass is never reached with a None signer")
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("the auto-accept pass is never reached with a None signer")
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            false
        }
    }

    fn seed_for(phrase: &str) -> [u8; 64] {
        use key_wallet::mnemonic::{Language, Mnemonic};
        Mnemonic::from_phrase(phrase, Language::English)
            .expect("valid test mnemonic")
            .to_seed("")
    }

    fn test_identity(id_byte: u8) -> dpp::identity::Identity {
        use dpp::identity::v0::IdentityV0;
        dpp::identity::Identity::V0(IdentityV0 {
            id: Identifier::from([id_byte; 32]),
            public_keys: std::collections::BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    /// A manager holding one wallet that owns one identity with a single
    /// queued `RegisterReceiving` op — the smallest state in which the drain
    /// has real work, and the op that derives a contact receiving xpub
    /// straight from the provider with no network round trip.
    async fn manager_with_queued_contact_crypto() -> (
        std::sync::Arc<crate::PlatformWalletManager<crate::test_support::NoopTestPersister>>,
        WalletId,
    ) {
        use crate::changeset::{
            upsert_pending_contact_crypto, PendingContactCrypto, PendingContactCryptoOp,
        };
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};

        let (manager, wallet_id) = crate::test_support::test_platform_wallet_manager().await;
        let persister = WalletPersister::new(wallet_id, std::sync::Arc::new(NoPlatformPersistence));

        let mut wm = manager.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        info.identity_manager
            .add_identity(test_identity(1), 0, wallet_id, &persister)
            .expect("add identity");
        let managed = info
            .identity_manager
            .managed_identity_mut(&Identifier::from([1u8; 32]))
            .expect("managed identity");
        upsert_pending_contact_crypto(
            managed.dashpay_pending_contact_crypto_mut(),
            PendingContactCrypto {
                owner_identity_id: Identifier::from([1u8; 32]),
                contact_id: Identifier::from([2u8; 32]),
                op: PendingContactCryptoOp::RegisterReceiving,
                enqueued_at_ms: 0,
            },
        );
        drop(wm);

        (manager, wallet_id)
    }

    /// Count the DashPay receiving accounts the wallet is watching. The thing
    /// a wrong-seed drain would corrupt: `register_contact_account` keys its
    /// existence check on `(index, us, them)` and NOT on the xpub, so an
    /// account written from the wrong seed is never revisited.
    async fn receiving_account_count(
        manager: &crate::PlatformWalletManager<crate::test_support::NoopTestPersister>,
        wallet_id: &WalletId,
    ) -> usize {
        let wm = manager.wallet_manager.read().await;
        wm.get_wallet_info(wallet_id)
            .map(|info| info.core_wallet.accounts.dashpay_receival_accounts.len())
            .unwrap_or(0)
    }

    async fn drainable(
        manager: &crate::PlatformWalletManager<crate::test_support::NoopTestPersister>,
        wallet_id: &WalletId,
    ) -> usize {
        let wallet = manager.get_wallet(wallet_id).await.expect("wallet");
        wallet
            .identity()
            .dashpay()
            .drainable_contact_crypto_count()
            .await
    }

    /// The defect this gate closes: a provider resolving someone else's seed
    /// derives contact receiving xpubs that are written once and never
    /// corrected, so the wallet watches addresses nobody pays to. The drain
    /// must not run at all, and the queue must survive intact for the next
    /// signer-present attempt.
    #[tokio::test]
    async fn a_wrong_seed_provider_never_reaches_the_drain() {
        use crate::wallet::identity::network::SeedCryptoProvider;

        let (manager, wallet_id) = manager_with_queued_contact_crypto().await;
        assert_eq!(receiving_account_count(&manager, &wallet_id).await, 0);
        assert_eq!(drainable(&manager, &wallet_id).await, 1);

        let foreign =
            SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), key_wallet::Network::Testnet);
        let outcome = manager
            .start_wallet_subsystems(
                &wallet_id,
                None,
                Some(&foreign),
                None::<&UnusedSigner>,
                WalletStartupOptions::default(),
            )
            .await
            .expect("a wrong seed is reported, not raised");

        assert_eq!(outcome.status, WalletStartupStatus::SeedBindingUnverified);
        assert!(outcome.seed_binding_unverified);
        assert_eq!(
            outcome.contact_accounts_drained, 0,
            "nothing may be drained with a provider that does not own the wallet"
        );
        assert_eq!(
            receiving_account_count(&manager, &wallet_id).await,
            0,
            "not one contact account may be registered from the wrong seed"
        );
        assert_eq!(
            drainable(&manager, &wallet_id).await,
            1,
            "the queue must survive so the next signer-present drain can do the work"
        );
    }

    /// The other half: the wallet's own seed passes the gate and the drain
    /// runs. Without this the test above would also pass if the gate simply
    /// refused everything.
    #[tokio::test]
    async fn the_owning_seed_passes_the_gate_and_the_drain_runs() {
        use crate::wallet::identity::network::SeedCryptoProvider;

        let (manager, wallet_id) = manager_with_queued_contact_crypto().await;
        let owning =
            SeedCryptoProvider::from_seed(seed_for(OWNING_MNEMONIC), key_wallet::Network::Testnet);

        let outcome = manager
            .start_wallet_subsystems(
                &wallet_id,
                None,
                Some(&owning),
                None::<&UnusedSigner>,
                WalletStartupOptions::default(),
            )
            .await
            .expect("bring-up reports rather than raises");

        assert!(
            !outcome.seed_binding_unverified,
            "the wallet's own seed must bind"
        );
        assert_ne!(outcome.status, WalletStartupStatus::SeedBindingUnverified);
        assert_eq!(
            outcome.contact_accounts_drained, 1,
            "the queued RegisterReceiving op must have been completed"
        );
        assert_eq!(
            receiving_account_count(&manager, &wallet_id).await,
            1,
            "the contact receiving account must exist after a verified drain"
        );
    }

    /// The gate is paid for only when there is something to protect. An empty
    /// queue means the drain would derive nothing, so no key material is
    /// resolved — which is what keeps this affordable on a warm launch.
    /// Proven with a provider that would FAIL the check: reaching a status
    /// other than `SeedBindingUnverified` shows it was never consulted.
    #[tokio::test]
    async fn an_empty_queue_skips_the_gate_entirely() {
        use crate::changeset::{PendingContactCryptoKey, PendingContactCryptoKind};
        use crate::wallet::identity::network::SeedCryptoProvider;

        let (manager, wallet_id) = manager_with_queued_contact_crypto().await;
        // Empty the queue so the drain has nothing to do.
        {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            let managed = info
                .identity_manager
                .managed_identity_mut(&Identifier::from([1u8; 32]))
                .expect("managed identity");
            let key = PendingContactCryptoKey {
                owner_identity_id: Identifier::from([1u8; 32]),
                contact_id: Identifier::from([2u8; 32]),
                kind: PendingContactCryptoKind::RegisterReceiving,
            };
            managed
                .dashpay_pending_contact_crypto_mut()
                .retain(|e| e.key() != key);
        }
        assert_eq!(drainable(&manager, &wallet_id).await, 0);

        let foreign =
            SeedCryptoProvider::from_seed(seed_for(FOREIGN_MNEMONIC), key_wallet::Network::Testnet);
        let outcome = manager
            .start_wallet_subsystems(
                &wallet_id,
                None,
                Some(&foreign),
                None::<&UnusedSigner>,
                WalletStartupOptions::default(),
            )
            .await
            .expect("bring-up reports rather than raises");

        assert!(
            !outcome.seed_binding_unverified,
            "with nothing to drain the binding check must not run at all"
        );
    }

    /// The F1 regression, end to end and against a Platform that answers
    /// nothing (the mock SDK fails every contact fetch, which is exactly the
    /// DAPI-unreachable shape).
    ///
    /// Before the fix this pass returned `Ok(vec![])`, startup called
    /// `record_sync_ran`, and a wallet whose contacts had never been read
    /// reported `Ready` — the status that promises every contact's DIP-15
    /// addresses exist before Core SPV starts.
    #[tokio::test]
    async fn a_contact_pass_that_reached_nobody_is_not_a_completed_sync() {
        use crate::wallet::identity::network::SeedCryptoProvider;

        let (manager, wallet_id) = manager_with_queued_contact_crypto().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");

        // The pass itself: one identity attempted, none reached.
        let report = wallet
            .identity()
            .dashpay()
            .sync_contact_requests_reporting()
            .await
            .expect("the pass returns a report");
        assert_eq!(report.identities_attempted, 1);
        assert_eq!(report.failed_identities.len(), 1);
        assert!(!report.is_complete());
        assert!(report.is_fully_degraded());

        // The back-compat return shape must not render this as success.
        let err = wallet
            .identity()
            .dashpay()
            .sync_contact_requests()
            .await
            .expect_err("reaching nobody must not look like an empty result");
        assert!(
            matches!(
                err,
                PlatformWalletError::ContactSyncUnreachable { identities: 1 }
            ),
            "expected ContactSyncUnreachable, got: {err:?}"
        );

        // The cursors stayed put, so the next sweep re-requests the same
        // range — this is what makes the failure retried rather than buried.
        {
            let wm = manager.wallet_manager.read().await;
            let managed = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet info")
                .identity_manager
                .managed_identity(&Identifier::from([1u8; 32]))
                .expect("managed identity");
            assert_eq!(
                managed.dashpay().high_water_received_ms(),
                None,
                "a failed fetch must not advance the cursor past requests it never read"
            );
            assert_eq!(managed.dashpay().high_water_sent_ms(), None);
        }

        // And the sequence must not record it as a sync that ran.
        let owning =
            SeedCryptoProvider::from_seed(seed_for(OWNING_MNEMONIC), key_wallet::Network::Testnet);
        let outcome = manager
            .start_wallet_subsystems(
                &wallet_id,
                None,
                Some(&owning),
                None::<&UnusedSigner>,
                WalletStartupOptions::default(),
            )
            .await
            .expect("bring-up reports rather than raises");

        assert!(
            !outcome.dashpay_sync_ran,
            "a pass that read none of the wallet's identities is not a completed sync"
        );
        assert_ne!(
            outcome.status,
            WalletStartupStatus::Ready,
            "Ready promises contact addresses this call never prepared"
        );
        assert_eq!(outcome.status, WalletStartupStatus::PartialAccountsPending);
    }

    /// The wire-up, end to end: the sequence reads the wallet's RECORDED scan
    /// verdict once discovery is done and carries it out on the outcome.
    ///
    /// Reading the verdict rather than inferring from this call's discovery
    /// counters is the point — a launch that took the warm shortcut, or whose
    /// rescan was abandoned before it started, still has to report the gap the
    /// wallet is on record as having, and neither of those launches has a
    /// discovery counter to infer it from.
    ///
    /// Driven with a zero budget so no branch depends on network timing: every
    /// step is abandoned at its deadline and what is asserted is purely which
    /// verdict came out. ("No verdict at all" is not reachable here — the
    /// harness's mock SDK answers no probe, so creating the wallet already
    /// leaves one — and it is the accessor's own documented contract that an
    /// absent verdict reads as unknown rather than incomplete.)
    #[tokio::test]
    async fn a_recorded_incomplete_scan_reaches_the_outcome() {
        use crate::changeset::IdentityScanStateEntry;
        use crate::wallet::identity::network::SeedCryptoProvider;

        let no_time = WalletStartupOptions {
            budget: Duration::ZERO,
            gap_limit: None,
        };

        let (manager, wallet_id) = manager_with_queued_contact_crypto().await;
        let owning =
            SeedCryptoProvider::from_seed(seed_for(OWNING_MNEMONIC), key_wallet::Network::Testnet);

        // The wallet arrives with an unanswered index on record — a real
        // incomplete scan, produced by the mock SDK refusing every probe
        // during wallet creation, not a hand-planted flag.
        let covered_through;
        {
            let wm = manager.wallet_manager.read().await;
            let verdict = wm
                .get_wallet_info(&wallet_id)
                .expect("wallet info")
                .identity_manager
                .identity_scan_state(&wallet_id)
                .cloned()
                .expect("precondition: the creation scan recorded a verdict");
            assert!(
                !verdict.complete,
                "precondition: that verdict must be the incomplete one"
            );
            covered_through = verdict.probed_through;
        }

        let outcome = manager
            .start_wallet_subsystems(
                &wallet_id,
                None,
                Some(&owning),
                None::<&UnusedSigner>,
                no_time,
            )
            .await
            .expect("bring-up reports rather than raises");

        assert!(
            outcome.identity_scan_incomplete,
            "the recorded gap must reach the client: {outcome:?}"
        );
        assert_ne!(
            outcome.status,
            WalletStartupStatus::Ready,
            "Ready promises an identity set this launch did not establish"
        );
        assert!(
            outcome.identity_id.is_some(),
            "the identity that IS known must still be reported"
        );

        // The other direction, through the same sequence: once the scan is on
        // record as having answered everything it probed, the signal clears.
        // Without this the assertion above would keep passing if the flag were
        // simply stuck on.
        {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            // Coverage matters: a verdict only clears the gaps it walked, so
            // this stand-in for a clean rescan has to span the same indices
            // the creation scan left unanswered.
            info.identity_manager.record_identity_scan(
                wallet_id,
                IdentityScanStateEntry::completed(0, covered_through),
            );
        }

        let outcome = manager
            .start_wallet_subsystems(
                &wallet_id,
                None,
                Some(&owning),
                None::<&UnusedSigner>,
                no_time,
            )
            .await
            .expect("bring-up reports rather than raises");

        assert!(
            !outcome.identity_scan_incomplete,
            "a complete verdict leaves nothing to report: {outcome:?}"
        );
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
    // ---------------------------------------------------------------------
    // Discovery mutates identity state incrementally, so a fault raised
    // part-way through does not mean the wallet owns nothing.
    // ---------------------------------------------------------------------

    /// Persister handing back one already-persisted wallet, the way a restored
    /// launch hydrates. Its wallet is `ExternalSignable` — the Keychain-backed
    /// shape whose seed lives outside the process — which is what makes the
    /// resident-key derive inside `discover` fail with a LOCAL fault rather
    /// than an unreachable-Platform one.
    struct RestoredWalletPersister {
        wallet: key_wallet::Wallet,
        managed: key_wallet::wallet::ManagedWalletInfo,
    }

    impl crate::changeset::PlatformWalletPersistence for RestoredWalletPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: crate::changeset::PlatformWalletChangeSet,
        ) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }

        fn load(
            &self,
        ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError>
        {
            let mut wallets = std::collections::BTreeMap::new();
            wallets.insert(
                self.wallet.compute_wallet_id(),
                crate::changeset::ClientWalletStartState {
                    wallet: self.wallet.clone(),
                    wallet_info: self.managed.clone(),
                    identity_manager: crate::changeset::IdentityManagerStartState::default(),
                    unused_asset_locks: std::collections::BTreeMap::new(),
                },
            );
            Ok(crate::changeset::ClientStartState {
                wallets,
                ..Default::default()
            })
        }
    }

    struct RestoreEventHandler;
    impl crate::events::EventHandler for RestoreEventHandler {}
    impl crate::events::PlatformEventHandler for RestoreEventHandler {}

    /// A hydrated manager holding one external-signable wallet: a scan on it
    /// cannot derive from resident key material, so `discover` returns a local
    /// fault instead of an unanswered-probe one.
    async fn manager_whose_scan_faults_locally() -> (
        std::sync::Arc<crate::PlatformWalletManager<RestoredWalletPersister>>,
        WalletId,
    ) {
        let ctx = key_wallet::test_utils::TestWalletContext::new_random();
        let mut wallet = ctx.wallet;
        wallet.downgrade_to_external_signable();
        let wallet_id = wallet.compute_wallet_id();

        let manager = std::sync::Arc::new(crate::PlatformWalletManager::new(
            std::sync::Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
            std::sync::Arc::new(RestoredWalletPersister {
                wallet,
                managed: ctx.managed_wallet,
            }),
            std::sync::Arc::new(RestoreEventHandler)
                as std::sync::Arc<dyn crate::events::PlatformEventHandler>,
        ));
        manager
            .load_from_persistor()
            .await
            .expect("the restored wallet must hydrate");

        (manager, wallet_id)
    }

    /// The defect: `discover` folds each identity in as it finds it —
    /// `add_identity` inserts it and installs its public keys in memory —
    /// before the persist that can still fail. When that failure surfaced
    /// here, the branch recorded only the local-fault signal and left
    /// `identity_id` unset, so the sequence exited at the no-identity guard
    /// and reported a wallet with no identity while one was resident. The
    /// contact sync and the contact-account drain were skipped for it.
    ///
    /// Reproduced by driving the retry loop directly against a wallet whose
    /// scan faults locally, with the identity already folded in — the exact
    /// state the failing persist leaves behind.
    #[tokio::test]
    async fn a_local_scan_fault_still_reports_an_identity_the_scan_had_already_folded_in() {
        use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};

        let (manager, wallet_id) = manager_whose_scan_faults_locally().await;

        // What the scan had already done before its persist failed.
        {
            let persister =
                WalletPersister::new(wallet_id, std::sync::Arc::new(NoPlatformPersistence));
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(test_identity(1), 0, wallet_id, &persister)
                .expect("fold the identity in the way discovery does");
        }

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet handle");
        let mut tally = StartupTally::default();
        manager
            .discover_identity_with_backoff(
                &wallet_id,
                wallet.identity(),
                None,
                Some(1),
                Instant::now() + Duration::from_secs(30),
                &mut tally,
            )
            .await;

        assert!(
            tally.discovery_failed_locally,
            "precondition: this scan must fault locally, not go unanswered"
        );
        assert_eq!(
            tally.identity_id,
            Some(Identifier::from([1u8; 32])),
            "an identity the scan already folded in must still be reported"
        );
        assert!(
            tally.has_identity(),
            "the no-identity guard is what skips the contact sync and the drain"
        );
        assert_ne!(
            tally.status(),
            WalletStartupStatus::DiscoveryFailed,
            "a launch that HAS an identity is not a no-identity launch"
        );
    }

    /// The other direction, and the reason the branch re-reads local state
    /// rather than assuming: with nothing folded in, the identical local fault
    /// still settles as `DiscoveryFailed`. Without this the test above would
    /// keep passing if the branch reported an identity unconditionally.
    #[tokio::test]
    async fn a_local_scan_fault_with_nothing_folded_in_is_still_a_failed_discovery() {
        let (manager, wallet_id) = manager_whose_scan_faults_locally().await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet handle");
        let mut tally = StartupTally::default();
        manager
            .discover_identity_with_backoff(
                &wallet_id,
                wallet.identity(),
                None,
                Some(1),
                Instant::now() + Duration::from_secs(30),
                &mut tally,
            )
            .await;

        assert!(tally.discovery_failed_locally);
        assert!(
            !tally.has_identity(),
            "there is no identity to report, and none may be invented"
        );
        assert_eq!(tally.status(), WalletStartupStatus::DiscoveryFailed);
    }
}
