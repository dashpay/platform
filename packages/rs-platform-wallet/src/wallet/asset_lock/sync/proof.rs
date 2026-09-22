//! Proof lifecycle: waiting for proofs, validation, and IS-lock to ChainLock upgrade.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::{BlockHash, OutPoint, Txid};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::locate::{chain_proof_height_from_lookup, HeaderLookup, Located};

/// Least time between two mined-height lookups of the same transaction while a
/// ChainLock-proof wait is parked. Lock events arrive every block and per
/// InstantSend lock; this keeps them from turning into a DAPI request each.
const LOCATE_MIN_INTERVAL: Duration = Duration::from_secs(30);

/// How long a parked ChainLock-proof wait sleeps without a lock event before
/// re-checking, so a DAPI outage or an SPV disconnect that heals is noticed
/// without waiting for the next event.
const LOCATE_RETRY_INTERVAL: Duration = Duration::from_secs(60);

/// Longest a single mined-height lookup may hold the ChainLock-proof wait.
/// The wait cannot see lock events or its deadline while a lookup is
/// in flight, so a stalled DAPI node must not be able to stretch that window.
/// One attempt makes two requests (the placement, then the block).
const LOCATE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(20);

/// Longest a single SPV header read may hold the ChainLock-proof wait. Its own
/// cap, not [`LOCATE_ATTEMPT_TIMEOUT`]: the lookup that produced the placement
/// has already spent that budget, and the production header read takes three
/// locks, so a stalled reader must not push the wait past its deadline.
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// The outcome of a bounded SPV header read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HeaderRead {
    /// The store answered — with a header, with "nothing there", or with a
    /// reason it could not answer.
    Answer(HeaderLookup),
    /// The read did not finish within the budget it was given.
    Stalled(Duration),
}

/// Whether a transaction record's own height may back a ChainLock proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::wallet::asset_lock) enum RecordHeight {
    /// The height is good to build a proof at.
    Usable(u32),
    /// The SPV headers hold a different block at that height than the record
    /// names, so the record must not be used.
    Contradicted,
    /// Nothing was established this round: the wallet's ChainLock does not
    /// cover the height yet, or the header could not be read in time.
    Pending,
    /// The record carries no height at all.
    None,
}

/// Once-per-wait latches for the record-height header check, so a wait that
/// re-checks every few seconds logs each condition once.
#[derive(Debug, Default)]
pub(in crate::wallet::asset_lock) struct RecordHeaderLog {
    logged_mismatch: bool,
    logged_missing: bool,
    logged_unread: bool,
    logged_network_mismatch: bool,
}

/// Mined-height lookup state for ONE ChainLock-proof wait: the throttle, the
/// latest answer, and which conditions have already been logged.
#[derive(Debug, Default)]
struct LocateState {
    last_lookup: Option<tokio::time::Instant>,
    last: Option<Located>,
    logged_placed: bool,
    logged_header_missing: bool,
    logged_header_mismatch: bool,
    logged_not_included: bool,
    logged_block_unverifiable: bool,
    logged_not_found: bool,
    logged_not_mined: bool,
    logged_chain_lock_short: bool,
    logged_network_mismatch: bool,
    logged_placement_invalidated: bool,
    logged_header_read_stalled: bool,
    logged_header_unreadable: bool,
    /// The `Unavailable` reason last logged, so a repeat of the same failure
    /// stays quiet and a new one is logged.
    logged_unavailable_reason: Option<String>,
}

impl LocateState {
    /// A lookup is due unless the transaction is already placed — after that
    /// only the wallet's ChainLock can be short, which needs no DAPI call — or
    /// the previous lookup was too recent.
    fn lookup_due(&self) -> bool {
        !matches!(self.last, Some(Located::Mined { .. }))
            && self
                .last_lookup
                .is_none_or(|at| at.elapsed() >= LOCATE_MIN_INTERVAL)
    }

    /// Keep `located` as the latest answer, stamp the throttle, and log it the
    /// first time this wait sees that kind of answer.
    fn record(&mut self, out_point: &OutPoint, located: Located) {
        match &located {
            Located::Mined { height, .. } if !self.logged_placed => {
                self.logged_placed = true;
                tracing::info!(
                    outpoint = %out_point,
                    height,
                    "ChainLock wait: funding tx inclusion verified against the SPV header at height {height}"
                );
            }
            Located::HeaderMissing { height } if !self.logged_header_missing => {
                self.logged_header_missing = true;
                tracing::warn!(
                    outpoint = %out_point,
                    height,
                    "ChainLock wait: DAPI places the funding tx at a height the SPV header store \
                     does not hold; not using it"
                );
            }
            Located::HeaderMismatch { height } if !self.logged_header_mismatch => {
                self.logged_header_mismatch = true;
                tracing::warn!(
                    outpoint = %out_point,
                    height,
                    "ChainLock wait: DAPI's block for the funding tx differs from the SPV header \
                     at that height; not using it"
                );
            }
            Located::NotIncluded { height } if !self.logged_not_included => {
                self.logged_not_included = true;
                tracing::warn!(
                    outpoint = %out_point,
                    height,
                    "ChainLock wait: the SPV-verified block at that height does not contain the \
                     funding tx; not using DAPI's placement"
                );
            }
            Located::BlockUnverifiable { height, reason } if !self.logged_block_unverifiable => {
                self.logged_block_unverifiable = true;
                tracing::warn!(
                    outpoint = %out_point,
                    height,
                    reason = %reason,
                    "ChainLock wait: could not verify the funding tx's inclusion in the \
                     SPV-verified block"
                );
            }
            Located::NotFound if !self.logged_not_found => {
                self.logged_not_found = true;
                tracing::warn!(
                    outpoint = %out_point,
                    "ChainLock wait: DAPI does not know the funding tx"
                );
            }
            Located::NotMined if !self.logged_not_mined => {
                self.logged_not_mined = true;
                tracing::info!(
                    outpoint = %out_point,
                    "ChainLock wait: the funding tx is not mined yet"
                );
            }
            Located::Unavailable(reason)
                if self.logged_unavailable_reason.as_deref() != Some(reason.as_str()) =>
            {
                self.logged_unavailable_reason = Some(reason.clone());
                tracing::warn!(
                    outpoint = %out_point,
                    reason = %reason,
                    "ChainLock wait: mined-height lookup unavailable; will retry on the next \
                     lock event or tick"
                );
            }
            _ => {}
        }
        self.last_lookup = Some(tokio::time::Instant::now());
        self.last = Some(located);
    }

    /// Drop a cached placement whose block the SPV header chain no longer
    /// holds at its height (a reorg, or a header store that lost it), so the
    /// next due lookup starts over. `last_lookup` is kept: the throttle still
    /// applies.
    fn invalidate_placement(
        &mut self,
        out_point: &OutPoint,
        height: u32,
        verified: BlockHash,
        now: Option<BlockHash>,
    ) {
        self.last = None;
        self.logged_placed = false;
        if !self.logged_placement_invalidated {
            self.logged_placement_invalidated = true;
            tracing::warn!(
                outpoint = %out_point,
                "ChainLock wait: the SPV header at height {height} no longer matches the block \
                 the funding tx was verified in (verified={verified}, now={now:?}); discarding \
                 the placement and looking it up again"
            );
        }
    }
}

/// Fall back to the persister if the in-memory `transactions()` map
/// didn't have the record.
///
/// With upstream's `keep-finalized-transactions` Cargo feature OFF
/// (the default), chainlocked records are evicted from the in-memory
/// map and only their txids retained in `finalized_txids`. The
/// persister received the full record on the chainlock-transition
/// `store` call before eviction, so it can answer the lookup. A
/// persister that doesn't index records by txid (the trait's default
/// impl,
/// [`NoPlatformPersistence`](crate::wallet::persister::NoPlatformPersistence))
/// returns `Ok(None)` here — callers must still handle the absence.
///
/// Persister errors are surfaced as `Err(PersistenceError)` so call
/// sites can choose their own policy:
///
/// - **Poll loops** (`wait_for_chain_lock`, `wait_for_proof`) read every
///   failure as a miss and keep waiting on the live sync stream — see
///   [`record_or_persister_for_poll`].
/// - **One-shot recovery / fast-fail call sites** want the error
///   visible so a transient backend failure isn't silently classified
///   as "tx not found" — they handle the `Err` arm explicitly.
pub(super) fn record_or_persister(
    in_memory: Option<TransactionRecord>,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &Txid,
) -> Result<Option<TransactionRecord>, crate::changeset::PersistenceError> {
    if let Some(record) = in_memory {
        return Ok(Some(record));
    }
    persister.get_core_tx_record(txid)
}

/// Family-aware in-memory funding-tx record lookup, shared by EVERY proof,
/// ChainLock-wait, and recovery path.
///
/// `TrackedAssetLock.account_index` is family-less — it records the source
/// index, not which accounts ended up funding the lock — while key-wallet files
/// a transaction under *every* account its inputs touch. So the record can sit
/// in any of the families a lock may be funded from, and looking in only some
/// of them leaves it invisible (fatal on hosts running `NoPlatformPersistence`,
/// whose persister fallback always returns `None`, and a burnt proof-wait
/// timeout everywhere else).
///
/// Two shapes make that a live concern: a whole-balance CoinJoin drain files
/// only under `coinjoin_accounts`, and a POOLED asset lock
/// (`ASSET_LOCK_FUNDING_SOURCES`) may take nothing from BIP44 and be funded
/// entirely out of the BIP32 account or a DashPay contact-receiving one. All
/// four families are therefore checked: the standard pair and CoinJoin at
/// `account_index`, then the DashPay receiving accounts, which span their own
/// indices and so are searched by txid alone. BIP44 stays first — it holds
/// every historical lock.
pub(in crate::wallet::asset_lock) fn funding_tx_record(
    accounts: &key_wallet::account::ManagedAccountCollection,
    account_index: u32,
    txid: &Txid,
) -> Option<TransactionRecord> {
    funding_accounts(accounts, account_index)
        .find_map(|account| account.transactions().get(txid).cloned())
}

/// The account families a lock funded from `account_index` can have filed
/// its funding transaction under, in the order [`funding_tx_record`]
/// documents.
fn funding_accounts(
    accounts: &key_wallet::account::ManagedAccountCollection,
    account_index: u32,
) -> impl Iterator<Item = &key_wallet::managed_account::ManagedCoreFundsAccount> {
    let at_index = [
        accounts.standard_bip44_accounts.get(&account_index),
        accounts.standard_bip32_accounts.get(&account_index),
        accounts.coinjoin_accounts.get(&account_index),
    ];
    at_index
        .into_iter()
        .flatten()
        .chain(accounts.dashpay_receival_accounts.values())
}

/// Whether any account family that could hold the funding transaction
/// reports `txid` as chainlock-finalized.
///
/// This is the same finality question [`record_holds_local_finality`] asks,
/// for the record that is no longer there to ask it of. Under the default
/// `keep-finalized-transactions` configuration a chainlock promotion drops
/// the promoted record and keeps only its txid in the account's finalized
/// set, so from that moment on a lookup by record cannot see a finality the
/// wallet has already recorded — the txid set is the only place it survives.
/// Searched over the same families, in the same order, as
/// [`funding_tx_record`].
pub(in crate::wallet::asset_lock) fn funding_tx_is_finalized(
    accounts: &key_wallet::account::ManagedAccountCollection,
    account_index: u32,
    txid: &Txid,
) -> bool {
    funding_accounts(accounts, account_index).any(|account| account.transaction_is_finalized(txid))
}

/// Whether `record` on its own already establishes local finality for a
/// funding transaction — the three record shapes [`AssetLockManager::wait_for_proof`]
/// turns into a proof, reduced to a yes/no.
///
/// It exists so a caller holding a wallet read guard can ask the finality
/// question inside its own snapshot instead of taking a second read. The
/// answer must be read together with the rest of a decision that depends on
/// it; splitting the two reads lets finality land in between and be missed.
///
/// `wallet_chain_lock_height` and `networks_match` come from the same
/// snapshot as `record`. They serve only the third shape — a record whose
/// own context is not yet promoted but whose block the wallet's applied
/// chainlock already buries — and carry the same chain-id refusal as the
/// proof builder: a `last_applied_chain_lock` persisted from a different
/// network says nothing about this record's block.
pub(in crate::wallet::asset_lock) fn record_holds_local_finality(
    record: &TransactionRecord,
    wallet_chain_lock_height: Option<dashcore::prelude::CoreBlockHeight>,
    networks_match: bool,
) -> bool {
    use key_wallet::transaction_checking::TransactionContext;
    match &record.context {
        TransactionContext::InstantSend(_) => true,
        TransactionContext::InChainLockedBlock(_) => record.height().is_some(),
        _ => {
            networks_match
                && matches!(
                    (wallet_chain_lock_height, record.height()),
                    (Some(chain_lock), Some(height)) if chain_lock >= height
                )
        }
    }
}

/// Variant of [`record_or_persister`] for poll loops: never aborts the wait,
/// whatever the persister does.
///
/// This read is a FALLBACK for records the in-memory map evicted — the live
/// SPV stream can still deliver one — so any failure reads as a miss and the
/// loop keeps waiting, bounded by its own finality timeout.
///
/// Both failure classes report once per wait, via `state`: per-iteration
/// logging would let a broken backend flood the log from inside an unbounded
/// poll loop, saying the same thing every time.
pub(super) fn record_or_persister_for_poll(
    in_memory: Option<TransactionRecord>,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &Txid,
    state: &mut PollReadState,
) -> Option<TransactionRecord> {
    if let Some(record) = in_memory {
        return Some(record);
    }
    match persister.get_core_tx_record_or_transient_miss(txid, &mut state.transient_misses) {
        Ok(found) => found,
        Err(e) => {
            if !state.permanent_reported {
                state.permanent_reported = true;
                tracing::error!(
                    txid = %txid,
                    error = %e,
                    "Core tx-record fallback read is permanently failing; waiting on the \
                     live sync stream instead until this wait's timeout"
                );
            }
            None
        }
    }
}

/// Read diagnostics for ONE wait, owned by the polling loop.
///
/// The permanent-failure latch fires on the first `Err`; the transient tally
/// summarises itself when the wait ends, whichever way it ends.
#[derive(Debug, Default)]
pub(super) struct PollReadState {
    permanent_reported: bool,
    transient_misses: crate::wallet::persister::TransientMissTally,
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Validate an IS-lock proof and upgrade it to a ChainLock proof if the
    /// transaction is old enough that the IS-lock may have expired.
    ///
    /// When the asset lock transaction has been chain-locked and has enough
    /// confirmations (> 8), the InstantSend lock quorum may have rotated,
    /// causing Platform to reject the IS proof. In that case, if the
    /// transaction's block height is within Platform's verified range
    /// (`core_chain_locked_height`), we can safely switch to a ChainLock
    /// proof.
    ///
    /// If the proof is already a ChainLock proof, or the IS proof is still
    /// fresh, it is returned unchanged.
    pub(crate) async fn validate_or_upgrade_proof(
        &self,
        proof: dpp::prelude::AssetLockProof,
        account_index: u32,
        out_point: &OutPoint,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        if !matches!(&proof, dpp::prelude::AssetLockProof::Instant(_)) {
            return Ok(proof);
        }

        let in_memory = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            funding_tx_record(&info.core_wallet.accounts, account_index, &out_point.txid)
            // wm dropped at end of block — release before persister + DAPI calls.
        };

        let record = record_or_persister(in_memory, &self.persister, &out_point.txid)
            .map_err(|e| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Persister lookup for tx {} failed: {}",
                    out_point.txid, e
                ))
            })?
            .ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Transaction {} not found in account {} (in-memory or persister)",
                    out_point.txid, account_index
                ))
            })?;

        // Local SPV-verified ChainLock is the only signal we trust.
        // Skipping a Platform-tip pre-flight: it's an unproven self-
        // report and a malicious DAPI could stall us forever; the
        // submission layer handles the CL-height race by retrying with
        // a bumped `user_fee_increase` if Platform's tip lags ours.
        if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
            if let Some(height) = record.height() {
                tracing::debug!(
                    "Upgrading IS-lock proof to ChainLock proof for tx {} (height={})",
                    out_point.txid,
                    height,
                );
                return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                    core_chain_locked_height: height,
                    out_point: *out_point,
                }));
            }
        }

        Ok(proof)
    }

    /// Upgrade an IS-lock proof to a ChainLock proof after a Platform
    /// rejection.
    ///
    /// Called from the recovery layer when `put_to_platform` fails with
    /// `InvalidInstantAssetLockProofSignature`. If the TX is already
    /// chain-locked, constructs the proof immediately. Otherwise, **waits**
    /// for a ChainLock via SPV events so the caller doesn't see a failure —
    /// just a longer wait.
    ///
    /// The wait does not depend on the record being promoted. A record can be
    /// left with no height at all — an `InstantSend` context carries the lock
    /// and no `BlockInfo` — and ChainLock promotion only advances `InBlock`
    /// records, so such a record would never satisfy a promotion wait. For a
    /// record without a height the wait asks the manager's mined-height
    /// locator where the transaction was mined, and builds the proof at that
    /// height once the SPV header chain holds the same block and the wallet's
    /// own ChainLock covers it.
    ///
    /// `timeout` is `Option<Duration>`: `None` waits **indefinitely**. A
    /// ChainLock is deterministic finality that will eventually cover any
    /// broadcast asset-lock tx, so the user-facing funding flows
    /// (identity registration / top-up, platform-address top-up, shielded
    /// funding) pass `None` — a broadcast lock is pending, never failed.
    /// The only bounded caller is the shielded seed pool, where a
    /// `FinalityTimeout` is a deliberate pacing signal for the
    /// unconfirmed-ancestor stall (see `CL_FALLBACK_TIMEOUT`).
    pub(crate) async fn upgrade_to_chain_lock_proof(
        &self,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        let txid = out_point.txid;

        let account_index = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            info.tracked_asset_locks
                .get(out_point)
                .map(|lock| lock.account_index)
                .ok_or_else(|| {
                    PlatformWalletError::AssetLockProofWait(format!(
                        "Asset lock {} is not tracked",
                        out_point
                    ))
                })?
        };

        // Check if already chain-locked. Falls back to the persister if
        // the in-memory map already evicted the record (default config).
        let in_memory = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            funding_tx_record(&info.core_wallet.accounts, account_index, &txid)
        };

        let record = record_or_persister(in_memory, &self.persister, &txid).map_err(|e| {
            PlatformWalletError::AssetLockProofWait(format!(
                "Persister lookup for tx {} failed: {}",
                txid, e
            ))
        })?;
        let record = record.ok_or_else(|| {
            // Both lookups missed. The asset lock is known-tracked
            // (we validated `tracked_asset_locks.get` above), so this
            // is a wallet-state mismatch / post-wipe race rather than
            // a "not chain-locked yet" case. Fast-fail rather than
            // dispatching to `wait_for_chain_lock` and burning the
            // full timeout.
            PlatformWalletError::AssetLockProofWait(format!(
                "Transaction {} not found in account {} (in-memory or persister)",
                txid, account_index
            ))
        })?;
        let height = if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
            record.height()
        } else {
            None
        };

        let height = match height {
            Some(h) => h,
            None => {
                // Not chain-locked yet — wait for a ChainLock via SPV events.
                tracing::info!(
                    "Transaction {} not yet chain-locked, waiting for ChainLock...",
                    txid
                );
                self.wait_for_chain_lock(account_index, out_point, timeout)
                    .await?
            }
        };

        // Build the proof at the wallet's SPV-verified ChainLock height.
        // We DON'T consult Platform's self-reported `core_chain_locked_height`
        // here — that metadata is unproven and a malicious DAPI node could
        // stall us indefinitely. If Platform's CL tip is briefly behind
        // ours at submission time (race window up to
        // `create-empty-blocks-interval`, ~3m on mainnet), the caller's
        // submission layer (`registration.rs`) detects the resulting
        // `InvalidAssetLockProofCoreChainHeightError` (code 10506) and
        // retries with a fresh ST (bumped `user_fee_increase`) to bypass
        // Tenderdash's invalid-tx cache (`keep-invalid-txs-in-cache = true`
        // on mainnet/testnet).
        tracing::info!(
            "Building ChainLock proof for tx {} (height={}, SPV-verified locally)",
            txid,
            height,
        );

        Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: height,
            out_point: *out_point,
        }))
    }

    /// Wait for a ChainLock that covers the given transaction.
    ///
    /// Resolves with the height to build the proof at, from whichever of these
    /// holds first:
    ///
    /// 1. the record is `InChainLockedBlock`;
    /// 2. the record is `InBlock` at a height the wallet's applied ChainLock
    ///    already covers (the promotion event passed before the record was
    ///    back in memory) and the SPV header at that height is still the block
    ///    the record names;
    /// 3. the record has no height, and the mined-height locator places the
    ///    transaction in a block the SPV header chain holds, at a height the
    ///    wallet's ChainLock covers.
    ///
    /// Re-checks on every InstantSend / ChainLock event and every
    /// [`LOCATE_RETRY_INTERVAL`]; lookups are throttled by
    /// [`LOCATE_MIN_INTERVAL`]. `timeout` is `Option<Duration>`: `None` waits
    /// **indefinitely** (a ChainLock is guaranteed finality that will
    /// eventually arrive, so a broadcast lock is pending, not failed).
    async fn wait_for_chain_lock(
        &self,
        account_index: u32,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<u32, PlatformWalletError> {
        let deadline = timeout.map(|t| tokio::time::Instant::now() + t);
        // Once-per-wait read diagnostics; see `record_or_persister_for_poll`.
        let mut read_state = PollReadState::default();
        let mut locate_state = LocateState::default();
        let mut record_header_log = RecordHeaderLog::default();

        loop {
            // Arm the `Notify` future BEFORE the state check, closing
            // the missed-wakeup race: `notify_waiters()` only wakes
            // already-registered waiters and does NOT store a permit, so a
            // CL/IS event arriving in the gap between "no proof yet"
            // and the `.await` below would be discarded and we'd
            // sleep until `FinalityTimeout`. Calling `enable()` on
            // the pinned `Notified` future registers this waiter
            // first; any subsequent `notify_waiters()` is captured
            // and the `await` either completes immediately or, if
            // the event fires after, wakes us up normally.
            let notified = self.lock_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            // Check — might have been updated by SPV sync. Falls back
            // to the persister so a chainlock that arrived between
            // polls (and was evicted from the in-memory map) is still
            // observed.
            let in_memory = {
                let wm = self.wallet_manager.read().await;
                wm.get_wallet_info(&self.wallet_id).and_then(|info| {
                    funding_tx_record(&info.core_wallet.accounts, account_index, &out_point.txid)
                })
            };
            let record = record_or_persister_for_poll(
                in_memory,
                &self.persister,
                &out_point.txid,
                &mut read_state,
            );
            // Any record path goes through the one eligibility check, so a
            // record the SPV headers contradict is refused here exactly as it
            // is in `wait_for_proof`.
            let mut record_height_usable = false;
            if let Some(record) = record.as_ref() {
                match self
                    .record_chain_proof_height(record, deadline, &mut record_header_log)
                    .await
                {
                    RecordHeight::Usable(h) => {
                        tracing::info!(
                            "ChainLock wait for tx {} resolved from the record's own height \
                             ({}), checked against the SPV header chain",
                            out_point.txid,
                            h,
                        );
                        return Ok(h);
                    }
                    // The record has a height that cannot be used; the lookup
                    // is the only thing that can still resolve this wait.
                    RecordHeight::Contradicted | RecordHeight::None => {
                        record_height_usable = false;
                    }
                    // The record may yet become usable — nothing for the
                    // lookup to add this round.
                    RecordHeight::Pending => record_height_usable = true,
                }
            }

            if !record_height_usable {
                if let Some(h) = self
                    .located_chain_proof_height(out_point, &mut locate_state, deadline)
                    .await
                {
                    tracing::info!(
                        "ChainLock proof height {} for tx {} taken from a mined-height lookup \
                         verified against the SPV header chain (record ctx={:?})",
                        h,
                        out_point.txid,
                        record.as_ref().map(|r| &r.context),
                    );
                    return Ok(h);
                }
            }

            // Wait for a lock event, the retry tick, or the deadline when
            // one is configured. The `notified` future is the one we armed
            // above, so any CL/IS event since then is already buffered into it.
            let retry_tick = tokio::time::sleep(LOCATE_RETRY_INTERVAL);
            match deadline {
                Some(dl) => {
                    let remaining = dl.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(PlatformWalletError::FinalityTimeout(*out_point));
                    }
                    tokio::select! {
                        _ = &mut notified => continue,
                        _ = retry_tick => continue,
                        _ = tokio::time::sleep(remaining) => {
                            return Err(PlatformWalletError::FinalityTimeout(*out_point));
                        }
                    }
                }
                // No deadline: wait indefinitely, re-checking on each wake.
                None => {
                    tokio::select! {
                        _ = &mut notified => {}
                        _ = retry_tick => {}
                    }
                }
            }
        }
    }

    /// Whether `record`'s own height may back a ChainLock proof right now.
    ///
    /// One place decides this for every caller — the ChainLock wait, the proof
    /// wait, and the zero-timeout probes that run through them — so a record
    /// the SPV headers contradict cannot be used by one path after another
    /// refused it.
    ///
    /// An `InChainLockedBlock` context is accepted outright: its promotion was
    /// driven by a BLS-verified ChainLock. Any other context with a height has
    /// to clear three checks — the wallet's ChainLock covers it, the wallet's
    /// network matches the SDK's, and the SPV header at that height is still
    /// the block the record names. A store that holds no header there, or no
    /// store at all, is accepted with a once-per-wait note: that is the
    /// long-standing behaviour hosts without a header store rely on.
    pub(in crate::wallet::asset_lock) async fn record_chain_proof_height(
        &self,
        record: &TransactionRecord,
        deadline: Option<tokio::time::Instant>,
        log: &mut RecordHeaderLog,
    ) -> RecordHeight {
        use key_wallet::transaction_checking::TransactionContext;

        let Some(height) = record.height() else {
            return RecordHeight::None;
        };
        if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
            return RecordHeight::Usable(height);
        }

        let (wallet_cl_height, networks_match) = self.wallet_chain_lock_state().await;
        if !wallet_cl_height.is_some_and(|cl| cl >= height) {
            return RecordHeight::Pending;
        }
        if !networks_match {
            if !log.logged_network_mismatch {
                log.logged_network_mismatch = true;
                tracing::error!(
                    sdk_network = ?self.sdk.network,
                    height,
                    "REFUSING to build a ChainLock proof from the record's height — the \
                     wallet's declared network does not match the SDK's. Persisted state \
                     was likely loaded into the wrong network."
                );
            }
            return RecordHeight::Pending;
        }

        // A height always comes from a `BlockInfo`, so this is the block the
        // record names.
        let Some(recorded) = record.context.block_info().map(|info| info.block_hash()) else {
            return RecordHeight::Usable(height);
        };
        match self.header_hash_within(height, deadline).await {
            HeaderRead::Answer(HeaderLookup::Found(spv)) if spv == recorded => {
                RecordHeight::Usable(height)
            }
            HeaderRead::Answer(HeaderLookup::Found(_)) => {
                if !log.logged_mismatch {
                    log.logged_mismatch = true;
                    tracing::warn!(
                        height,
                        "the SPV header at the record's height is a different block than the \
                         record names; not building a proof from the record"
                    );
                }
                RecordHeight::Contradicted
            }
            // Nothing to contradict the record — the compatibility path for
            // hosts that keep no SPV headers.
            HeaderRead::Answer(HeaderLookup::Absent)
            | HeaderRead::Answer(HeaderLookup::NoSource) => {
                if !log.logged_missing {
                    log.logged_missing = true;
                    tracing::info!(
                        height,
                        "no SPV header stored at the record's height; accepting the record's \
                         own block"
                    );
                }
                RecordHeight::Usable(height)
            }
            HeaderRead::Answer(HeaderLookup::Unreadable(reason)) => {
                if !log.logged_unread {
                    log.logged_unread = true;
                    tracing::warn!(
                        height,
                        reason = %reason,
                        "SPV header store could not be read at the record's height; not \
                         building a proof from the record this round"
                    );
                }
                RecordHeight::Pending
            }
            HeaderRead::Stalled(budget) => {
                if !log.logged_unread {
                    log.logged_unread = true;
                    tracing::warn!(
                        height,
                        "SPV header read at the record's height did not complete within \
                         {budget:?}; not building a proof from the record this round"
                    );
                }
                RecordHeight::Pending
            }
        }
    }

    /// Read the SPV header hash at `height`, bounded by [`HEADER_READ_TIMEOUT`]
    /// and by whatever is left of `deadline`.
    ///
    /// The wait is blind while this runs — it sees neither lock events nor its
    /// own deadline — so the read gets a budget rather than however long the
    /// header store takes.
    async fn header_hash_within(
        &self,
        height: u32,
        deadline: Option<tokio::time::Instant>,
    ) -> HeaderRead {
        let budget = deadline.map_or(HEADER_READ_TIMEOUT, |dl| {
            HEADER_READ_TIMEOUT.min(dl.saturating_duration_since(tokio::time::Instant::now()))
        });
        if budget.is_zero() {
            return HeaderRead::Stalled(budget);
        }
        match tokio::time::timeout(budget, self.mined_height_locator.header_hash_at(height)).await {
            Ok(answer) => HeaderRead::Answer(answer),
            Err(_) => HeaderRead::Stalled(budget),
        }
    }

    /// The wallet's applied ChainLock height, and whether the wallet's
    /// declared network matches the SDK's.
    ///
    /// A persisted ChainLock from another network (config drift, a restore
    /// gone wrong) must not be used to build a proof: Platform would reject
    /// it with 10506 and the submission layer would burn its retry budget.
    async fn wallet_chain_lock_state(&self) -> (Option<u32>, bool) {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id);
        let wallet_cl_height = info
            .and_then(|i| i.core_wallet.metadata.last_applied_chain_lock.as_ref())
            .map(|cl| cl.block_height);
        let networks_match = info.is_some_and(|i| i.network() == self.sdk.network);
        (wallet_cl_height, networks_match)
    }

    /// The proof height a mined-height lookup supports right now, if any.
    ///
    /// Calls the locator only when [`LocateState::lookup_due`] allows it and
    /// otherwise re-evaluates the previous answer against the wallet's
    /// current ChainLock.
    ///
    /// Each lookup is capped at [`LOCATE_ATTEMPT_TIMEOUT`] and at whatever is
    /// left of `deadline`; with no time left it is skipped. A lookup that runs
    /// out of time is recorded as `Unavailable`, which stamps the throttle like
    /// any other answer, so the next attempt waits [`LOCATE_MIN_INTERVAL`].
    ///
    /// A verified placement is cached, but never trusted blindly: every time
    /// it would yield a height, the SPV header at that height is read again
    /// and must still be the block the inclusion was verified in. A mismatch
    /// or a missing header discards the placement. A reorg can still land
    /// between that read and the caller submitting the proof; Platform checks
    /// the transaction's height against its own Core view, and a rejection
    /// sends the row off that proof (see `invalidate_rejected_chain_proof`).
    async fn located_chain_proof_height(
        &self,
        out_point: &OutPoint,
        state: &mut LocateState,
        deadline: Option<tokio::time::Instant>,
    ) -> Option<u32> {
        if state.lookup_due() {
            let budget = deadline.map_or(LOCATE_ATTEMPT_TIMEOUT, |dl| {
                LOCATE_ATTEMPT_TIMEOUT
                    .min(dl.saturating_duration_since(tokio::time::Instant::now()))
            });
            if !budget.is_zero() {
                let located = match tokio::time::timeout(
                    budget,
                    self.mined_height_locator.locate(&out_point.txid),
                )
                .await
                {
                    Ok(located) => located,
                    Err(_) => Located::Unavailable(format!("lookup timed out after {budget:?}")),
                };
                state.record(out_point, located);
            }
        }
        let located = state.last.clone()?;
        let (wallet_cl_height, networks_match) = self.wallet_chain_lock_state().await;
        let height = chain_proof_height_from_lookup(&located, wallet_cl_height, networks_match);
        if let (
            Some(h),
            Located::Mined {
                block_hash: verified,
                ..
            },
        ) = (height, &located)
        {
            match self.header_hash_within(h, deadline).await {
                HeaderRead::Answer(HeaderLookup::Found(now)) if now == *verified => return Some(h),
                HeaderRead::Answer(HeaderLookup::Found(now)) => {
                    state.invalidate_placement(out_point, h, *verified, Some(now));
                    return None;
                }
                // The chain holds nothing at that height any more: the block
                // the placement was verified in is gone.
                HeaderRead::Answer(HeaderLookup::Absent) => {
                    state.invalidate_placement(out_point, h, *verified, None);
                    return None;
                }
                // Unread is not evidence — keep the placement and try again
                // on the next wake.
                HeaderRead::Answer(HeaderLookup::Unreadable(reason)) => {
                    if !state.logged_header_unreadable {
                        state.logged_header_unreadable = true;
                        tracing::warn!(
                            outpoint = %out_point,
                            reason = %reason,
                            "ChainLock wait: SPV header store could not be read at height {h}; \
                             not using the placement this round"
                        );
                    }
                    return None;
                }
                HeaderRead::Answer(HeaderLookup::NoSource) => {
                    if !state.logged_header_unreadable {
                        state.logged_header_unreadable = true;
                        tracing::warn!(
                            outpoint = %out_point,
                            "ChainLock wait: no SPV header source to re-check the placement \
                             against; not using it"
                        );
                    }
                    return None;
                }
                // Keep the placement: a reader that did not answer says
                // nothing about the header, and the next wake reads again.
                HeaderRead::Stalled(budget) => {
                    if !state.logged_header_read_stalled {
                        state.logged_header_read_stalled = true;
                        tracing::warn!(
                            outpoint = %out_point,
                            "ChainLock wait: SPV header read at height {h} did not complete \
                             within {budget:?}; not using the placement this round"
                        );
                    }
                    return None;
                }
            }
        }
        if let (None, Located::Mined { height: placed, .. }) = (height, &located) {
            if !networks_match && !state.logged_network_mismatch {
                state.logged_network_mismatch = true;
                tracing::error!(
                    sdk_network = ?self.sdk.network,
                    outpoint = %out_point,
                    "ChainLock wait: REFUSING to build a proof from the looked-up height — the \
                     wallet's declared network does not match the SDK's"
                );
            } else if networks_match && !state.logged_chain_lock_short {
                state.logged_chain_lock_short = true;
                tracing::info!(
                    outpoint = %out_point,
                    height = placed,
                    wallet_cl_height = ?wallet_cl_height,
                    "ChainLock wait: waiting for the wallet's ChainLock to reach the funding tx's block"
                );
            }
        }
        height
    }

    /// Wait for an asset lock proof by subscribing to SPV events.
    ///
    /// Wait for an asset lock proof by checking transaction context state.
    ///
    /// Wakes on `lock_notify` (fired by `SpvEventForwarder` on InstantLock /
    /// ChainLock events) and re-checks the transaction record context.
    ///
    /// Returns a properly-constructed `AssetLockProof` on success, or
    /// `FinalityTimeout` if the timeout elapses first.
    ///
    /// `timeout` is `Option<Duration>`: `None` waits **indefinitely** for
    /// either an InstantSend or a ChainLock proof. Bounded callers use the
    /// deadline as an InstantSend-preference window — on expiry they get a
    /// `FinalityTimeout` and fall back to an (unbounded) ChainLock wait via
    /// [`Self::upgrade_to_chain_lock_proof`].
    pub(in crate::wallet::asset_lock) async fn wait_for_proof(
        &self,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
        use key_wallet::transaction_checking::TransactionContext;

        tracing::info!(outpoint = %out_point, ?timeout, "wait_for_proof: entered");
        let deadline = timeout.map(|t| tokio::time::Instant::now() + t);
        let mut iter: u32 = 0;
        let mut record_header_log = RecordHeaderLog::default();
        // Once-per-wait read diagnostics; see `record_or_persister_for_poll`.
        let mut read_state = PollReadState::default();

        // Read account_index and transaction from the tracked lock.
        let (account_index, tracked_tx) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let lock = info.tracked_asset_locks.get(out_point).ok_or_else(|| {
                PlatformWalletError::AssetLockProofWait(format!(
                    "Asset lock {} is not tracked",
                    out_point.txid
                ))
            })?;
            (lock.account_index, lock.transaction.clone())
        };

        loop {
            iter += 1;
            // Arm the `Notify` future BEFORE the state check, closing
            // the missed-wakeup race: `notify_waiters()` only wakes
            // already-registered waiters and does NOT store a permit, so an
            // IS/CL event arriving in the gap between "no proof yet"
            // and the `.await` below would be discarded and we'd
            // sleep until `FinalityTimeout`. Calling `enable()` on
            // the pinned `Notified` future registers this waiter
            // first; any subsequent `notify_waiters()` is captured
            // and the `await` either completes immediately or, if
            // the event fires after, wakes us up normally.
            let notified = self.lock_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            // Snapshot the wallet's global CL state for diagnostics.
            let (wallet_cl_height, in_memory_tx_ctx, in_memory_tx_height) = {
                let wm = self.wallet_manager.read().await;
                let info = wm.get_wallet_info(&self.wallet_id);
                let cl_h = info
                    .as_ref()
                    .and_then(|i| i.core_wallet.metadata.last_applied_chain_lock.as_ref())
                    .map(|cl| cl.block_height);
                let rec = info.as_ref().and_then(|i| {
                    funding_tx_record(&i.core_wallet.accounts, account_index, &out_point.txid)
                });
                let ctx = rec.as_ref().map(|r| format!("{:?}", r.context));
                let h = rec.as_ref().and_then(|r| r.height());
                (cl_h, ctx, h)
            };
            tracing::debug!(
                outpoint = %out_point,
                iter,
                wallet_cl_height = ?wallet_cl_height,
                in_memory_tx_ctx = ?in_memory_tx_ctx,
                in_memory_tx_height = ?in_memory_tx_height,
                "wait_for_proof: iteration"
            );
            // Check the transaction record context for finality. Falls
            // back to the persister so a chainlocked record evicted
            // from the in-memory map is still observed.
            let in_memory = {
                let wm = self.wallet_manager.read().await;
                wm.get_wallet_info(&self.wallet_id).and_then(|info| {
                    funding_tx_record(&info.core_wallet.accounts, account_index, &out_point.txid)
                })
            };
            if let Some(record) = record_or_persister_for_poll(
                in_memory,
                &self.persister,
                &out_point.txid,
                &mut read_state,
            ) {
                match &record.context {
                    TransactionContext::InstantSend(instant_lock) => {
                        return Ok(dpp::prelude::AssetLockProof::Instant(
                            InstantAssetLockProof::new(
                                instant_lock.clone(),
                                tracked_tx,
                                out_point.vout,
                            ),
                        ));
                    }
                    TransactionContext::InChainLockedBlock(_) => {
                        if let Some(height) = record.height() {
                            // SPV-verified ChainLock BLS signature already
                            // promoted this record's context — local
                            // finality is cryptographically established.
                            // We don't pre-flight Platform here; the
                            // submission layer handles the CL-height race
                            // by retrying with a bumped `user_fee_increase`
                            // when Platform's tip is briefly behind ours.
                            return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                                core_chain_locked_height: height,
                                out_point: *out_point,
                            }));
                        }
                    }
                    _ => {
                        // Per-record context isn't `InChainLockedBlock` yet,
                        // but the wallet's global `last_applied_chain_lock`
                        // may already cover this record's block height — e.g.
                        // on app-launch catch-up, when a record is re-injected
                        // at its persisted `InBlock` context after the CL
                        // event that would have promoted it already passed.
                        //
                        // The same eligibility check the ChainLock wait uses
                        // decides that, so a record the SPV headers contradict
                        // cannot produce a proof here either.
                        if let RecordHeight::Usable(height) = self
                            .record_chain_proof_height(&record, deadline, &mut record_header_log)
                            .await
                        {
                            tracing::info!(
                                "Building ChainLock proof for tx {} from the record's height \
                                 ({}), checked against the SPV header chain",
                                out_point.txid,
                                height,
                            );
                            return Ok(dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                                core_chain_locked_height: height,
                                out_point: *out_point,
                            }));
                        }
                    }
                }
            }

            // Wait for a lock event notification (or timeout, when one is
            // configured). The `notified` future is the one we armed above,
            // so any IS/CL event since then is already buffered into it.
            match deadline {
                Some(dl) => {
                    let remaining = dl.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(PlatformWalletError::FinalityTimeout(*out_point));
                    }
                    tokio::select! {
                        _ = &mut notified => continue,
                        _ = tokio::time::sleep(remaining) => {
                            return Err(PlatformWalletError::FinalityTimeout(*out_point));
                        }
                    }
                }
                // No deadline: wait indefinitely for the next lock event.
                None => {
                    notified.as_mut().await;
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use dashcore::blockdata::transaction::Transaction;
    use dashcore::TxIn;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::transaction_record::TransactionDirection;
    use key_wallet::transaction_checking::{TransactionContext, TransactionType};

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use crate::wallet::platform_wallet::WalletId;

    /// CoinJoin-family regression for [`funding_tx_record`]: a record filed
    /// only under `coinjoin_accounts` (how key-wallet records a tx spending
    /// CoinJoin inputs, e.g. a whole-balance drain asset lock) must be
    /// visible — the pre-fix BIP44-only lookups missed it and burned the
    /// full proof-wait timeout under `NoPlatformPersistence`.
    #[test]
    fn funding_tx_record_finds_coinjoin_only_record() {
        use key_wallet::test_utils::TestWalletContext;

        let mut ctx = TestWalletContext::new_random();
        let record = coinjoin_record_with_txid(0x77);
        let txid = record.txid;
        ctx.managed_wallet
            .first_coinjoin_managed_account_mut()
            .expect("default wallet has CoinJoin account 0")
            .transactions_mut()
            .insert(txid, record);

        let found = funding_tx_record(&ctx.managed_wallet.accounts, 0, &txid)
            .expect("CoinJoin-family record must be found by the shared lookup");
        assert_eq!(found.txid, txid);

        // Unknown txid and unknown account index are clean misses.
        assert!(
            funding_tx_record(&ctx.managed_wallet.accounts, 0, &Txid::from([0x01; 32])).is_none()
        );
        assert!(funding_tx_record(&ctx.managed_wallet.accounts, 9, &txid).is_none());
    }

    /// The historical BIP44 path through [`funding_tx_record`] still resolves.
    #[test]
    fn funding_tx_record_finds_bip44_record() {
        use key_wallet::test_utils::TestWalletContext;

        let mut ctx = TestWalletContext::new_random();
        let record = record_with_txid(0x42);
        let txid = record.txid;
        ctx.managed_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("default wallet has BIP44 account 0")
            .transactions_mut()
            .insert(txid, record);

        let found = funding_tx_record(&ctx.managed_wallet.accounts, 0, &txid)
            .expect("BIP44-family record must be found by the shared lookup");
        assert_eq!(found.txid, txid);
    }

    /// BIP32-family regression for [`funding_tx_record`]: a POOLED asset
    /// lock (`ASSET_LOCK_FUNDING_SOURCES`) may take nothing from BIP44 and
    /// be funded entirely out of the BIP32 account, filing the record only
    /// under `standard_bip32_accounts` — the lookup must still see it.
    #[test]
    fn funding_tx_record_finds_bip32_only_record() {
        use key_wallet::test_utils::TestWalletContext;

        let mut ctx = TestWalletContext::new_random();
        let record = bip32_record_with_txid(0x55);
        let txid = record.txid;
        ctx.managed_wallet
            .first_bip32_managed_account_mut()
            .expect("default wallet has BIP32 account 0")
            .transactions_mut()
            .insert(txid, record);

        let found = funding_tx_record(&ctx.managed_wallet.accounts, 0, &txid)
            .expect("BIP32-family record must be found by the shared lookup");
        assert_eq!(found.txid, txid);

        // Unknown txid and unknown account index are clean misses.
        assert!(
            funding_tx_record(&ctx.managed_wallet.accounts, 0, &Txid::from([0x02; 32])).is_none()
        );
        assert!(funding_tx_record(&ctx.managed_wallet.accounts, 9, &txid).is_none());
    }

    /// DashPay-family regression for [`funding_tx_record`]: the receiving
    /// accounts span their own indices, so the lookup searches them by txid
    /// alone. A record filed only under a contact-receiving account whose
    /// OWN index (7) differs from the tracked source `account_index` (0)
    /// must still be found.
    #[test]
    fn funding_tx_record_finds_dashpay_receival_record_across_indices() {
        use key_wallet::account::account_collection::DashpayAccountKey;
        use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, KeySource};
        use key_wallet::managed_account::ManagedCoreFundsAccount;
        use key_wallet::test_utils::TestWalletContext;
        use key_wallet::{DerivationPath, ManagedAccountType, Network};

        let mut ctx = TestWalletContext::new_random();

        let user_identity_id = [0xAB; 32];
        let friend_identity_id = [0xCD; 32];
        let addresses = AddressPool::new(
            DerivationPath::master(),
            AddressPoolType::Absent,
            20,
            Network::Testnet,
            &KeySource::NoKeySource,
        )
        .expect("single DashPay address pool");
        let mut account = ManagedCoreFundsAccount::new(
            ManagedAccountType::DashpayReceivingFunds {
                index: 7,
                user_identity_id,
                friend_identity_id,
                addresses,
            },
            Network::Testnet,
        );

        let record = dashpay_record_with_txid(0x66, 7, user_identity_id, friend_identity_id);
        let txid = record.txid;
        account.transactions_mut().insert(txid, record);
        ctx.managed_wallet
            .accounts
            .dashpay_receival_accounts
            .insert(
                DashpayAccountKey {
                    index: 7,
                    user_identity_id,
                    friend_identity_id,
                },
                account,
            );

        // The tracked source index (0) does not match the account's own
        // index (7), yet the record is found — DashPay receiving accounts
        // are searched by txid, not by the tracked source index.
        let found = funding_tx_record(&ctx.managed_wallet.accounts, 0, &txid)
            .expect("DashPay receival record must be found regardless of account_index");
        assert_eq!(found.txid, txid);

        // Even an account_index matching no account in any family still
        // resolves the DashPay record — the search is index-independent.
        let found_any_index = funding_tx_record(&ctx.managed_wallet.accounts, 9, &txid)
            .expect("DashPay lookup is index-independent");
        assert_eq!(found_any_index.txid, txid);

        // Unknown txid is still a clean miss.
        assert!(
            funding_tx_record(&ctx.managed_wallet.accounts, 0, &Txid::from([0x03; 32])).is_none()
        );
    }

    /// [`record_with_txid`] sibling filed as a BIP32-account record.
    fn bip32_record_with_txid(seed: u8) -> TransactionRecord {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP32Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// [`record_with_txid`] sibling filed as a DashPay contact-receiving
    /// account record.
    fn dashpay_record_with_txid(
        seed: u8,
        index: u32,
        user_identity_id: [u8; 32],
        friend_identity_id: [u8; 32],
    ) -> TransactionRecord {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::DashpayReceivingFunds {
                index,
                user_identity_id,
                friend_identity_id,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// [`record_with_txid`] sibling filed as a CoinJoin-account record.
    fn coinjoin_record_with_txid(seed: u8) -> TransactionRecord {
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::CoinJoin { index: 0 },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    fn record_with_txid(seed: u8) -> TransactionRecord {
        // A unique txid per `seed` falls out of the (different) input
        // outpoint; the actual transaction body doesn't matter for the
        // helper-under-test's purposes.
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(Txid::from([seed; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::Mempool,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// Test persister that answers `get_core_tx_record` from a
    /// configurable in-memory map. `store` / `flush` are no-ops; `load`
    /// returns the default state.
    struct FakeRecordStore {
        records: Mutex<HashMap<Txid, TransactionRecord>>,
    }

    impl FakeRecordStore {
        fn with_records<I: IntoIterator<Item = TransactionRecord>>(records: I) -> Self {
            let map = records.into_iter().map(|r| (r.txid, r)).collect();
            Self {
                records: Mutex::new(map),
            }
        }
    }

    impl PlatformWalletPersistence for FakeRecordStore {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            Ok(self.records.lock().unwrap().get(txid).cloned())
        }
    }

    /// Persister with a permanent `get_core_tx_record` failure.
    struct ErroringStore;

    impl PlatformWalletPersistence for ErroringStore {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            _txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            Err(PersistenceError::backend("simulated backend failure"))
        }
    }

    struct TransientErroringStore;

    impl PlatformWalletPersistence for TransientErroringStore {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            _txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            Err(PersistenceError::backend_with_kind(
                crate::changeset::PersistenceErrorKind::Transient,
                "simulated transient backend failure",
            ))
        }
    }

    fn wallet_persister(inner: Arc<dyn PlatformWalletPersistence>) -> WalletPersister {
        WalletPersister::new([0u8; 32], inner)
    }

    #[test]
    fn record_or_persister_prefers_in_memory_when_present() {
        // The in-memory record wins; the persister's record is never
        // consulted (so even a record stored under a *different* txid
        // would be ignored — this verifies the helper short-circuits on
        // a hit).
        let in_memory = record_with_txid(0xAA);
        let in_memory_txid = in_memory.txid;
        let other = record_with_txid(0xBB);
        let persister = wallet_persister(Arc::new(FakeRecordStore::with_records([other])));

        let resolved = record_or_persister(Some(in_memory.clone()), &persister, &in_memory_txid)
            .expect("in-memory hit cannot fail");
        assert_eq!(resolved.map(|r| r.txid), Some(in_memory_txid));
    }

    #[test]
    fn record_or_persister_falls_back_to_persister_on_miss() {
        // The in-memory map evicted (None); the persister still has it.
        // This is the chain-locked-eviction recovery path.
        let stored = record_with_txid(0xCC);
        let stored_txid = stored.txid;
        let persister = wallet_persister(Arc::new(FakeRecordStore::with_records([stored])));

        let resolved =
            record_or_persister(None, &persister, &stored_txid).expect("persister succeeded");
        assert_eq!(resolved.map(|r| r.txid), Some(stored_txid));
    }

    #[test]
    fn record_or_persister_returns_none_when_neither_has_it() {
        // Both miss → Ok(None); callers handle this as "tx not found
        // locally" (proof flow returns its own AssetLockProofWait
        // error, poll loops continue waiting).
        let unknown_txid = Txid::from([0xDD; 32]);
        let persister = wallet_persister(Arc::new(FakeRecordStore::with_records([])));

        let resolved =
            record_or_persister(None, &persister, &unknown_txid).expect("persister succeeded");
        assert!(resolved.is_none());
    }

    #[test]
    fn record_or_persister_default_persister_returns_none() {
        // The trait default impl on `NoPlatformPersistence` returns
        // `Ok(None)` — confirms backends that don't override the new
        // method gracefully no-op (the proof flow still works for
        // mempool/InBlock txs; only the chainlock-eviction recovery is
        // unavailable).
        let unknown_txid = Txid::from([0xEE; 32]);
        let persister = wallet_persister(Arc::new(NoPlatformPersistence));

        let resolved =
            record_or_persister(None, &persister, &unknown_txid).expect("persister succeeded");
        assert!(resolved.is_none());
    }

    #[test]
    fn record_or_persister_propagates_backend_errors() {
        // Backend errors surface as `Err` so call sites can choose
        // their own policy; poll loops only downgrade transient errors.
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));

        let resolved = record_or_persister(None, &persister, &unknown_txid);
        assert!(resolved.is_err());
    }

    /// A poll loop degrades on a permanent read failure rather than aborting:
    /// the live SPV stream can still end the wait.
    #[test]
    fn poll_read_degrades_to_a_miss_on_permanent_backend_errors() {
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));
        let mut state = PollReadState::default();

        let resolved = record_or_persister_for_poll(None, &persister, &unknown_txid, &mut state);
        assert!(
            resolved.is_none(),
            "a permanent read failure must read as a miss, not abort the wait"
        );
        assert!(
            state.permanent_reported,
            "the first permanent failure must be reported"
        );

        // Subsequent iterations of the SAME wait stay silent.
        let resolved = record_or_persister_for_poll(None, &persister, &unknown_txid, &mut state);
        assert!(resolved.is_none());
        assert!(state.permanent_reported);
    }

    /// The report fires ONCE per wait: a poll loop spins many times against the
    /// same broken backend, and per-iteration reporting buries the log.
    ///
    /// Counting is the point — an assertion that merely finds a report present
    /// passes just as happily when every iteration emits one.
    #[test]
    fn poll_read_reports_a_permanent_failure_once_per_wait_not_once_per_iteration() {
        use crate::test_support::tracing_capture::{RecordedEvents, RecordingGuard};
        use tracing::Level;

        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));
        let mut state = PollReadState::default();

        let recorder = RecordedEvents::default();
        let _guard = RecordingGuard::install(recorder.clone());

        // Three iterations of ONE wait, as a poll loop would.
        for _ in 0..3 {
            assert!(
                record_or_persister_for_poll(None, &persister, &unknown_txid, &mut state).is_none()
            );
        }

        let reports = recorder
            .entries()
            .into_iter()
            .filter(|(level, msg)| {
                *level == Level::ERROR && msg.contains("Core tx-record fallback read")
            })
            .count();
        assert_eq!(
            reports, 1,
            "three iterations of one wait must produce exactly one report, got {reports}"
        );
    }

    /// A transient failure must not consume the once-per-wait report.
    #[test]
    fn poll_read_treats_transient_backend_errors_as_a_silent_miss() {
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(TransientErroringStore));
        let mut state = PollReadState::default();

        let resolved = record_or_persister_for_poll(None, &persister, &unknown_txid, &mut state);
        assert!(resolved.is_none());
        assert!(
            !state.permanent_reported,
            "a transient failure must not consume the permanent-failure report"
        );
    }

    /// The shared helper collapses transient failures, not permanent ones, and
    /// only the collapsed ones are counted for the end-of-pass summary.
    #[test]
    fn transient_miss_read_helper_separates_transient_from_permanent() {
        use crate::wallet::persister::TransientMissTally;

        let unknown_txid = Txid::from([0xFF; 32]);
        let mut tally = TransientMissTally::default();

        let transient = wallet_persister(Arc::new(TransientErroringStore));
        assert!(transient
            .get_core_tx_record_or_transient_miss(&unknown_txid, &mut tally)
            .expect("a transient failure must read as a miss")
            .is_none());
        assert_eq!(tally.misses(), 1, "a collapsed failure must be counted");

        let permanent = wallet_persister(Arc::new(ErroringStore));
        assert!(
            permanent
                .get_core_tx_record_or_transient_miss(&unknown_txid, &mut tally)
                .is_err(),
            "a permanent failure must stay visible to the caller"
        );
        assert_eq!(
            tally.misses(),
            1,
            "a permanent failure is reported on its own, never counted as a miss"
        );
    }
}

/// The ChainLock-proof wait against funding records it cannot read a
/// chain-locked height from, with a scripted mined-height locator.
#[cfg(test)]
mod chain_lock_wait_tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, InstantLock, Network, OutPoint, Transaction, TxIn, Txid};
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use dpp::prelude::AssetLockProof;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::AccountType;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use tokio::sync::{Notify, RwLock};

    use crate::error::PlatformWalletError;
    #[cfg(feature = "shielded")]
    use crate::test_support::AlwaysOkBroadcaster;
    use crate::test_support::{
        funded_wallet_manager, AlwaysRejectedBroadcaster, NoopTestPersister,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    #[cfg(feature = "shielded")]
    use crate::wallet::asset_lock::orchestration::{AssetLockFunding, FundingResolution};
    use crate::wallet::asset_lock::sync::locate::{HeaderLookup, Located, MinedHeightLocator};
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use key_wallet_manager::WalletManager;

    /// A distinct block hash per `byte`.
    fn hash(byte: u8) -> BlockHash {
        BlockHash::from_byte_array([byte; 32])
    }

    /// A verified placement at `height` in the block `hash(7)`.
    fn mined(height: u32) -> Located {
        Located::Mined {
            height,
            block_hash: hash(7),
        }
    }

    /// The SPV headers a script implies: each `Mined` answer's block at its height.
    fn headers_for(script: &[Located]) -> HashMap<u32, HeaderLookup> {
        script
            .iter()
            .filter_map(|located| match located {
                Located::Mined { height, block_hash } => {
                    Some((*height, HeaderLookup::Found(*block_hash)))
                }
                _ => None,
            })
            .collect()
    }

    /// Answers from a script: each call takes the front answer while more
    /// than one is left, then keeps repeating the last. Its SPV header store
    /// holds every scripted `Mined` block until a test changes it.
    struct ScriptedLocator {
        script: Mutex<Vec<Located>>,
        calls: AtomicUsize,
        headers: Mutex<HashMap<u32, HeaderLookup>>,
        stall_headers: AtomicBool,
    }

    impl ScriptedLocator {
        /// A locator that answers `script`; it must not be empty.
        fn new(script: Vec<Located>) -> Arc<Self> {
            assert!(!script.is_empty());
            Arc::new(Self {
                headers: Mutex::new(headers_for(&script)),
                script: Mutex::new(script),
                calls: AtomicUsize::new(0),
                stall_headers: AtomicBool::new(false),
            })
        }

        /// Make header reads hang (`true`) or answer again (`false`). A read
        /// already hanging stays hung; the next one sees the new setting.
        fn stall_headers(&self, stalled: bool) {
            self.stall_headers.store(stalled, Ordering::SeqCst);
        }

        /// Replace the remaining script, adding its `Mined` blocks to the headers.
        fn set(&self, script: Vec<Located>) {
            self.headers.lock().unwrap().extend(headers_for(&script));
            *self.script.lock().unwrap() = script;
        }

        /// Put `hash` at `height` in the SPV header store.
        fn set_header(&self, height: u32, hash: BlockHash) {
            self.headers
                .lock()
                .unwrap()
                .insert(height, HeaderLookup::Found(hash));
        }

        /// Drop the SPV header at `height`: the store holds nothing there.
        fn remove_header(&self, height: u32) {
            self.headers.lock().unwrap().remove(&height);
        }

        /// Make the header at `height` unreadable rather than absent.
        fn set_header_unreadable(&self, height: u32) {
            self.headers.lock().unwrap().insert(
                height,
                HeaderLookup::Unreadable("scripted read failure".to_string()),
            );
        }

        /// How many lookups the wait has made.
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl MinedHeightLocator for ScriptedLocator {
        async fn locate(&self, _txid: &Txid) -> Located {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut script = self.script.lock().unwrap();
            if script.len() > 1 {
                script.remove(0)
            } else {
                script[0].clone()
            }
        }

        async fn header_hash_at(&self, height: u32) -> HeaderLookup {
            if self.stall_headers.load(Ordering::SeqCst) {
                std::future::pending::<()>().await;
            }
            self.headers
                .lock()
                .unwrap()
                .get(&height)
                .cloned()
                .unwrap_or(HeaderLookup::Absent)
        }
    }

    /// Never answers: a lookup stalled on an unresponsive DAPI node.
    struct HangingLocator;

    #[async_trait]
    impl MinedHeightLocator for HangingLocator {
        async fn locate(&self, _txid: &Txid) -> Located {
            std::future::pending().await
        }

        async fn header_hash_at(&self, _height: u32) -> HeaderLookup {
            HeaderLookup::NoSource
        }
    }

    /// Stalls on its first call, then answers `answer`; records when each
    /// call started.
    struct HangThenScript {
        answer: Located,
        call_times: Mutex<Vec<tokio::time::Instant>>,
    }

    impl HangThenScript {
        /// A locator whose second and later calls answer `answer`.
        fn new(answer: Located) -> Arc<Self> {
            Arc::new(Self {
                answer,
                call_times: Mutex::new(Vec::new()),
            })
        }

        /// When each lookup started.
        fn call_times(&self) -> Vec<tokio::time::Instant> {
            self.call_times.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl MinedHeightLocator for HangThenScript {
        async fn locate(&self, _txid: &Txid) -> Located {
            let first = {
                let mut times = self.call_times.lock().unwrap();
                times.push(tokio::time::Instant::now());
                times.len() == 1
            };
            if first {
                std::future::pending::<()>().await;
            }
            self.answer.clone()
        }

        async fn header_hash_at(&self, height: u32) -> HeaderLookup {
            headers_for(std::slice::from_ref(&self.answer))
                .get(&height)
                .cloned()
                .unwrap_or(HeaderLookup::Absent)
        }
    }

    /// The manager under test plus the wallet state the tests reach into.
    struct Ctx {
        manager: Arc<AssetLockManager<AlwaysRejectedBroadcaster>>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        notify: Arc<Notify>,
        out_point: OutPoint,
        // Read only by the shielded-gated revert tests.
        #[cfg_attr(not(feature = "shielded"), allow(dead_code))]
        instant_proof: AssetLockProof,
    }

    /// A funded testnet wallet with one tracked, IS-locked asset lock whose
    /// funding record is in `context`.
    async fn ctx(
        context: TransactionContext,
        sdk_network: Network,
        locator: Option<Arc<dyn MinedHeightLocator>>,
    ) -> Ctx {
        let (wallet_manager, wallet_id, _generation, _signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(sdk_network)
                .build()
                .expect("mock sdk"),
        );
        let notify = Arc::new(Notify::new());
        let mut manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::clone(&notify),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(wallet_id, Arc::new(NoopTestPersister)),
        );
        if let Some(locator) = locator {
            manager = manager.with_mined_height_locator(locator);
        }

        let transaction = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from([0x5a; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let out_point = OutPoint::new(transaction.txid(), 0);
        let instant_proof = AssetLockProof::Instant(InstantAssetLockProof::new(
            InstantLock::default(),
            transaction.clone(),
            0,
        ));
        let record = TransactionRecord::new(
            transaction.clone(),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        );
        {
            let mut wm = wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction,
                    account_index: 0,
                    funding_type: AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    identity_index: 0,
                    amount: 1_000_000,
                    status: AssetLockStatus::InstantSendLocked,
                    proof: Some(instant_proof.clone()),
                },
            );
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded fixture has BIP44 account 0")
                .transactions_mut()
                .insert(out_point.txid, record);
        }

        Ctx {
            manager: Arc::new(manager),
            wallet_manager,
            wallet_id,
            notify,
            out_point,
            instant_proof,
        }
    }

    /// The same fixture with a broadcaster that accepts, for the paths that
    /// re-broadcast before waiting. `Ctx` pins the rejecting broadcaster in its
    /// type, so the accepting one needs its own fixture rather than a flag.
    #[cfg(feature = "shielded")]
    struct OkCtx {
        manager: Arc<AssetLockManager<AlwaysOkBroadcaster>>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        out_point: OutPoint,
        signer: crate::test_support::WalletSigner,
    }

    /// A funded testnet wallet with one tracked lock at `status` (carrying
    /// `proof`), whose funding record is in `context`.
    #[cfg(feature = "shielded")]
    async fn ok_ctx(
        context: TransactionContext,
        status: AssetLockStatus,
        proof: Option<AssetLockProof>,
        locator: Option<Arc<dyn MinedHeightLocator>>,
    ) -> OkCtx {
        let (wallet_manager, wallet_id, _generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let mut manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysOkBroadcaster),
            WalletPersister::new(wallet_id, Arc::new(NoopTestPersister)),
        );
        if let Some(locator) = locator {
            manager = manager.with_mined_height_locator(locator);
        }

        // A real asset-lock transaction: the resume path derives the
        // credit-output key from its special-transaction payload, so a
        // hand-built transaction cannot stand in here.
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        let record = TransactionRecord::new(
            transaction.clone(),
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        );
        {
            let mut wm = wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction,
                    account_index: 0,
                    funding_type: AssetLockFundingType::AssetLockShieldedAddressTopUp,
                    identity_index: 0,
                    amount: 1_000_000,
                    status,
                    proof,
                },
            );
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded fixture has BIP44 account 0")
                .transactions_mut()
                .insert(out_point.txid, record);
        }

        OkCtx {
            manager: Arc::new(manager),
            wallet_manager,
            wallet_id,
            out_point,
            signer,
        }
    }

    /// The tracked row of an [`OkCtx`].
    #[cfg(feature = "shielded")]
    async fn ok_tracked_row(ctx: &OkCtx) -> TrackedAssetLock {
        ctx.wallet_manager
            .read()
            .await
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("row")
            .clone()
    }

    /// Set an [`OkCtx`] wallet's applied ChainLock to `height`.
    #[cfg(feature = "shielded")]
    async fn set_ok_wallet_chain_lock(ctx: &OkCtx, height: u32) {
        let mut wm = ctx.wallet_manager.write().await;
        wm.get_wallet_info_mut(&ctx.wallet_id)
            .expect("wallet must remain registered")
            .core_wallet
            .metadata
            .last_applied_chain_lock = Some(ChainLock {
            block_height: height,
            block_hash: BlockHash::all_zeros(),
            signature: BLSSignature::from([0u8; 96]),
        });
    }

    /// Set the wallet's applied ChainLock to `height`.
    async fn set_wallet_chain_lock(ctx: &Ctx, height: u32) {
        let mut wm = ctx.wallet_manager.write().await;
        wm.get_wallet_info_mut(&ctx.wallet_id)
            .expect("wallet must remain registered")
            .core_wallet
            .metadata
            .last_applied_chain_lock = Some(ChainLock {
            block_height: height,
            block_hash: BlockHash::all_zeros(),
            signature: BLSSignature::from([0u8; 96]),
        });
    }

    /// Move the funding record to `context`.
    async fn set_record_context(ctx: &Ctx, context: TransactionContext) {
        let mut wm = ctx.wallet_manager.write().await;
        wm.get_wallet_info_mut(&ctx.wallet_id)
            .expect("wallet must remain registered")
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("funded fixture has BIP44 account 0")
            .transactions_mut()
            .get_mut(&ctx.out_point.txid)
            .expect("funding record")
            .update_context(context);
    }

    /// The record shape with a lock and no block: `InstantSend`.
    fn instant_send_context() -> TransactionContext {
        TransactionContext::InstantSend(InstantLock::default())
    }

    /// The height of a ChainLock proof; panics on any other proof.
    fn chain_proof_height(proof: AssetLockProof) -> u32 {
        match proof {
            AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height,
                ..
            }) => core_chain_locked_height,
            other => panic!("expected a ChainLock proof, got {other:?}"),
        }
    }

    /// The record shape a promotion wait can never satisfy: without a
    /// locator it still times out, as before.
    #[tokio::test(start_paused = true)]
    async fn instant_send_record_without_a_locator_cannot_resolve() {
        let ctx = ctx(instant_send_context(), Network::Testnet, None).await;
        set_wallet_chain_lock(&ctx, 150).await;

        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(600)))
            .await
            .expect_err("nothing can place the transaction");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(op) if op == ctx.out_point));
    }

    /// A verified placement covered by the wallet's ChainLock resolves at once.
    #[tokio::test(start_paused = true)]
    async fn instant_send_record_builds_chain_proof_from_located_height() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(1),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("resolves without waiting")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(locator.calls(), 1);
    }

    /// A placement the SPV headers contradict is ignored until one matches.
    #[tokio::test(start_paused = true)]
    async fn mismatched_header_is_not_trusted_until_a_later_lookup_matches() {
        let locator =
            ScriptedLocator::new(vec![Located::HeaderMismatch { height: 100 }, mined(100)]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        // No lock event is ever sent: only the retry tick can bring the
        // second lookup.
        let proof = tokio::time::timeout(
            Duration::from_secs(3600),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("the retry tick re-runs the lookup")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(locator.calls(), 2);
    }

    /// A placement above the wallet's ChainLock waits for the ChainLock, not DAPI.
    #[tokio::test(start_paused = true)]
    async fn located_height_above_wallet_chain_lock_waits_then_builds() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 90).await;

        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait =
            tokio::spawn(
                async move { manager.upgrade_to_chain_lock_proof(&out_point, None).await },
            );
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert!(
            !wait.is_finished(),
            "the wallet's ChainLock is below the block"
        );

        set_wallet_chain_lock(&ctx, 120).await;
        ctx.notify.notify_waiters();
        let proof = tokio::time::timeout(Duration::from_secs(3600), wait)
            .await
            .expect("resolves once the ChainLock covers the block")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        // The placement is reused; only the wallet's ChainLock had to move.
        assert_eq!(locator.calls(), 1);
    }

    /// A wallet ChainLock from another network never backs a looked-up proof.
    #[tokio::test(start_paused = true)]
    async fn network_mismatch_refuses_lookup_path() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        let ctx = ctx(
            instant_send_context(),
            Network::Mainnet,
            Some(locator as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(600)))
            .await
            .expect_err("a ChainLock from another network must not back the proof");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
    }

    /// An unmined transaction is the reason the wait has no deadline: it keeps
    /// waiting, re-running the lookup on the retry tick alone, and resolves
    /// once the transaction is placed.
    #[tokio::test(start_paused = true)]
    async fn unmined_tx_keeps_waiting_with_no_timeout() {
        let locator = ScriptedLocator::new(vec![Located::NotMined]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait =
            tokio::spawn(
                async move { manager.upgrade_to_chain_lock_proof(&out_point, None).await },
            );
        tokio::time::sleep(Duration::from_secs(20 * 60)).await;
        assert!(
            !wait.is_finished(),
            "an unmined transaction must not fail the wait"
        );
        let calls = locator.calls();
        assert!(
            (15..=25).contains(&calls),
            "one lookup per retry tick over 20 minutes, got {calls}"
        );

        locator.set(vec![mined(140)]);
        let proof = tokio::time::timeout(Duration::from_secs(5 * 60), wait)
            .await
            .expect("resolves on the next tick")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 140);
    }

    /// A record re-injected at `InBlock` after the promotion event already
    /// passed resolves from the wallet's applied ChainLock.
    #[tokio::test(start_paused = true)]
    async fn chain_lock_wait_accepts_in_block_record_covered_by_wallet_chain_lock() {
        // The SPV header at that height is the block the record names.
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: BlockHash::all_zeros(),
        }]);
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, BlockHash::all_zeros(), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(1),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("resolves without waiting")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(
            locator.calls(),
            0,
            "the record answered; no lookup was needed"
        );
    }

    /// A record that already has a height never costs a DAPI lookup.
    #[tokio::test(start_paused = true)]
    async fn record_with_a_height_never_triggers_a_lookup() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, BlockHash::all_zeros(), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 90).await;

        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(300)))
            .await
            .expect_err("the wallet's ChainLock never reaches the block");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert_eq!(locator.calls(), 0);
    }

    /// The tx-height consensus error is told apart from the IS-signature one.
    #[test]
    fn transaction_height_rejection_is_recognised() {
        use dpp::consensus::basic::identity::InvalidAssetLockProofTransactionHeightError;
        use dpp::consensus::ConsensusError;

        let height_error = dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            ConsensusError::from(InvalidAssetLockProofTransactionHeightError::new(100, None)),
        )));
        assert!(crate::error::is_asset_lock_proof_transaction_height_invalid(&height_error));
        assert!(!crate::error::is_instant_lock_proof_invalid(&height_error));
    }

    /// A stalled lookup cannot push a bounded wait past its deadline.
    #[tokio::test(start_paused = true)]
    async fn stalled_lookup_does_not_extend_a_bounded_wait() {
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::new(HangingLocator)),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let started = tokio::time::Instant::now();
        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(20)))
            .await
            .expect_err("nothing can place the transaction");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert!(
            started.elapsed() <= Duration::from_secs(20),
            "{:?}",
            started.elapsed()
        );
    }

    /// With less than the attempt cap left, the lookup gets only what remains.
    #[tokio::test(start_paused = true)]
    async fn lookup_attempt_is_capped_by_the_remaining_deadline() {
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::new(HangingLocator)),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let started = tokio::time::Instant::now();
        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(5)))
            .await
            .expect_err("nothing can place the transaction");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert_eq!(started.elapsed(), Duration::from_secs(5));
    }

    /// A record promotion that lands during a stalled lookup is seen as soon
    /// as the attempt cap ends the lookup.
    #[tokio::test(start_paused = true)]
    async fn stalled_lookup_cannot_starve_the_record_path() {
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::new(HangingLocator)),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let started = tokio::time::Instant::now();
        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait =
            tokio::spawn(
                async move { manager.upgrade_to_chain_lock_proof(&out_point, None).await },
            );
        tokio::time::sleep(Duration::from_secs(5)).await;

        set_record_context(
            &ctx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(120, BlockHash::all_zeros(), 0)),
        )
        .await;
        ctx.notify.notify_waiters();

        // The guard outlasts the attempt cap by a second: both expire at the
        // same paused instant otherwise. The bound itself is asserted below.
        let proof = tokio::time::timeout(Duration::from_secs(16), wait)
            .await
            .expect("resolves once the stalled lookup is cut off")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 120);
        assert!(
            started.elapsed() <= Duration::from_secs(20),
            "{:?}",
            started.elapsed()
        );
    }

    /// A timed-out lookup counts as `Unavailable`: the next one waits out the
    /// throttle, then its answer resolves the wait.
    #[tokio::test(start_paused = true)]
    async fn timed_out_lookup_is_recorded_as_unavailable_and_throttled() {
        let locator = HangThenScript::new(mined(100));
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let started = tokio::time::Instant::now();
        let proof = tokio::time::timeout(
            Duration::from_secs(3600),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("the second lookup resolves the wait")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);

        let times = locator.call_times();
        assert_eq!(times.len(), 2);
        assert!(
            times[1].duration_since(started) >= Duration::from_secs(20 + 30),
            "second lookup at {:?}",
            times[1].duration_since(started)
        );
    }

    /// A block that lacks the funding tx is no placement: the wait keeps going
    /// until a later lookup verifies inclusion.
    #[tokio::test(start_paused = true)]
    async fn not_included_answer_keeps_waiting_until_a_verified_placement() {
        let locator = ScriptedLocator::new(vec![Located::NotIncluded { height: 100 }, mined(100)]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(3600),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("the verified placement resolves the wait")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(locator.calls(), 2);
    }

    /// A block whose inclusion could not be checked never backs a proof.
    #[tokio::test(start_paused = true)]
    async fn unverifiable_block_is_never_a_proof_height() {
        let locator = ScriptedLocator::new(vec![Located::BlockUnverifiable {
            height: 100,
            reason: "block not served".to_string(),
        }]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(locator as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(90)))
            .await
            .expect_err("an unverified block must not produce a proof");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
    }

    /// A placement cached before a reorg is refused once the SPV header at its
    /// height is a different block, and each fresh lookup that repeats it is
    /// refused the same way.
    #[tokio::test(start_paused = true)]
    async fn stale_placement_is_rejected_after_the_header_at_its_height_changes() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(7),
        }]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 90).await;

        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait = tokio::spawn(async move {
            manager
                .upgrade_to_chain_lock_proof(&out_point, Some(Duration::from_secs(45)))
                .await
        });
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(locator.calls(), 1);

        locator.set_header(100, hash(8));
        set_wallet_chain_lock(&ctx, 120).await;
        ctx.notify.notify_waiters();
        tokio::time::sleep(Duration::from_secs(30)).await;
        // Past the throttle: the next wake looks the placement up again.
        ctx.notify.notify_waiters();

        let err = tokio::time::timeout(Duration::from_secs(60), wait)
            .await
            .expect("the bounded wait ends")
            .expect("task")
            .expect_err("a placement in a block the headers no longer hold is refused");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert!(locator.calls() >= 2, "calls = {}", locator.calls());
    }

    /// After a header change, a lookup that re-verifies against the new block
    /// resolves the wait.
    #[tokio::test(start_paused = true)]
    async fn reverified_placement_after_a_header_change_resolves() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(7),
        }]);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 90).await;

        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait = tokio::spawn(async move {
            manager
                .upgrade_to_chain_lock_proof(&out_point, Some(Duration::from_secs(45)))
                .await
        });
        tokio::time::sleep(Duration::from_secs(1)).await;

        locator.set_header(100, hash(8));
        set_wallet_chain_lock(&ctx, 120).await;
        ctx.notify.notify_waiters();
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(
            !wait.is_finished(),
            "the stale placement must not resolve the wait"
        );

        locator.set(vec![Located::Mined {
            height: 100,
            block_hash: hash(8),
        }]);
        tokio::time::sleep(Duration::from_secs(29)).await;
        ctx.notify.notify_waiters();

        let proof = tokio::time::timeout(Duration::from_secs(60), wait)
            .await
            .expect("resolves on the re-verified placement")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
    }

    /// A header missing at use time invalidates the placement, even on the
    /// first use.
    #[tokio::test(start_paused = true)]
    async fn missing_header_at_use_time_invalidates_the_placement() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        locator.remove_header(100);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(20)))
            .await
            .expect_err("no header, no proof");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert!(locator.calls() >= 1);
    }

    /// An `Unavailable` answer is logged once per distinct reason.
    #[tokio::test]
    async fn unavailable_lookup_logs_once_per_reason() {
        let out_point = OutPoint::new(Txid::from([1; 32]), 0);
        let mut state = super::LocateState::default();

        state.record(&out_point, Located::Unavailable("a".to_string()));
        assert_eq!(state.logged_unavailable_reason.as_deref(), Some("a"));
        state.record(&out_point, Located::Unavailable("a".to_string()));
        assert_eq!(state.logged_unavailable_reason.as_deref(), Some("a"));
        state.record(&out_point, Located::Unavailable("b".to_string()));
        assert_eq!(state.logged_unavailable_reason.as_deref(), Some("b"));
    }

    /// A stalled header read cannot push a bounded wait past its deadline.
    #[tokio::test(start_paused = true)]
    async fn stalled_header_read_does_not_extend_a_bounded_wait() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(7),
        }]);
        locator.stall_headers(true);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let started = tokio::time::Instant::now();
        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(5)))
            .await
            .expect_err("the header was never read");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert_eq!(started.elapsed(), Duration::from_secs(5));
    }

    /// A stalled header read keeps the placement: once the store answers, the
    /// same placement resolves the wait without a second lookup.
    #[tokio::test(start_paused = true)]
    async fn stalled_header_read_keeps_the_placement_and_resolves_when_headers_answer() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(7),
        }]);
        locator.stall_headers(true);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait =
            tokio::spawn(
                async move { manager.upgrade_to_chain_lock_proof(&out_point, None).await },
            );
        tokio::time::sleep(Duration::from_secs(70)).await;
        assert!(
            !wait.is_finished(),
            "a header that never answers resolves nothing"
        );
        locator.stall_headers(false);

        let proof = tokio::time::timeout(Duration::from_secs(600), wait)
            .await
            .expect("the next header read resolves the wait")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(locator.calls(), 1, "the cached placement was reused");
    }

    /// A record promotion that lands during a stalled header read is seen as
    /// soon as the read gives up.
    #[tokio::test(start_paused = true)]
    async fn stalled_header_read_cannot_starve_the_record_path() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(7),
        }]);
        locator.stall_headers(true);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let started = tokio::time::Instant::now();
        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait =
            tokio::spawn(
                async move { manager.upgrade_to_chain_lock_proof(&out_point, None).await },
            );
        tokio::time::sleep(Duration::from_secs(2)).await;

        set_record_context(
            &ctx,
            TransactionContext::InChainLockedBlock(BlockInfo::new(120, BlockHash::all_zeros(), 0)),
        )
        .await;
        ctx.notify.notify_waiters();

        let proof = tokio::time::timeout(Duration::from_secs(10), wait)
            .await
            .expect("resolves once the stalled read is cut off")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 120);
        assert!(
            started.elapsed() <= Duration::from_secs(6),
            "{:?}",
            started.elapsed()
        );
    }

    /// A record whose block the SPV headers contradict is not used; the
    /// lookup decides instead.
    #[tokio::test(start_paused = true)]
    async fn in_block_record_with_a_divergent_spv_header_is_not_used() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(2),
        }]);
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, hash(1), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(1),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("the lookup resolves it")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(
            locator.calls(),
            1,
            "the record was refused, so the lookup ran"
        );
    }

    /// With no header stored at the record's height there is nothing to
    /// contradict it, so the record still resolves the wait.
    #[tokio::test(start_paused = true)]
    async fn in_block_record_with_no_stored_header_still_resolves() {
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, hash(1), 0)),
            Network::Testnet,
            None,
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(1),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("resolves without waiting")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
    }

    /// The tracked row as it stands.
    #[cfg(feature = "shielded")]
    async fn tracked_row(ctx: &Ctx) -> TrackedAssetLock {
        ctx.wallet_manager
            .read()
            .await
            .get_wallet_info(&ctx.wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&ctx.out_point)
            .expect("row")
            .clone()
    }

    /// A ChainLock proof at `height` for the fixture's outpoint.
    #[cfg(feature = "shielded")]
    fn chain_proof_at(ctx: &Ctx, height: u32) -> AssetLockProof {
        AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: height,
            out_point: ctx.out_point,
        })
    }

    /// Put the row on `proof` at `ChainLocked`, as a submit would have left it.
    #[cfg(feature = "shielded")]
    async fn persist_chain_proof(ctx: &Ctx, proof: AssetLockProof) {
        ctx.manager
            .advance_asset_lock_status(&ctx.out_point, AssetLockStatus::ChainLocked, Some(proof))
            .await
            .expect("advance");
    }

    /// Platform's "the tx is not at that height" rejection.
    #[cfg(feature = "shielded")]
    fn height_rejection() -> dash_sdk::Error {
        use dpp::consensus::basic::identity::InvalidAssetLockProofTransactionHeightError;
        use dpp::consensus::ConsensusError;

        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            ConsensusError::from(InvalidAssetLockProofTransactionHeightError::new(100, None)),
        )))
    }

    /// A rejection that says nothing about the proof's height.
    #[cfg(feature = "shielded")]
    fn unrelated_rejection() -> dash_sdk::Error {
        use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
        use dpp::consensus::ConsensusError;

        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(
            ConsensusError::from(InvalidInstantAssetLockProofSignatureError::new()),
        )))
    }

    /// A Chain proof rejected on the first submit — one loaded from the row,
    /// not built by this attempt — falls back to the record's InstantSend proof.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn persisted_chain_proof_rejected_on_first_submit_falls_back_to_the_record_instant_proof()
    {
        let ctx = ctx(instant_send_context(), Network::Testnet, None).await;
        let submitted = chain_proof_at(&ctx, 100);
        persist_chain_proof(&ctx, submitted.clone()).await;

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &submitted, &height_rejection())
            .await;

        let row = tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::InstantSendLocked);
        assert!(matches!(row.proof, Some(AssetLockProof::Instant(_))));
    }

    /// With no local finality to fall back on, the row drops to `Broadcast`
    /// with no proof, which is the arm that rebuilds one.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn persisted_chain_proof_rejected_without_a_local_proof_reverts_to_broadcast() {
        let ctx = ctx(TransactionContext::Mempool, Network::Testnet, None).await;
        let submitted = chain_proof_at(&ctx, 100);
        persist_chain_proof(&ctx, submitted.clone()).await;

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &submitted, &height_rejection())
            .await;

        let row = tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::Broadcast);
        assert!(row.proof.is_none(), "the refused proof must not survive");
    }

    /// When the record supports a different Chain proof, that one replaces the
    /// refused proof and the row stays `ChainLocked`.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn rejected_chain_proof_is_replaced_by_the_records_chain_proof() {
        let ctx = ctx(
            TransactionContext::InChainLockedBlock(BlockInfo::new(140, BlockHash::all_zeros(), 0)),
            Network::Testnet,
            None,
        )
        .await;
        let submitted = chain_proof_at(&ctx, 100);
        persist_chain_proof(&ctx, submitted.clone()).await;

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &submitted, &height_rejection())
            .await;

        let row = tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::ChainLocked);
        assert_eq!(chain_proof_height(row.proof.expect("proof")), 140);
    }

    /// Only a Chain submission rejected for its height is invalidated.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn invalidation_ignores_instant_submissions_and_other_errors() {
        let ctx = ctx(instant_send_context(), Network::Testnet, None).await;
        let submitted = chain_proof_at(&ctx, 100);
        persist_chain_proof(&ctx, submitted.clone()).await;

        ctx.manager
            .invalidate_rejected_chain_proof(
                &ctx.out_point,
                &ctx.instant_proof.clone(),
                &height_rejection(),
            )
            .await;
        assert_eq!(tracked_row(&ctx).await.status, AssetLockStatus::ChainLocked);

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &submitted, &unrelated_rejection())
            .await;
        let row = tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::ChainLocked);
        assert_eq!(chain_proof_height(row.proof.expect("proof")), 100);
    }

    /// A row that has moved off `ChainLocked` is left alone.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn invalidation_leaves_a_row_that_moved_off_chain_locked() {
        let ctx = ctx(instant_send_context(), Network::Testnet, None).await;
        let submitted = chain_proof_at(&ctx, 100);

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &submitted, &height_rejection())
            .await;

        let row = tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::InstantSendLocked);
        assert!(matches!(row.proof, Some(AssetLockProof::Instant(_))));
    }

    /// The predicate: a Chain submission plus a height rejection, nothing else.
    #[cfg(feature = "shielded")]
    #[test]
    fn rejected_chain_proof_needs_invalidation_only_for_a_chain_height_rejection() {
        type Manager = AssetLockManager<AlwaysRejectedBroadcaster>;

        let chain = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::new(Txid::from([0x5a; 32]), 0),
        });
        let instant = AssetLockProof::Instant(InstantAssetLockProof::new(
            InstantLock::default(),
            Transaction {
                version: 1,
                lock_time: 0,
                input: vec![TxIn {
                    previous_output: OutPoint::new(Txid::from([0x5a; 32]), 0),
                    ..Default::default()
                }],
                output: Vec::new(),
                special_transaction_payload: None,
            },
            0,
        ));

        assert!(Manager::rejected_chain_proof_needs_invalidation(
            &chain,
            &height_rejection()
        ));
        assert!(!Manager::rejected_chain_proof_needs_invalidation(
            &instant,
            &height_rejection()
        ));
        assert!(!Manager::rejected_chain_proof_needs_invalidation(
            &chain,
            &unrelated_rejection()
        ));
    }

    /// The IS-timeout fallback persists the proof it rebuilt before deriving
    /// the path, so the resume it runs takes the already-final arm instead of
    /// re-entering the proof wait that just timed out.
    #[cfg(feature = "shielded")]
    #[tokio::test(start_paused = true)]
    async fn is_timeout_fallback_persists_the_rebuilt_chain_proof_before_deriving_the_path() {
        let locator = ScriptedLocator::new(vec![mined(140)]);
        let ctx = ok_ctx(
            TransactionContext::Mempool,
            AssetLockStatus::Broadcast,
            None,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_ok_wallet_chain_lock(&ctx, 150).await;

        // A proof wait would burn minutes; this must not.
        let (proof, _path) = tokio::time::timeout(
            Duration::from_secs(30),
            ctx.manager
                .resolve_chain_proof_after_is_timeout(&ctx.out_point, None),
        )
        .await
        .expect("the persisted proof short-circuits the resume's proof wait")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 140);

        let row = ok_tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::ChainLocked);
        assert_eq!(chain_proof_height(row.proof.expect("proof")), 140);
    }

    /// End to end: a row cleared by the invalidator resolves on the next
    /// resume through the lookup, and the rebuilt proof is persisted.
    #[tokio::test(start_paused = true)]
    #[cfg(feature = "shielded")]
    async fn cleared_row_completes_the_next_resume_through_the_lookup() {
        let locator = ScriptedLocator::new(vec![mined(140)]);
        let rejected = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::new(Txid::from([0x5a; 32]), 0),
        });
        let ctx = ok_ctx(
            TransactionContext::Mempool,
            AssetLockStatus::ChainLocked,
            Some(rejected.clone()),
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_ok_wallet_chain_lock(&ctx, 150).await;

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &rejected, &height_rejection())
            .await;
        let cleared = ok_tracked_row(&ctx).await;
        assert_eq!(cleared.status, AssetLockStatus::Broadcast);
        assert!(cleared.proof.is_none());

        let resolution = tokio::time::timeout(
            Duration::from_secs(900),
            ctx.manager.resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::FromExistingAssetLock {
                    out_point: ctx.out_point,
                    consume_invitation_voucher: false,
                },
                AssetLockFundingType::AssetLockShieldedAddressTopUp,
                0,
                &ctx.signer,
            ),
        )
        .await
        .expect("the record-only wait expires into the IS-timeout outcome")
        .expect("resolution");

        let out_point = match resolution {
            FundingResolution::IsTimeout { out_point } => out_point,
            FundingResolution::Resolved(_) => panic!("the record supports no proof"),
        };
        let (proof, _path) = ctx
            .manager
            .resolve_chain_proof_after_is_timeout(&out_point, None)
            .await
            .expect("the lookup rebuilds the proof");
        assert_eq!(chain_proof_height(proof), 140);

        let row = ok_tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::ChainLocked);
        assert_eq!(chain_proof_height(row.proof.expect("proof")), 140);
    }

    /// `wait_for_proof` refuses a record the SPV headers contradict, exactly
    /// as the ChainLock wait does.
    #[tokio::test(start_paused = true)]
    async fn wait_for_proof_does_not_build_a_chain_proof_from_a_contradicted_record() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        locator.set_header(100, hash(2));
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, hash(1), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let err = ctx
            .manager
            .wait_for_proof(&ctx.out_point, Some(Duration::from_secs(90)))
            .await
            .expect_err("the record's block is not the one the headers hold");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
    }

    /// A record matching the SPV header still produces a proof in
    /// `wait_for_proof` — the check refuses contradiction, not the path.
    #[tokio::test(start_paused = true)]
    async fn in_block_record_matching_the_spv_header_still_yields_a_proof_in_wait_for_proof() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(1),
        }]);
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, hash(1), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(1),
            ctx.manager.wait_for_proof(&ctx.out_point, None),
        )
        .await
        .expect("resolves without waiting")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
    }

    /// The invalidator's zero-timeout probe goes through the same check, so a
    /// contradicted record cannot supply the replacement proof.
    #[cfg(feature = "shielded")]
    #[tokio::test]
    async fn invalidation_does_not_replace_a_rejected_proof_from_a_contradicted_record() {
        let locator = ScriptedLocator::new(vec![mined(140)]);
        locator.set_header(140, hash(2));
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(140, hash(1), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;
        let submitted = chain_proof_at(&ctx, 100);
        persist_chain_proof(&ctx, submitted.clone()).await;

        ctx.manager
            .invalidate_rejected_chain_proof(&ctx.out_point, &submitted, &height_rejection())
            .await;

        let row = tracked_row(&ctx).await;
        assert_eq!(row.status, AssetLockStatus::Broadcast);
        assert!(
            row.proof.is_none(),
            "a contradicted record must not supply a replacement proof"
        );
    }

    /// An unreadable header store is not "no header": the record is not
    /// accepted on the strength of a read that failed.
    #[tokio::test(start_paused = true)]
    async fn unreadable_header_does_not_accept_an_in_block_record() {
        let locator = ScriptedLocator::new(vec![mined(100)]);
        locator.set_header_unreadable(100);
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, hash(1), 0)),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let err = ctx
            .manager
            .upgrade_to_chain_lock_proof(&ctx.out_point, Some(Duration::from_secs(90)))
            .await
            .expect_err("an unread header establishes nothing");
        assert!(matches!(err, PlatformWalletError::FinalityTimeout(_)));
        assert_eq!(locator.calls(), 0, "the record was pending, not refused");
    }

    /// An unreadable header keeps a cached placement; once the store answers,
    /// the same placement resolves the wait.
    #[tokio::test(start_paused = true)]
    async fn unreadable_header_keeps_the_placement_and_resolves_when_readable() {
        let locator = ScriptedLocator::new(vec![Located::Mined {
            height: 100,
            block_hash: hash(7),
        }]);
        locator.set_header_unreadable(100);
        let ctx = ctx(
            instant_send_context(),
            Network::Testnet,
            Some(Arc::clone(&locator) as Arc<dyn MinedHeightLocator>),
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let manager = Arc::clone(&ctx.manager);
        let out_point = ctx.out_point;
        let wait =
            tokio::spawn(
                async move { manager.upgrade_to_chain_lock_proof(&out_point, None).await },
            );
        tokio::time::sleep(Duration::from_secs(70)).await;
        assert!(!wait.is_finished(), "an unread header resolves nothing");

        locator.set_header(100, hash(7));
        let proof = tokio::time::timeout(Duration::from_secs(600), wait)
            .await
            .expect("the readable header resolves the wait")
            .expect("task")
            .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
        assert_eq!(locator.calls(), 1, "the cached placement was reused");
    }

    /// A host with no SPV header source at all still resolves a covered
    /// `InBlock` record — the compatibility path.
    #[tokio::test(start_paused = true)]
    async fn no_header_source_still_accepts_a_covered_in_block_record() {
        let ctx = ctx(
            TransactionContext::InBlock(BlockInfo::new(100, hash(1), 0)),
            Network::Testnet,
            None,
        )
        .await;
        set_wallet_chain_lock(&ctx, 150).await;

        let proof = tokio::time::timeout(
            Duration::from_secs(1),
            ctx.manager
                .upgrade_to_chain_lock_proof(&ctx.out_point, None),
        )
        .await
        .expect("resolves without waiting")
        .expect("proof");
        assert_eq!(chain_proof_height(proof), 100);
    }
}
