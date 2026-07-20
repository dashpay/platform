//! Asset lock transaction building.
//!
//! Contains methods for building asset lock transactions, peeking at funding
//! addresses, and the unified `create_funded_asset_lock_proof` entry point.

use crate::broadcaster::TransactionBroadcaster;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, Transaction, TxOut};
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::signer::Signer;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;

use crate::error::PlatformWalletError;

use super::manager::{AssetLockManager, DEFAULT_FEE_PER_KB};
use super::tracked::{AssetLockStatus, TrackedAssetLock};

// ---------------------------------------------------------------------------
// Asset lock transaction building
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Build an asset lock transaction using the key-wallet builder.
    ///
    /// Delegates UTXO selection, fee calculation, and signing to
    /// `ManagedWalletInfo::build_asset_lock_with_signer`. The host
    /// never sees a raw credit-output private key — the returned
    /// `DerivationPath` is what the caller hands back to the same
    /// `signer` when the credit output is later consumed on Platform.
    ///
    /// # Arguments
    ///
    /// * `amount_duffs` — Amount to lock in duffs.
    /// * `account_index` — BIP44 account index to select UTXOs from.
    /// * `funding_type` — Which account to derive the one-time key from
    ///   (e.g., `IdentityRegistration`, `IdentityTopUp`).
    /// * `identity_index` — Identity index (used by `IdentityTopUp`, ignored by others).
    /// * `signer` — External signer that produces both the funding-input
    ///   P2PKH signatures and the credit-output public key. For Swift,
    ///   this is typically a
    ///   [`MnemonicResolverCoreSigner`](crate::wallet::asset_lock::build)
    ///   from `platform-wallet-ffi` — built on top of the
    ///   Keychain-resolver vtable so private keys never cross the FFI
    ///   boundary.
    pub async fn build_asset_lock_transaction<S: Signer>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(Transaction, DerivationPath), PlatformWalletError> {
        if amount_duffs == 0 {
            return Err(PlatformWalletError::AssetLockTransaction(
                "Amount must be greater than zero".to_string(),
            ));
        }

        let mut wm = self.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_and_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

        // 1. Peek at the next unused address from the funding account to
        //    build the credit output P2PKH script.
        let funding_address = Self::peek_next_funding_address(
            &mut info.core_wallet,
            wallet,
            funding_type,
            identity_index,
        )?;

        // 2. Build the credit output for the asset lock payload.
        let credit_output = TxOut {
            value: amount_duffs,
            script_pubkey: funding_address.script_pubkey(),
        };

        let funding = CreditOutputFunding {
            output: credit_output,
            funding_type,
            identity_index,
        };

        // 3. Delegate to the key-wallet signer-driven builder.
        let result = info
            .core_wallet
            .build_asset_lock_with_signer(
                wallet,
                account_index,
                vec![funding],
                DEFAULT_FEE_PER_KB,
                signer,
            )
            .await
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Asset lock builder failed: {}",
                    e
                ))
            })?;

        // 4. Pull the (pubkey, path) for our single credit output.
        //
        // `build_asset_lock_with_signer` always returns the `Public`
        // variant. The `Private` arm would only come from the soft-
        // wallet `build_asset_lock` path which we no longer call from
        // platform-wallet — defensively bail if it appears.
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockCreditKeys;
        let path = match result.keys {
            AssetLockCreditKeys::Public(mut keys) => {
                let (_pubkey, path) = keys.drain(..).next().ok_or_else(|| {
                    PlatformWalletError::AssetLockTransaction(
                        "Builder returned no credit-output keys".to_string(),
                    )
                })?;
                path
            }
            AssetLockCreditKeys::Private(_) => {
                return Err(PlatformWalletError::AssetLockTransaction(
                    "Builder returned Private keys; signer-driven path expected Public".to_string(),
                ));
            }
        };

        Ok((result.transaction, path))
    }

    /// Peek at the next unused address from a funding account without
    /// consuming it (i.e. without marking it as used).
    ///
    /// The key-wallet builder's `next_private_key` will later find the same
    /// address, derive the private key, and mark it as used.
    fn peek_next_funding_address(
        wallet_info: &mut ManagedWalletInfo,
        wallet: &Wallet,
        funding_type: AssetLockFundingType,
        identity_index: u32,
    ) -> Result<DashAddress, PlatformWalletError> {
        let (managed_account, account_xpub) = match funding_type {
            AssetLockFundingType::IdentityRegistration => {
                let xpub = wallet
                    .accounts
                    .identity_registration
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_registration
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity registration account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUp => {
                let xpub = wallet
                    .accounts
                    .identity_topup
                    .get(&identity_index)
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup
                    .get_mut(&identity_index)
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(format!(
                            "Identity top-up account for index {} not found",
                            identity_index
                        ))
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityTopUpNotBound => {
                let xpub = wallet
                    .accounts
                    .identity_topup_not_bound
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_topup_not_bound
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity top-up (unbound) account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::IdentityInvitation => {
                let xpub = wallet
                    .accounts
                    .identity_invitation
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .identity_invitation
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Identity invitation account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::AssetLockAddressTopUp => {
                let xpub = wallet
                    .accounts
                    .asset_lock_address_topup
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .asset_lock_address_topup
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Asset lock address top-up account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => {
                let xpub = wallet
                    .accounts
                    .asset_lock_shielded_address_topup
                    .as_ref()
                    .map(|a| a.account_xpub);
                let account = wallet_info
                    .accounts
                    .asset_lock_shielded_address_topup
                    .as_mut()
                    .ok_or_else(|| {
                        PlatformWalletError::AssetLockTransaction(
                            "Asset lock shielded address top-up account not found".to_string(),
                        )
                    })?;
                (account, xpub)
            }
        };

        // Get the next unused address from the pool. `next_address`
        // always persists the newly-generated address into the pool's
        // state so the builder's `next_private_key` can find it. The
        // address is NOT marked as used yet — that happens inside the
        // builder after a successful transaction build.
        managed_account
            .next_address(account_xpub.as_ref(), false)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to get next funding address: {}",
                    e
                ))
            })
    }

    /// Persist the asset-lock funding accounts' address-pool snapshots so a
    /// consumed `funding_index` survives an app restart.
    ///
    /// The `IdentityRegistration` / `IdentityTopUp` / `IdentityInvitation` /
    /// asset-lock-top-up accounts fund credit outputs that live only in an
    /// asset-lock special-tx payload; the on-chain output is an `OP_RETURN`
    /// burn, so these addresses never appear as UTXOs and SPV can never
    /// rediscover their used indices. Without persisting the pool, the
    /// in-memory `mark_used` is lost on restart and `next_unused` resets to 0 —
    /// which for `IdentityInvitation` reuses the EXPORTED one-time voucher key
    /// across invitations (a bearer-key reuse: one leaked link could then claim
    /// every same-key invite). The pool round-trips through the existing
    /// `account_address_pools` persist path and is rebuilt by
    /// `restore_address_pool` on load. Funds accounts are skipped — they already
    /// persist their pools via the normal address-sync path. Best-effort.
    ///
    /// The snapshot re-acquires a read lock after the build's write lock is
    /// released, so callers must serialize asset-lock builds per wallet (the app
    /// creates invitations one-at-a-time from the UI); two concurrent builds on
    /// one wallet could otherwise persist a stale snapshot that drops the higher
    /// burned index — self-healing on the next build, but a residual to respect.
    async fn persist_asset_lock_account_pools(
        &self,
    ) -> Result<(), crate::changeset::PersistenceError> {
        use crate::changeset::{AccountAddressPoolEntry, PlatformWalletChangeSet};
        use key_wallet::account::AccountType;

        let entries: Vec<AccountAddressPoolEntry> = {
            let wm = self.wallet_manager.read().await;
            let Some(wallet_info) = wm.get_wallet_info(&self.wallet_id) else {
                return Ok(());
            };
            wallet_info
                .core_wallet
                .all_managed_accounts()
                .iter()
                .filter(|managed| {
                    matches!(
                        managed.managed_account_type().to_account_type(),
                        AccountType::IdentityRegistration
                            | AccountType::IdentityTopUp { .. }
                            | AccountType::IdentityTopUpNotBoundToIdentity
                            | AccountType::IdentityInvitation
                            | AccountType::AssetLockAddressTopUp
                            | AccountType::AssetLockShieldedAddressTopUp
                    )
                })
                .flat_map(|managed| {
                    let account_type = managed.managed_account_type().to_account_type();
                    managed
                        .managed_account_type()
                        .address_pools()
                        .into_iter()
                        .filter_map(move |pool| {
                            let addresses: Vec<key_wallet::AddressInfo> =
                                pool.addresses.values().cloned().collect();
                            if addresses.is_empty() {
                                return None;
                            }
                            Some(AccountAddressPoolEntry {
                                account_type,
                                pool_type: pool.pool_type,
                                addresses,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        };

        if entries.is_empty() {
            return Ok(());
        }
        self.persister.store(PlatformWalletChangeSet {
            account_address_pools: entries,
            ..Default::default()
        })
    }

    /// Build, broadcast, and wait for an asset lock proof.
    ///
    /// This is the **unified** entry point for obtaining a funded asset lock
    /// proof, replacing the earlier `create_registration_asset_lock_proof` and
    /// `create_topup_asset_lock_proof` methods.
    ///
    /// ## Flow
    ///
    /// 1. Build the asset lock transaction via the key-wallet
    ///    signer-driven builder.
    /// 2. Track the lifecycle as `Built` (in-memory).
    /// 3. Broadcast the transaction.
    /// 4. Wait for an InstantLock or ChainLock proof via the event channel.
    /// 5. Track the lifecycle as `InstantSendLocked` or `ChainLocked`.
    /// 6. Return `(proof, credit_output_derivation_path, txid)` — the
    ///    caller hands the path back to the same `signer` when
    ///    consuming the credit on Platform.
    ///
    /// ## Persistence
    ///
    /// This method tracks the asset lock in memory before broadcasting, so
    /// the lock is recoverable even if the proof wait is interrupted. However,
    /// the `AssetLockManager` does not persist state directly — **callers MUST
    /// persist the wallet state** after this method returns (or after broadcast
    /// if crash-safety before finality is required). The changeset system
    /// (`AssetLockChangeSet`) will capture the tracked lock state when the
    /// persister flushes.
    ///
    /// ## Parameters
    ///
    /// * `amount_duffs` — Amount to lock.
    /// * `account_index` — BIP44 account index to select UTXOs from.
    /// * `funding_type` — Which account to derive the one-time key from.
    /// * `identity_index` — HD identity index (for `IdentityTopUp`, this is
    ///   the registration index identifying which identity is being topped up).
    /// * `signer` — External ECDSA signer (Swift Keychain-backed in
    ///   production via `MnemonicResolverCoreSigner`).
    pub async fn create_funded_asset_lock_proof<S: Signer>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath, OutPoint), PlatformWalletError> {
        let (path, out_point) = self
            .broadcast_funded_asset_lock(
                amount_duffs,
                account_index,
                funding_type,
                identity_index,
                signer,
            )
            .await?;
        let proof = self
            .wait_for_funded_asset_lock_proof(&out_point, account_index)
            .await?;
        Ok((proof, path, out_point))
    }

    /// Broadcast half of [`Self::create_funded_asset_lock_proof`] — steps 1–4:
    /// build + fund the asset-lock transaction, persist the funding account's
    /// address pool, track the lifecycle row, and broadcast. Returns as soon as
    /// the transaction is on the wire (status `Broadcast`), BEFORE any proof
    /// wait, so a caller can durably record its own bookkeeping for the funded
    /// lock (e.g. the inviter-side invitation row) between the broadcast and
    /// the potentially long proof wait in
    /// [`Self::wait_for_funded_asset_lock_proof`].
    pub(crate) async fn broadcast_funded_asset_lock<S: Signer>(
        &self,
        amount_duffs: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        signer: &S,
    ) -> Result<(DerivationPath, OutPoint), PlatformWalletError> {
        // Serialize build→persist so a concurrent build cannot interleave its
        // pool snapshot with ours. The snapshot is collected from live wallet
        // state at persist time; without this guard, build A's snapshot
        // (missing B's just-marked index) can be persisted AFTER B's,
        // rolling the durable used-index state back — after a restart the
        // next invitation would re-select B's index and re-export the same
        // bearer voucher key. Held through the persist/flush gate below and
        // dropped before the broadcast (only snapshot ordering needs
        // serializing; the UI's own single-flight guard is NOT sufficient —
        // a dismissed sheet's unstructured task keeps running).
        // Test-only occupancy gauge for the serialization gate (see
        // `build_serial_gate`). RAII so every exit path — including the
        // pre-broadcast aborts below — decrements.
        #[cfg(test)]
        struct GateGauge<'a>(&'a std::sync::atomic::AtomicUsize);
        #[cfg(test)]
        impl Drop for GateGauge<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        #[cfg(test)]
        let _gate_gauge = {
            self.build_serial_gate
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            GateGauge(&self.build_serial_gate)
        };
        let build_persist_guard = self.build_persist_serial.lock().await;

        // 1. Build the asset lock transaction.
        let (tx, path) = self
            .build_asset_lock_transaction(
                amount_duffs,
                account_index,
                funding_type,
                identity_index,
                signer,
            )
            .await?;

        let txid = tx.txid();
        let out_point = OutPoint::new(txid, 0);

        // Persist the funding account's address pool now that the build marked
        // its index used. These asset-lock accounts fund OP_RETURN-payload
        // credit outputs that never appear as on-chain UTXOs, so SPV can never
        // rediscover the used index — the persisted pool is the only thing that
        // carries `funding_index` across a restart. For an INVITATION this write
        // is a security gate: the voucher key is exported into a bearer link, so
        // a failed persist would let the next restart reuse this index/key.
        // `store()` alone is only a buffer hint under the persistence contract
        // (backends may defer I/O until `flush`), so the invitation gate also
        // drives `flush()` — the contract's durability boundary — before
        // anything hits the wire.
        // Aborting BEFORE broadcast is harmless (no tx on the wire); the other
        // asset-lock accounts keep their keys on-device, so they stay best-effort.
        let pool_durability = match self.persist_asset_lock_account_pools().await {
            Ok(()) if funding_type == AssetLockFundingType::IdentityInvitation => {
                self.persister.flush()
            }
            other => other,
        };
        if let Err(e) = pool_durability {
            tracing::error!(error = %e, "failed to persist asset-lock funding index");
            if funding_type == AssetLockFundingType::IdentityInvitation {
                return Err(PlatformWalletError::AssetLockTransaction(format!(
                    "aborted before broadcast: could not durably record the invitation \
                     funding index (broadcasting anyway would risk voucher-key reuse on \
                     restart): {e}"
                )));
            }
        }

        // The durable snapshot now includes this build's index; broadcast and
        // everything after it can safely run concurrently with the next build.
        drop(build_persist_guard);

        // 2. Track as Built and queue the changeset onto the persister
        //    so a crash after broadcast leaves a row we can recover from.
        let cs_built = self
            .track_asset_lock(TrackedAssetLock {
                out_point,
                transaction: tx.clone(),
                account_index,
                funding_type,
                identity_index,
                amount: amount_duffs,
                status: AssetLockStatus::Built,
                proof: None,
            })
            .await;
        self.queue_asset_lock_changeset(cs_built);

        tracing::debug!(
            %txid,
            "Asset lock tracked as Built and queued for persistence; broadcasting."
        );

        // 3. Broadcast. On a definitive pre-send rejection, untrack the
        //    `Built` row BEFORE releasing the funding reservation (the
        //    asset-lock builder funds from the BIP44 account at
        //    `account_index`): while the reservation is held the inputs
        //    cannot be re-selected by a new build, and once the row is gone
        //    `resume_asset_lock` can no longer re-drive the rejected
        //    transaction — so at no point is the row resumable while its
        //    inputs are re-spendable. A `MaybeSent` failure keeps both the
        //    reservation and the resumable row.
        if let Err(e) = self.broadcaster.broadcast(&tx).await {
            if matches!(e, crate::broadcaster::BroadcastError::Rejected { .. }) {
                let cs_untrack = self.untrack_asset_lock(&out_point).await;
                // Release only when the Built row was actually removed. If
                // the untrack guard fired instead — a concurrent
                // `resume_asset_lock` advanced the row past `Built`, positive
                // evidence the transaction reached the network after all —
                // the inputs must stay reserved exactly like a `MaybeSent`
                // outcome, or the still-tracked row would be resumable while
                // its inputs are re-spendable.
                let removed_built_row = cs_untrack.removed.contains(&out_point);
                self.queue_asset_lock_changeset(cs_untrack);
                if removed_built_row {
                    crate::wallet::reservations::release_reservation_after_rejected_broadcast(
                        &self.wallet_manager,
                        &self.wallet_id,
                        key_wallet::account::account_type::StandardAccountType::BIP44Account,
                        account_index,
                        &tx,
                    )
                    .await;
                }
            }
            return Err(e.into());
        }

        // 4. Transition to Broadcast and queue the changeset.
        let cs_broadcast = self
            .advance_asset_lock_status(&out_point, AssetLockStatus::Broadcast, None)
            .await?;
        self.queue_asset_lock_changeset(cs_broadcast);

        Ok((path, out_point))
    }

    /// Proof half of [`Self::create_funded_asset_lock_proof`] — steps 5–6:
    /// wait for the InstantSend/ChainLock proof of an already-broadcast asset
    /// lock, upgrade it when Platform would reject it, and attach it to the
    /// tracked row.
    pub(crate) async fn wait_for_funded_asset_lock_proof(
        &self,
        out_point: &OutPoint,
        account_index: u32,
    ) -> Result<dpp::prelude::AssetLockProof, PlatformWalletError> {
        // 5. Wait for proof via SPV events. The 300s bound is an
        //    InstantSend-preference window, NOT a finality timeout: on
        //    expiry the resolver falls back to an unbounded ChainLock wait
        //    (`upgrade_to_chain_lock_proof(None)`), so a broadcast lock is
        //    never surfaced as "failed" just because IS was slow.
        let proof = self
            .wait_for_proof(out_point, Some(Duration::from_secs(300)))
            .await?;

        // 5b. If we got an IS-lock proof, check whether the transaction is
        // old enough that Platform might reject it. If so, upgrade to a
        // ChainLock proof proactively.
        let proof = self
            .validate_or_upgrade_proof(proof, account_index, out_point)
            .await?;

        // 6. Attach proof — status matches the proof type received —
        //    and queue the final changeset.
        let status = match &proof {
            dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
            dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
        };
        let cs_final = self
            .advance_asset_lock_status(out_point, status, Some(proof.clone()))
            .await?;
        self.queue_asset_lock_changeset(cs_final);

        Ok(proof)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dashcore::OutPoint;
    use key_wallet::account::account_type::StandardAccountType;
    use tokio::sync::Notify;

    use async_trait::async_trait;
    use dashcore::{Transaction, Txid};
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysOkBroadcaster,
        AlwaysRejectedBroadcaster, WalletSigner,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::AssetLockStatus;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::wallet::platform_wallet::WalletId;
    use crate::{AssetLockFundingType, PlatformWalletError};

    /// Persistence stub that records every stored changeset so tests can
    /// assert what the asset-lock flow queued. `fail_flush` simulates a
    /// backend whose durability boundary fails; `flushes` counts `flush`
    /// calls so tests can assert the invitation gate drove one.
    #[derive(Default)]
    struct CapturingPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
        flushes: std::sync::atomic::AtomicUsize,
        fail_flush: bool,
    }

    impl CapturingPersistence {
        /// Outpoints queued for persisted-row deletion across all stored
        /// changesets.
        fn removed_outpoints(&self) -> Vec<OutPoint> {
            self.stored
                .lock()
                .expect("capturing persistence mutex")
                .iter()
                .filter_map(|cs| cs.asset_locks.as_ref())
                .flat_map(|al| al.removed.iter().copied())
                .collect()
        }
    }

    impl PlatformWalletPersistence for CapturingPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored
                .lock()
                .expect("capturing persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            self.flushes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail_flush {
                return Err(PersistenceError::backend("simulated flush failure"));
            }
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Builds an `AssetLockManager` over the shared BIP44-funded fixture.
    async fn funded_asset_lock_manager<B: TransactionBroadcaster>(
        broadcaster: Arc<B>,
    ) -> (
        Arc<AssetLockManager<B>>,
        WalletSigner,
        Arc<CapturingPersistence>,
    ) {
        let persistence = Arc::new(CapturingPersistence::default());
        let (manager, signer) =
            funded_asset_lock_manager_with_persistence(broadcaster, Arc::clone(&persistence)).await;
        (manager, signer, persistence)
    }

    /// Like [`funded_asset_lock_manager`] but over a caller-built persistence
    /// stub (e.g. one with `fail_flush` set).
    async fn funded_asset_lock_manager_with_persistence<B: TransactionBroadcaster>(
        broadcaster: Arc<B>,
        persistence: Arc<CapturingPersistence>,
    ) -> (Arc<AssetLockManager<B>>, WalletSigner) {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(wallet_id, persistence as Arc<dyn PlatformWalletPersistence>),
        ));

        (manager, signer)
    }

    /// Regression: a build must persist the funding account's address-pool
    /// snapshot with the newly-used index. The asset-lock funding accounts fund
    /// OP_RETURN-payload credit outputs that never appear as on-chain UTXOs, so
    /// SPV can't rediscover the used index — the persisted pool is the only
    /// thing that carries `funding_index` across a restart. Before the fix the
    /// pool was never emitted, so `funding_index` reset to 0 each launch and (for
    /// `IdentityInvitation`) the EXPORTED one-time voucher key was reused across
    /// invitations. The pool snapshot is emitted right after the tx is built,
    /// before broadcast, so a rejected broadcast still exercises it.
    #[tokio::test]
    async fn asset_lock_build_persists_funding_account_used_index() {
        use key_wallet::account::AccountType;

        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;

        let _ = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await;

        let stored = persistence
            .stored
            .lock()
            .expect("capturing persistence mutex");
        let persisted_invitation_used = stored.iter().any(|cs| {
            cs.account_address_pools.iter().any(|entry| {
                matches!(entry.account_type, AccountType::IdentityInvitation)
                    && entry.addresses.iter().any(|a| a.used)
            })
        });
        assert!(
            persisted_invitation_used,
            "a build must persist the IdentityInvitation account's pool with the used \
             funding index; without it funding_index resets on restart and the exported \
             voucher key is reused across invitations"
        );
    }

    /// A definitively rejected asset-lock broadcast must untrack the `Built`
    /// row (in-memory and via the changeset's `removed` set) and release the
    /// funding reservation, so nothing can resume the dead transaction and a
    /// fresh funding attempt can reselect the inputs immediately.
    #[tokio::test]
    async fn rejected_asset_lock_broadcast_untracks_row_and_releases_reservation() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysRejectedBroadcaster)).await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "rejected broadcast should surface as TransactionBroadcast, got {result:?}"
        );

        // The Built row is gone in memory…
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert!(
                info.tracked_asset_locks.is_empty(),
                "rejected lock must be untracked, got {:?}",
                info.tracked_asset_locks
            );
        }
        // …and its persisted row was queued for deletion.
        assert_eq!(
            persistence.removed_outpoints().len(),
            1,
            "exactly the rejected lock's outpoint should be queued as removed"
        );

        // The funding reservation was released: a fresh build over the same
        // single-UTXO wallet can reselect the inputs immediately.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            rebuild.is_ok(),
            "rebuild after a rejected broadcast should reselect the released \
             inputs, got {rebuild:?}"
        );
    }

    /// An *ambiguous* asset-lock broadcast failure must keep both the funding
    /// reservation and the resumable `Built` row: the transaction may already
    /// be propagating, so a retry must not double-spend and a resume must
    /// stay possible.
    #[tokio::test]
    async fn ambiguous_asset_lock_broadcast_keeps_reservation_and_built_row() {
        let (manager, signer, persistence) =
            funded_asset_lock_manager(Arc::new(AlwaysMaybeSentBroadcaster)).await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(
                result,
                Err(PlatformWalletError::TransactionBroadcastUnconfirmed(_))
            ),
            "ambiguous broadcast should surface as TransactionBroadcastUnconfirmed, got {result:?}"
        );

        // The Built row survives for a later resume…
        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert_eq!(info.tracked_asset_locks.len(), 1);
            let lock = info.tracked_asset_locks.values().next().expect("built row");
            assert_eq!(lock.status, AssetLockStatus::Built);
        }
        // …no persisted-row deletion was queued…
        assert!(
            persistence.removed_outpoints().is_empty(),
            "ambiguous failure must not queue a row deletion"
        );

        // …and the reservation is kept: a fresh build cannot reselect the
        // single reserved UTXO and fails at input selection.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(rebuild, Err(PlatformWalletError::AssetLockTransaction(_))),
            "rebuild must fail at input selection while the reservation is \
             kept, got {rebuild:?}"
        );
    }

    /// Broadcaster that simulates the racing interleave the release gate
    /// exists for: "during" the broadcast a concurrent `resume_asset_lock`
    /// advances the tracked row to `Broadcast`, then the original call still
    /// comes back `Rejected`. The advanced row is positive evidence the
    /// transaction reached the network, so the cleanup must keep it AND keep
    /// the funding reservation.
    struct RejectAfterConcurrentResumeBroadcaster {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
    }

    #[async_trait]
    impl TransactionBroadcaster for RejectAfterConcurrentResumeBroadcaster {
        async fn broadcast(&self, _transaction: &Transaction) -> Result<Txid, BroadcastError> {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet present");
            let lock = info
                .tracked_asset_locks
                .values_mut()
                .next()
                .expect("Built row tracked before broadcast");
            lock.status = AssetLockStatus::Broadcast;
            drop(wm);
            Err(BroadcastError::Rejected {
                reason: "simulated rejection racing a concurrent resume".to_string(),
            })
        }
    }

    /// If a concurrent resume advanced the row past `Built` in the rejection
    /// window, the cleanup must keep the row (guard) AND keep the funding
    /// reservation (release gate) — otherwise the still-tracked transaction
    /// would be resumable while its inputs are re-spendable.
    #[tokio::test]
    async fn rejected_broadcast_racing_concurrent_resume_keeps_row_and_reservation() {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;

        let broadcaster = Arc::new(RejectAfterConcurrentResumeBroadcaster {
            wallet_manager: Arc::clone(&wallet_manager),
            wallet_id,
        });
        let persistence = Arc::new(CapturingPersistence::default());
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "rejection should still surface, got {result:?}"
        );

        // The concurrently-advanced row survives the cleanup…
        {
            let wm = wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            assert_eq!(info.tracked_asset_locks.len(), 1);
            let lock = info.tracked_asset_locks.values().next().expect("row kept");
            assert_eq!(lock.status, AssetLockStatus::Broadcast);
        }
        // …no persisted-row deletion was queued…
        assert!(
            persistence.removed_outpoints().is_empty(),
            "advanced row must not be queued for deletion"
        );

        // …and the reservation was NOT released: a fresh build cannot
        // reselect the single reserved UTXO.
        let rebuild = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(rebuild, Err(PlatformWalletError::AssetLockTransaction(_))),
            "rebuild must fail at input selection while the reservation is \
             kept for the advanced row, got {rebuild:?}"
        );
    }

    /// Persistence stub whose FIRST address-pool store blocks on a 2-party
    /// barrier until the test arrives, holding that build inside its persist
    /// while the other build runs. Later stores pass straight through.
    struct GatedPoolPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
        first_pool_store: std::sync::Barrier,
        gate_used: std::sync::atomic::AtomicBool,
        /// Total pool-bearing `store` calls seen (counted before parking or
        /// pushing). Reaching 2 while the first store is parked proves the
        /// second build persisted concurrently — the exact regression.
        pool_stores_seen: std::sync::atomic::AtomicUsize,
        /// Set just before the first pool store parks at the barrier, so the
        /// test can spawn the second build only once the first is provably
        /// inside its persist.
        first_parked: std::sync::atomic::AtomicBool,
    }

    impl PlatformWalletPersistence for GatedPoolPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if !changeset.account_address_pools.is_empty() {
                self.pool_stores_seen
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if !self
                    .gate_used
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    self.first_parked
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    self.first_pool_store.wait();
                }
            }
            self.stored
                .lock()
                .expect("gated persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Two concurrent invitation builds must not be able to roll the durable
    /// used-index snapshot backwards. The pool snapshot is collected from live
    /// wallet state at persist time; unserialized, build A's snapshot
    /// (collected before B marked its index) can be persisted AFTER B's, so
    /// the last durable snapshot loses B's index — after a restart the next
    /// invitation re-selects it and re-exports the same bearer voucher key.
    /// The barrier holds the first-persisting build inside its store while
    /// the other runs: without the
    /// build→persist serialization, B's fuller snapshot lands first and A's
    /// stale one overwrites it (this test is red); with it, B parks until A's
    /// persist completes, so snapshots are monotonic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_invitation_builds_cannot_roll_back_the_used_index_snapshot() {
        use key_wallet::account::AccountType;

        let (wallet_manager, wallet_id, _balance, signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &[10_000_000, 10_000_000],
            )
            .await;

        let persistence = Arc::new(GatedPoolPersistence {
            stored: Mutex::new(Vec::new()),
            first_pool_store: std::sync::Barrier::new(2),
            gate_used: std::sync::atomic::AtomicBool::new(false),
            pool_stores_seen: std::sync::atomic::AtomicUsize::new(0),
            first_parked: std::sync::atomic::AtomicBool::new(false),
        });
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let manager = Arc::new(AssetLockManager::new(
            sdk,
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysOkBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        ));

        let manager_a = Arc::clone(&manager);
        let signer_a = signer.clone();
        let a = tokio::spawn(async move {
            manager_a
                .broadcast_funded_asset_lock(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityInvitation,
                    0,
                    &signer_a,
                )
                .await
        });

        // Spawn B only once A is provably parked inside its pool persist
        // (holding `build_persist_serial`), so the interleaving is staged,
        // not scheduled.
        while !persistence
            .first_parked
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        let manager_b = Arc::clone(&manager);
        let b = tokio::spawn(async move {
            manager_b
                .broadcast_funded_asset_lock(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityInvitation,
                    0,
                    &signer,
                )
                .await
        });

        // Release A only after B has provably reached the relevant stage —
        // no scheduling assumption. Exactly one of two states must occur:
        // - `pool_stores_seen >= 2`: B built and persisted its own (fuller)
        //   snapshot while A was parked — the regression manifested (an
        //   unserialized implementation always reaches this state, however
        //   slowly, so the rollback assertion below fires deterministically);
        // - `build_serial_gate >= 2`: B is queued at the build→persist
        //   serialization gate while A still holds it, so B cannot have
        //   collected a snapshot yet — the fixed behavior, verified
        //   positively rather than by the absence of a store within a delay.
        loop {
            let regressed = persistence
                .pool_stores_seen
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 2;
            let serialized = manager
                .build_serial_gate
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 2;
            if regressed || serialized {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        persistence.first_pool_store.wait();

        a.await.expect("join A").expect("build A succeeds");
        b.await.expect("join B").expect("build B succeeds");

        // Successive persisted invitation-pool snapshots must never lose a
        // used index, and both builds' indices must end up durably used.
        let stored = persistence.stored.lock().expect("gated persistence mutex");
        let mut last_used = 0usize;
        for cs in stored.iter() {
            for entry in cs
                .account_address_pools
                .iter()
                .filter(|e| matches!(e.account_type, AccountType::IdentityInvitation))
            {
                let used = entry.addresses.iter().filter(|a| a.used).count();
                assert!(
                    used >= last_used,
                    "invitation pool snapshot rolled back: {used} used after {last_used}"
                );
                last_used = used;
            }
        }
        assert!(
            last_used >= 2,
            "both builds' funding indices must be durably marked used, got {last_used}"
        );
    }

    /// The invitation pre-broadcast gate must treat `flush()` — the
    /// persistence contract's durability boundary — as part of recording the
    /// funding index, and abort BEFORE broadcast when it fails. `store()`
    /// alone may only buffer; an unflushed funding index can be re-selected
    /// after a restart, re-exporting the same bearer voucher key.
    #[tokio::test]
    async fn invitation_gate_aborts_before_broadcast_when_flush_fails() {
        let persistence = Arc::new(CapturingPersistence {
            fail_flush: true,
            ..Default::default()
        });
        // The broadcaster rejects loudly: reaching it at all would surface as
        // a `TransactionBroadcast` error, so the gate's own "aborted before
        // broadcast" message proves nothing hit the wire.
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await;
        match result {
            Err(PlatformWalletError::AssetLockTransaction(msg)) => assert!(
                msg.contains("aborted before broadcast"),
                "expected the pre-broadcast durability abort, got: {msg}"
            ),
            other => panic!("expected the pre-broadcast durability abort, got {other:?}"),
        }
        assert!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1,
            "the invitation gate must have driven flush()"
        );
    }

    /// Non-invitation funding types stay best-effort: their one-time keys
    /// never leave the device, so a failing durability boundary must NOT gate
    /// them — the flow proceeds to broadcast.
    #[tokio::test]
    async fn flush_failure_does_not_gate_non_invitation_funding() {
        let persistence = Arc::new(CapturingPersistence {
            fail_flush: true,
            ..Default::default()
        });
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysRejectedBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let result = manager
            .create_funded_asset_lock_proof(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        assert!(
            matches!(result, Err(PlatformWalletError::TransactionBroadcast(_))),
            "a registration build must reach the broadcaster despite the flush \
             failure (best-effort persistence), got {result:?}"
        );
    }

    /// The broadcast half returns as soon as the transaction is on the wire:
    /// the tracked row is `Broadcast` (recoverable/resumable) and the
    /// invitation funding pool was persisted AND flushed — all BEFORE any
    /// proof wait (the test completing at all proves no SPV wait ran), so a
    /// caller can durably record its own bookkeeping for the funded lock
    /// between the broadcast and the proof wait.
    #[tokio::test]
    async fn broadcast_half_leaves_broadcast_row_and_flushed_pool() {
        let persistence = Arc::new(CapturingPersistence::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysOkBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await
            .expect("broadcast half should succeed");

        {
            let wm = manager.wallet_manager.read().await;
            let (_, info) = wm
                .get_wallet_and_info(&manager.wallet_id)
                .expect("wallet still present");
            let lock = info
                .tracked_asset_locks
                .get(&out_point)
                .expect("broadcast lock must stay tracked");
            assert_eq!(
                lock.status,
                AssetLockStatus::Broadcast,
                "the broadcast half must stop at Broadcast (no proof attached)"
            );
        }
        assert!(
            persistence
                .flushes
                .load(std::sync::atomic::Ordering::SeqCst)
                >= 1,
            "the invitation funding pool must be flushed before broadcast"
        );
    }

    /// An `IdentityInvitation`-typed lock is a shared bearer voucher: the
    /// funding resolver must refuse to consume it through the generic
    /// `FromExistingAssetLock` path (no explicit authorization), and must
    /// let the explicitly-authorized reclaim variant past the gate. Consuming
    /// a voucher generically would both misdirect the funds into an unrelated
    /// local identity and invalidate the invitee's already-shared claim.
    #[tokio::test]
    async fn generic_resume_refuses_invitation_voucher_locks() {
        use crate::wallet::asset_lock::orchestration::AssetLockFunding;

        let persistence = Arc::new(CapturingPersistence::default());
        let (manager, signer) = funded_asset_lock_manager_with_persistence(
            Arc::new(AlwaysOkBroadcaster),
            Arc::clone(&persistence),
        )
        .await;

        // A real tracked invitation voucher, stopped at Broadcast (the
        // broadcast half never attaches a proof).
        let (_path, out_point) = manager
            .broadcast_funded_asset_lock(
                1_000_000,
                0,
                AssetLockFundingType::IdentityInvitation,
                0,
                &signer,
            )
            .await
            .expect("invitation broadcast half succeeds");

        // Unauthorized (generic) consume: refused by the gate, immediately.
        let refused = manager
            .resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::FromExistingAssetLock {
                    out_point,
                    consume_invitation_voucher: false,
                },
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            )
            .await;
        match refused {
            Err(PlatformWalletError::AssetLockTransaction(msg)) => assert!(
                msg.contains("invitation voucher"),
                "expected the voucher-refusal error, got: {msg}"
            ),
            Err(e) => panic!("expected the voucher-refusal error, got {e:?}"),
            Ok(_) => panic!("expected the voucher-refusal error, got Ok(..)"),
        }

        // Authorized (reclaim) consume: passes the gate. The lock has no
        // proof yet, so the resolver proceeds into the proof wait — getting
        // parked there (rather than an immediate refusal) is the positive
        // signal that the gate admitted the call.
        let authorized = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            manager.resolve_funding_with_is_timeout_fallback(
                AssetLockFunding::FromExistingAssetLock {
                    out_point,
                    consume_invitation_voucher: true,
                },
                AssetLockFundingType::IdentityRegistration,
                0,
                &signer,
            ),
        )
        .await;
        match authorized {
            Err(_elapsed) => {} // parked in the proof wait — past the gate
            Ok(Err(PlatformWalletError::AssetLockTransaction(msg)))
                if msg.contains("invitation voucher") =>
            {
                panic!("authorized reclaim consume must pass the voucher gate: {msg}")
            }
            Ok(other) => {
                // Any other outcome also proves the gate admitted the call.
                drop(other);
            }
        }
    }
}
