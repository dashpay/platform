//! Proof lifecycle: waiting for proofs, validation, and IS-lock to ChainLock upgrade.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::{OutPoint, Txid};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::transaction_record::TransactionRecord;

use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;

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
/// - **Poll loops** (`wait_for_chain_lock`, `wait_for_proof`) typically
///   downgrade to `None` for the current iteration so the next tick
///   retries — see [`record_or_persister_or_log`] for that policy.
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

/// Variant of [`record_or_persister`] that swallows persister errors
/// as `None` after a `warn`-level log. Use this from poll loops where
/// the next iteration retries — a hard error from a single tick would
/// abort the whole poll prematurely.
pub(super) fn record_or_persister_or_log(
    in_memory: Option<TransactionRecord>,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &Txid,
) -> Option<TransactionRecord> {
    match record_or_persister(in_memory, persister, txid) {
        Ok(opt) => opt,
        Err(e) => {
            tracing::warn!(
                txid = %txid,
                error = %e,
                "Persister fallback for core tx record failed; \
                 treating as miss for this poll iteration"
            );
            None
        }
    }
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
    /// Subscribes to SPV events and waits until the transaction's block
    /// is chain-locked. `timeout` is `Option<Duration>`: `None` waits
    /// **indefinitely** (a ChainLock is guaranteed finality that will
    /// eventually arrive, so a broadcast lock is pending, not failed).
    async fn wait_for_chain_lock(
        &self,
        account_index: u32,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<u32, PlatformWalletError> {
        use key_wallet::transaction_checking::TransactionContext;

        let deadline = timeout.map(|t| tokio::time::Instant::now() + t);

        loop {
            // Arm the `Notify` future BEFORE the state check, closing
            // the missed-wakeup race in dashpay/platform#3641
            // (Found-008): `notify_waiters()` only wakes already-
            // registered waiters and does NOT store a permit, so a
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
            if let Some(record) =
                record_or_persister_or_log(in_memory, &self.persister, &out_point.txid)
            {
                if matches!(record.context, TransactionContext::InChainLockedBlock(_)) {
                    if let Some(h) = record.height() {
                        return Ok(h);
                    }
                }
            }

            // Wait for a lock event notification (or timeout, when one is
            // configured). The `notified` future is the one we armed above,
            // so any CL/IS event since then is already buffered into it.
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
            // the missed-wakeup race in dashpay/platform#3641
            // (Found-008): `notify_waiters()` only wakes already-
            // registered waiters and does NOT store a permit, so an
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
            if let Some(record) =
                record_or_persister_or_log(in_memory, &self.persister, &out_point.txid)
            {
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
                        // Per-record context isn't `InChainLockedBlock`
                        // yet, but the wallet's global
                        // `last_applied_chain_lock` may already cover
                        // this record's block height — e.g. on
                        // app-launch catch-up, when a record is
                        // re-injected into the in-memory map at its
                        // persisted `InBlock` context, after the CL
                        // event that would have promoted it has already
                        // fired and passed. The wallet's metadata holds
                        // the BLS-verified ChainLock from that earlier
                        // event, so we can build a Chain proof directly
                        // — same cryptographic guarantee as the
                        // `InChainLockedBlock` arm above.
                        //
                        // Chain-id check: refuse the fallback if the
                        // wallet's declared network doesn't match the
                        // SDK's. A persisted `last_applied_chain_lock`
                        // from a different network (config drift /
                        // restore-from-backup gone wrong) would have
                        // us building a proof against the wrong chain;
                        // Platform would reject with 10506 and the
                        // submission layer would burn its full retry
                        // budget on impossible-to-satisfy bumps.
                        if let Some(height) = record.height() {
                            let (wallet_cl_height, wallet_network) = {
                                let wm = self.wallet_manager.read().await;
                                let info = wm.get_wallet_info(&self.wallet_id);
                                let cl_h = info
                                    .as_ref()
                                    .and_then(|i| {
                                        i.core_wallet.metadata.last_applied_chain_lock.as_ref()
                                    })
                                    .map(|cl| cl.block_height);
                                let net = info.map(|i| {
                                    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
                                    i.network()
                                });
                                (cl_h, net)
                            };
                            let networks_match =
                                matches!(wallet_network, Some(n) if n == self.sdk.network);
                            if matches!(wallet_cl_height, Some(h) if h >= height) && networks_match
                            {
                                tracing::info!(
                                    "Building ChainLock proof for tx {} from wallet's \
                                     last_applied_chain_lock (record at height={}, \
                                     wallet_cl={}) — per-record promotion missed by \
                                     SPV catch-up cascade",
                                    out_point.txid,
                                    height,
                                    wallet_cl_height.unwrap_or(0),
                                );
                                return Ok(dpp::prelude::AssetLockProof::Chain(
                                    ChainAssetLockProof {
                                        core_chain_locked_height: height,
                                        out_point: *out_point,
                                    },
                                ));
                            } else if matches!(wallet_cl_height, Some(h) if h >= height)
                                && !networks_match
                            {
                                tracing::error!(
                                    sdk_network = ?self.sdk.network,
                                    wallet_network = ?wallet_network,
                                    outpoint = %out_point,
                                    "wait_for_proof: REFUSING to build CL proof from \
                                     wallet_cl_height fallback — wallet's declared \
                                     network does not match the SDK's. Persisted \
                                     state likely loaded into the wrong network."
                                );
                            }
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

    /// Test persister that always errors out on `get_core_tx_record`,
    /// to exercise the error-swallowing branch in `record_or_persister`.
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
        // their own policy (one-shot recovery logs at error and
        // degrades; poll loops downgrade to None for one tick via
        // `record_or_persister_or_log`).
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));

        let resolved = record_or_persister(None, &persister, &unknown_txid);
        assert!(resolved.is_err());
    }

    #[test]
    fn record_or_persister_or_log_swallows_backend_errors_as_none() {
        // The poll-loop variant downgrades errors to `None` (after a
        // `warn` log) so a transient backend failure on one tick
        // doesn't abort the whole poll.
        let unknown_txid = Txid::from([0xFF; 32]);
        let persister = wallet_persister(Arc::new(ErroringStore));

        let resolved = record_or_persister_or_log(None, &persister, &unknown_txid);
        assert!(resolved.is_none());
    }
}
