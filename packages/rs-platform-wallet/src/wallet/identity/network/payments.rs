//! DashPay payment recording and send-to-contact flows.

use dpp::prelude::Identifier;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

use std::sync::Arc;

use tokio::sync::RwLock;

use key_wallet_manager::WalletManager;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

// ---------------------------------------------------------------------------
// Incoming payment recording + reconcile
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Derive missing `Received` [`PaymentEntry`]s from the wallet's
    /// `DashpayReceivingFunds` accounts' UTXO sets.
    ///
    /// Recovery path for incoming-payment history: live detection
    /// ([`record_incoming_dashpay_payments`]) only fires while the app
    /// is running, so payments received before a relaunch (whose UTXOs
    /// are restored from persistence) or during a missed event window
    /// would otherwise never appear in the payment history. Runs as a
    /// local-only step of `dashpay_sync()` — no network round-trips.
    ///
    /// Idempotent: entries are keyed by txid and an existing entry for
    /// a txid (including the owner's own `Sent` record when both
    /// identities live in one wallet) is never overwritten.
    ///
    /// Returns the number of newly recorded entries.
    pub async fn reconcile_incoming_payments(&self) -> Result<usize, PlatformWalletError> {
        use std::collections::BTreeMap;

        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return Ok(0);
        };

        // Sum per (owner, contact, txid) first so the immutable borrow
        // of the account collection ends before the identity-manager
        // mutations below. Multiple outputs of one tx to the same
        // receival account collapse into a single entry.
        let mut totals: BTreeMap<(Identifier, Identifier, String), u64> = BTreeMap::new();
        for (key, account) in &info.core_wallet.accounts.dashpay_receival_accounts {
            for utxo in account.utxos.values() {
                let txid = utxo.outpoint.txid.to_string();
                *totals
                    .entry((
                        Identifier::from(key.user_identity_id),
                        Identifier::from(key.friend_identity_id),
                        txid,
                    ))
                    .or_default() += utxo.txout.value;
            }
        }

        let recorded = record_received_payment_totals(
            info,
            &self.persister,
            totals
                .into_iter()
                .map(|((owner, contact, txid), amount_duffs)| (owner, contact, txid, amount_duffs)),
            "reconcile",
        );
        Ok(recorded)
    }

    /// Re-scan L1 for incoming DashPay payments that landed on a contact's
    /// receival address **before** that address was being watched (DIP-15 §8.7
    /// + §12.6).
    ///
    /// Receival accounts are built lazily — after restore-from-seed, on a second
    /// device, or in the offline-accept→pay window the account appears only once
    /// the contact is established, by which point SPV has already scanned past
    /// the contact's funding height. Those addresses then enter the compact
    /// filter match set **forward-only**, so a payment in an already-scanned
    /// block is silently missed.
    ///
    /// This lowers the wallet's SPV `synced_height` to the minimum
    /// `$coreHeightCreatedAt` across established receival contacts that haven't
    /// been rescanned yet — the filter manager (`dash-spv`) then re-downloads
    /// nothing it already has, re-matches the now-larger script set, and
    /// re-requests the matching blocks. Each contact is recorded in
    /// [`DashPayState::rescan_triggered`](crate::wallet::identity::DashPayState) so the recurring sweep does
    /// not re-lower the height every pass (which would reset the in-flight
    /// backfill and keep it from ever completing). The guard is in-memory, so a
    /// relaunch — where `synced_height` is restored at its high-water — safely
    /// re-triggers an interrupted backfill.
    ///
    /// `synced_height` may regress here: that is safe because it is the
    /// filter-scan checkpoint, decoupled from the monotonic
    /// `last_processed_height`, and every persisted sync cursor is monotonic-max
    /// guarded, so a transient rewind cannot corrupt state or persist a lower
    /// cursor. Only the **receival** account matters — `DashpayExternalAccount`
    /// is outbound and never receives. Returns the floor the height was lowered
    /// to, or `None`.
    pub async fn reconcile_dashpay_rescan(&self) -> Result<Option<u32>, PlatformWalletError> {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return Ok(None);
        };

        let synced_height = info.core_wallet.synced_height();
        // 0 means "scan from genesis / not yet started" — already a full
        // historical scan, nothing to backfill toward.
        if synced_height == 0 {
            return Ok(None);
        }

        // (owner, contact) pairs that have a receival account — we can only
        // watch a contact's incoming addresses once its receival account exists.
        let receival_pairs: Vec<(Identifier, Identifier)> = info
            .core_wallet
            .accounts
            .dashpay_receival_accounts
            .keys()
            .map(|k| {
                (
                    Identifier::from(k.user_identity_id),
                    Identifier::from(k.friend_identity_id),
                )
            })
            .collect();

        // Candidates: established receival contacts not yet rescanned this
        // lifetime whose funding height is below our scan tip. The floor is the
        // minimum funding height — one rewind covers them all (deeper-funded
        // contacts are in the watch set, so the backfill matches them too). The
        // funding height is `min(outgoing, incoming)` of the pair: the channel
        // is payable only once both requests exist, so the earlier of the two is
        // the conservative-correct lower bound.
        let mut floor: Option<u32> = None;
        let mut to_mark: Vec<(Identifier, Identifier)> = Vec::new();
        for (owner, contact) in receival_pairs {
            let Some(managed) = info.identity_manager.managed_identity(&owner) else {
                continue;
            };
            if managed.dashpay().rescan_triggered.contains(&contact) {
                continue;
            }
            let Some(established) = managed.dashpay().established_contacts().get(&contact) else {
                continue;
            };
            let funding = established
                .outgoing_request
                .core_height_created_at
                .min(established.incoming_request.core_height_created_at);
            // Contacts funded below the tip need a backfill — their addresses
            // weren't watched when those blocks were first scanned. Contacts
            // funded at or after the tip are already covered by the ongoing
            // forward scan (their addresses are watched from establishment).
            // EITHER way the contact is now handled, so mark it: once the
            // forward pointer later climbs past a still-forward-covered
            // contact's funding height, the recurring sweep must NOT then
            // rewind to it and redundantly re-scan an already-scanned range.
            if funding < synced_height {
                floor = Some(floor.map_or(funding, |cur| cur.min(funding)));
            }
            to_mark.push((owner, contact));
        }

        if to_mark.is_empty() {
            return Ok(None);
        }

        // Lower the filter-scan checkpoint only when a contact is funded below
        // the tip (otherwise every handled contact is forward-covered and we
        // record the guard without rewinding). The engine clamps `floor` to its
        // own header/birth floor, so no double-clamp here.
        if let Some(floor) = floor {
            info.core_wallet.update_synced_height(floor);
        }
        let triggered = to_mark.len();
        for (owner, contact) in to_mark {
            if let Some(managed) = info.identity_manager.managed_identity_mut(&owner) {
                managed.dashpay_rescan_triggered_mut().insert(contact);
            }
        }
        if let Some(floor) = floor {
            tracing::info!(
                wallet_id = %hex::encode(self.wallet_id),
                floor,
                contacts = triggered,
                "DashPay rescan: lowered SPV synced_height to backfill historical contact payments"
            );
        }
        Ok(floor)
    }

    /// Rebuild missing `Sent` [`PaymentEntry`]s by matching persisted
    /// wallet transaction outputs against the wallet's registered
    /// `DashpayExternalAccount` address pools.
    ///
    /// Recovery path for sent-payment history after restore-from-seed:
    /// the wallet's transaction records survive in persistence but the
    /// local DashPay payment cache may be empty. Unlike the
    /// receival-side UTXO walk this scans persisted tx records, sums
    /// every output that pays a contact's external-account address, and
    /// records a `Sent` entry per `(owner, contact, txid)`.
    ///
    /// Each contact is swept once per distinct state of the persisted
    /// transaction table (see
    /// [`DashPayState::sent_payment_reconcile_swept_table`](crate::wallet::identity::state::managed_identity::dashpay::DashPayState::sent_payment_reconcile_swept_table)):
    /// a full scan certifies the exact enumeration it inspected, and
    /// re-runs only when the table's digest changes. That keeps the
    /// recovery path cheap in steady state — one txid enumeration per
    /// recurring `dashpay_sync()` pass, no record reads — while any new
    /// row (rescan backfill, asynchronous persistence, mempool) makes
    /// the affected wallet's contacts eligible again. Eligibility
    /// deliberately does NOT consult the existing payment map: "the
    /// contact already has a `Sent` entry" proves one write landed, not
    /// that the contact's history is complete — using it as a
    /// completion marker permanently stranded any sibling entry whose
    /// write failed after the first one succeeded. The per-txid dedup
    /// guard below already makes re-sweeping recorded entries a no-op.
    ///
    /// Local-only and idempotent: an existing payment entry under the
    /// txid is never overwritten.
    pub async fn reconcile_sent_payments_from_tx_history(
        &self,
    ) -> Result<usize, PlatformWalletError> {
        use crate::wallet::identity::types::dashpay::payment::PaymentStatus;
        use dashcore::ScriptBuf;
        use key_wallet::managed_account::address_pool::{AddressPool, KeySource};
        use std::collections::{BTreeMap, BTreeSet};

        /// How many gap limits wide to derive before the match-driven walk
        /// starts, so a stretch of unused indices cannot stop it.
        ///
        /// Unused stretches come from sends that consumed an address and then
        /// failed to build; five gap limits (100 addresses at the DIP-15 gap of
        /// 20) covers far more consecutive failures than a contact realistically
        /// accumulates, and the walk still extends past it whenever a match
        /// lands near the frontier.
        const HISTORICAL_SEED_GAP_MULTIPLE: u32 = 5;

        /// The `(owner identity, contact identity)` pair every reconstructed
        /// entry is attributed to.
        type OwnerContact = (Identifier, Identifier);
        /// Script pubkeys derived from the eligible contacts' external
        /// accounts, mapped back to the pair that owns each one.
        ///
        /// Keyed by script pubkey, not by rendered address: the outputs this
        /// matches against come from a consensus-decoded `Transaction`, which
        /// carries scripts. Comparing scripts also sidesteps address-encoding
        /// pitfalls (network prefix, P2PKH vs P2SH rendering).
        type ContactScriptIndex = BTreeMap<ScriptBuf, OwnerContact>;

        /// Per-contact derivation context: a private clone of the contact's
        /// external address pool plus the key source to extend it with, so
        /// the historical range walk below never mutates resident wallet
        /// state and runs outside the wallet-manager lock.
        struct ContactWindow {
            owner: Identifier,
            contact: Identifier,
            pool: AddressPool,
            key_source: KeySource,
        }

        // Pass 1 (cheap, read lock): every external-account contact and the
        // table digest it was last certified against. No pool clones yet —
        // in steady state this pass plus one txid enumeration is the whole
        // sweep.
        let contact_digests: Vec<(Identifier, Identifier, Option<[u8; 32]>)> = {
            let wm = self.wallet_manager.read().await;
            let info = match wm.get_wallet_info(&self.wallet_id) {
                Some(info) => info,
                None => return Ok(0),
            };
            let mut out = Vec::new();
            for key in info.core_wallet.accounts.dashpay_external_accounts.keys() {
                let owner = Identifier::from(key.user_identity_id);
                let contact = Identifier::from(key.friend_identity_id);
                let Some(managed) = info.identity_manager.managed_identity(&owner) else {
                    continue;
                };
                let stored = managed
                    .dashpay()
                    .sent_payment_reconcile_swept_table
                    .get(&contact)
                    .copied();
                out.push((owner, contact, stored));
            }
            out
        };
        if contact_digests.is_empty() {
            return Ok(0);
        }

        let listed = self.persister.list_wallet_core_txids().map_err(|e| {
            PlatformWalletError::Persistence(format!("failed to enumerate wallet txids: {e}"))
        })?;
        // The digest this pass will certify. Computed from exactly the rows
        // enumerated here, so the stamp below can never claim more than this
        // pass inspected: rows that land after this enumeration — a rescan
        // backfill filling the table back in, the wallet-event adapter
        // committing asynchronously behind the in-memory chain height, a
        // mempool transaction with no height advance at all — change the next
        // enumeration's digest and make every stamped contact eligible again.
        let table_digest = wallet_tx_table_digest(&listed);

        let stale: BTreeSet<(Identifier, Identifier)> = contact_digests
            .iter()
            .filter(|(_, _, stored)| *stored != Some(table_digest))
            .map(|(owner, contact, _)| (*owner, *contact))
            .collect();
        if stale.is_empty() {
            // Steady state — every contact was certified against exactly this
            // table. Silent on purpose: this runs on every `dashpay_sync`
            // pass, and logging it would emit a line every 15 seconds for the
            // life of the process.
            return Ok(0);
        }

        // Pass 2 (read lock): derivation context for the stale contacts only —
        // a private clone of each contact's external address pool plus its
        // xpub, so the historical range walk below never mutates resident
        // wallet state and runs outside the wallet-manager lock.
        let mut windows: Vec<ContactWindow> = {
            let wm = self.wallet_manager.read().await;
            let info = match wm.get_wallet_info(&self.wallet_id) {
                Some(info) => info,
                None => return Ok(0),
            };
            // The contact xpubs live on the immutable `Account`s in
            // `wallet.accounts`; the managed collection only holds pool
            // state. Both reads sit under the same read guard.
            let wallet = match wm.get_wallet(&self.wallet_id) {
                Some(wallet) => wallet,
                None => return Ok(0),
            };
            let mut out = Vec::new();
            for (key, account) in &info.core_wallet.accounts.dashpay_external_accounts {
                let owner = Identifier::from(key.user_identity_id);
                let contact = Identifier::from(key.friend_identity_id);
                if !stale.contains(&(owner, contact)) {
                    continue;
                }
                let pools = account.managed_account_type().address_pools();
                let Some(pool) = pools.first() else {
                    continue;
                };
                let key_source = wallet
                    .accounts
                    .dashpay_external_accounts
                    .get(key)
                    .map(|a| KeySource::Public(a.account_xpub))
                    .unwrap_or(KeySource::NoKeySource);
                out.push(ContactWindow {
                    owner,
                    contact,
                    pool: (*pool).clone(),
                    key_source,
                });
            }
            out
        };
        if windows.is_empty() {
            return Ok(0);
        }
        tracing::info!(
            eligible_contacts = windows.len(),
            "reconcile_sent_payments_from_tx_history: candidate contacts selected"
        );

        // Read every wallet-funded record up front. Transactions the wallet
        // did not fund (`spends_wallet_input == false`) can never be sent
        // payments — the host persists them for incoming detection and for
        // third-party transactions that pay a watched contact address, and
        // treating those as ours would fabricate `Sent` history.
        //
        // `incomplete_scan` tracks whether this pass saw the wallet's full
        // funded history. A listed txid that resolves to `Ok(None)` counts as
        // NOT seen: the FFI record path collapses backend failures, missing
        // or undecodable tx bytes, and not-yet-minable InstantSend rows into
        // `None`, so a miss on a txid the host itself enumerated means "not
        // available yet", never "does not exist". Stamping the per-contact
        // completion guard on such a pass would end recovery with records
        // still unread, so the guard stays unstamped and the next sweep
        // retries.
        struct FundedTx {
            txid: String,
            status: PaymentStatus,
            outputs: Vec<(ScriptBuf, u64)>,
        }
        let mut incomplete_scan = false;
        let txid_count = listed.len();
        let mut funded: Vec<FundedTx> = Vec::new();
        for entry in listed {
            if !entry.spends_wallet_input {
                continue;
            }
            let txid = entry.txid;
            match self.persister.get_core_tx_record(&txid) {
                Ok(Some(record)) => {
                    // Walk the decoded transaction's outputs, NOT
                    // `record.output_details`. Records handed back by
                    // `get_core_tx_record` are rebuilt from the host's raw
                    // transaction bytes: `transaction`, `txid` and `context`
                    // are real, every other field is a placeholder —
                    // `output_details` is always an empty vec. Matching on it
                    // silently found nothing.
                    funded.push(FundedTx {
                        txid: txid.to_string(),
                        status: sent_payment_status_for_record(&record),
                        outputs: record
                            .transaction
                            .output
                            .iter()
                            .map(|out| (out.script_pubkey.clone(), out.value))
                            .collect(),
                    });
                }
                Ok(None) => {
                    incomplete_scan = true;
                    tracing::debug!(
                        %txid,
                        "reconcile_sent_payments_from_tx_history: listed tx record unavailable; will retry next sweep"
                    );
                }
                Err(e) => {
                    incomplete_scan = true;
                    tracing::warn!(
                        error = %e,
                        %txid,
                        "reconcile_sent_payments_from_tx_history: tx-record read failed; will retry next sweep"
                    );
                }
            }
        }

        // Extend each contact's pool clone over the historical range before
        // matching. After a restore-from-seed the resident pool holds only
        // the initial gap window (20 addresses at index 0..), while the
        // persisted history can pay indices past it — live sends only derive
        // index N once earlier addresses were marked used, so historical
        // usage always chains within the gap limit. Standard BIP44 recovery:
        // whenever an observed output matches a derived script, keep the
        // window generated through `matched index + gap limit` and rescan
        // until no match lands near the frontier.
        let observed_scripts: BTreeSet<&ScriptBuf> = funded
            .iter()
            .flat_map(|tx| tx.outputs.iter().map(|(script, _)| script))
            .collect();
        let mut address_matches: ContactScriptIndex = BTreeMap::new();
        for window in &mut windows {
            // Seed the walk with a window WIDER than one gap limit, and do it
            // whether or not the pool already materialized addresses.
            //
            // The match-driven loop below only extends past an address it has
            // already seen paid, so it cannot cross a stretch of unused
            // indices. Those stretches are reachable in practice: `send_payment`
            // marks the chosen contact address used before `build_signed`, and a
            // failed build never rolls that back. After enough failures a later
            // successful payment lands past a hole no on-chain output bridges,
            // and on restore the recreated pool stops short of it — the sweep
            // then finds nothing and stamps the contact as swept.
            //
            // Deriving a fixed bounded window first removes the dependency on an
            // earlier match. Cost is one derivation per contact per launch.
            if window.key_source.can_derive() {
                let want = window
                    .pool
                    .gap_limit
                    .saturating_mul(HISTORICAL_SEED_GAP_MULTIPLE);
                let have = window.pool.highest_generated.map_or(0, |i| i + 1);
                if have < want {
                    if let Err(e) =
                        window
                            .pool
                            .generate_addresses(want - have, &window.key_source, true)
                    {
                        incomplete_scan = true;
                        tracing::warn!(
                            error = %e,
                            owner = %window.owner,
                            contact = %window.contact,
                            "reconcile_sent_payments_from_tx_history: seed address derivation failed; will retry next sweep"
                        );
                    }
                }
            }
            loop {
                let highest_matched = window
                    .pool
                    .addresses
                    .values()
                    .filter(|info| observed_scripts.contains(&info.script_pubkey))
                    .map(|info| info.index)
                    .max();
                let Some(highest_matched) = highest_matched else {
                    break;
                };
                let target = highest_matched.saturating_add(window.pool.gap_limit);
                let generated_through = match window.pool.highest_generated {
                    Some(index) if index >= target => break,
                    Some(index) => index,
                    None => break,
                };
                if !window.key_source.can_derive() {
                    // No xpub to extend with — match what the resident pool
                    // already materialized, but do NOT certify completion:
                    // history past the materialized window is unreachable
                    // this pass.
                    incomplete_scan = true;
                    tracing::warn!(
                        owner = %window.owner,
                        contact = %window.contact,
                        "reconcile_sent_payments_from_tx_history: external account has no xpub; historical range walk skipped"
                    );
                    break;
                }
                if let Err(e) = window.pool.generate_addresses(
                    target - generated_through,
                    &window.key_source,
                    true,
                ) {
                    incomplete_scan = true;
                    tracing::warn!(
                        error = %e,
                        owner = %window.owner,
                        contact = %window.contact,
                        "reconcile_sent_payments_from_tx_history: address derivation failed; will retry next sweep"
                    );
                    break;
                }
            }
            for address_info in window.pool.addresses.values() {
                address_matches
                    .entry(address_info.script_pubkey.clone())
                    .or_insert((window.owner, window.contact));
            }
        }

        let mut totals: BTreeMap<(Identifier, Identifier, String), (u64, PaymentStatus)> =
            BTreeMap::new();
        let mut outputs_scanned = 0usize;
        for tx in &funded {
            for (script, value) in &tx.outputs {
                outputs_scanned += 1;
                let Some(&(owner, contact)) = address_matches.get(script) else {
                    continue;
                };
                let entry = totals
                    .entry((owner, contact, tx.txid.clone()))
                    .or_insert((0u64, tx.status));
                entry.0 += value;
                entry.1 = tx.status;
            }
        }

        tracing::info!(
            txids = txid_count,
            funded_records_read = funded.len(),
            candidate_addresses = address_matches.len(),
            outputs_scanned,
            matched_txids = totals.len(),
            "reconcile_sent_payments_from_tx_history: scan complete"
        );

        let mut wm = self.wallet_manager.write().await;
        let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
            return Ok(0);
        };

        let mut recorded = 0usize;
        let mut write_failed_for: BTreeSet<(Identifier, Identifier)> = BTreeSet::new();
        for ((owner, contact, txid), (amount_duffs, status)) in totals {
            let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
                continue;
            };
            if managed.dashpay().payments.contains_key(&txid) {
                continue;
            }
            let mut entry =
                crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_sent(
                    contact,
                    amount_duffs,
                    None,
                );
            entry.status = status;
            tracing::info!(
                owner = %owner,
                contact = %contact,
                %txid,
                amount_duffs,
                ?status,
                "Recording reconstructed sent DashPay payment"
            );
            if let Err(e) = managed.record_dashpay_payment(txid, entry, &self.persister) {
                tracing::warn!(
                    error = %e,
                    "Failed to persist reconstructed sent payment; will retry next sweep"
                );
                write_failed_for.insert((owner, contact));
                continue;
            }
            recorded += 1;
        }

        // Stamp the digest of exactly the enumeration this pass scanned —
        // never a bare "done". A pass is conclusive only for that snapshot of
        // the table; any row that lands afterwards (rescan backfill,
        // asynchronous wallet-event persistence at an unchanged height, a
        // mempool transaction) changes the next enumeration's digest, so the
        // contact is swept again whichever order this sweep and
        // `reconcile_dashpay_rescan` ran in. A table that has not changed
        // costs later passes one enumeration and no record reads.
        //
        // An enumeration that came back empty still proves nothing: after a
        // restore the recurring `dashpay_sync()` can fire before the host has
        // repopulated any of its transaction table, and a zero-txid sweep is
        // indistinguishable from a wallet that genuinely has nothing to
        // reconstruct. Stamping there would end recovery until the digest
        // next changes, so leave the guard unstamped until at least one row
        // exists. The same logic gates on `incomplete_scan`: a pass that
        // could not read every wallet-funded record (or could not derive a
        // contact's historical address range) has not proven anything about
        // the records it missed.
        if !incomplete_scan && txid_count > 0 {
            for window in &windows {
                if write_failed_for.contains(&(window.owner, window.contact)) {
                    continue;
                }
                let Some(managed) = info.identity_manager.managed_identity_mut(&window.owner)
                else {
                    continue;
                };
                managed
                    .dashpay_sent_payment_reconcile_swept_table_mut()
                    .insert(window.contact, table_digest);
            }
        }
        Ok(recorded)
    }

    /// Flip `Pending` `Sent` [`PaymentEntry`]s to `Confirmed` when the
    /// persisted core transaction record reports the transaction final.
    ///
    /// Recovery path for sent-payment confirmation. The live confirm path
    /// ([`confirm_sent_dashpay_payment`](super::confirm_sent_dashpay_payment))
    /// flips a sent payment the moment its block / InstantSend-lock event
    /// arrives, but that is a single live event: if it is missed — a lagged
    /// wallet-event broadcast, or a relaunch after the transaction confirmed
    /// but before the flip was captured — the entry would otherwise stay
    /// `Pending` forever (received payments self-heal from receival-account
    /// UTXOs; sent payments have no such ground truth). This sweep consults
    /// the persisted core tx record (txid + context) and flips any `Pending`
    /// `Sent` entry whose transaction is mined or InstantSend-locked.
    ///
    /// Runs as a local-only step of `dashpay_sync()` — one persister read
    /// per pending sent payment, no network round-trips. Idempotent: a
    /// `Confirmed` entry is left alone, and a transaction not yet final is
    /// retried on the next sweep.
    ///
    /// Returns the number of entries confirmed this pass.
    pub async fn reconcile_sent_payments(&self) -> Result<usize, PlatformWalletError> {
        use crate::wallet::identity::types::dashpay::payment::{PaymentDirection, PaymentStatus};

        // Snapshot the pending sent (owner, txid) pairs under a read lock so
        // the persister reads below don't hold the wallet lock across I/O.
        let pending: Vec<(Identifier, String)> = {
            let wm = self.wallet_manager.read().await;
            let Some(info) = wm.get_wallet_info(&self.wallet_id) else {
                return Ok(0);
            };
            let mut out = Vec::new();
            for owner in info.identity_manager.identity_ids() {
                let Some(managed) = info.identity_manager.managed_identity(&owner) else {
                    continue;
                };
                for (txid, entry) in &managed.dashpay().payments {
                    if entry.direction == PaymentDirection::Sent
                        && entry.status == PaymentStatus::Pending
                    {
                        out.push((owner, txid.clone()));
                    }
                }
            }
            out
        };

        let mut confirmed = 0usize;
        for (_owner, txid_str) in pending {
            let Ok(txid) = txid_str.parse::<dashcore::Txid>() else {
                continue;
            };
            let record = match self.persister.get_core_tx_record(&txid) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        txid = %txid_str,
                        "reconcile_sent_payments: tx-record read failed; will retry next sweep"
                    );
                    continue;
                }
            };
            // An InstantSend lock is final for DashPay display, same as a
            // mined block — one definition of "final", shared with the
            // reconstruction sweep.
            if sent_payment_status_for_record(&record) != PaymentStatus::Confirmed {
                continue;
            }
            // Flip in place via the shared confirm path (re-checks the
            // entry is still a `Pending` `Sent` under its own write lock,
            // so it stays correct if a live event raced this sweep).
            confirm_sent_payment_by_txid(
                &self.wallet_manager,
                &self.wallet_id,
                &self.persister,
                &txid_str,
            )
            .await;
            confirmed += 1;
        }
        Ok(confirmed)
    }
}

/// Record `Received` [`PaymentEntry`]s for a freshly detected Core
/// transaction whose outputs pay DashPay receival-account addresses.
///
/// Live-detection half of incoming-payment recording: called by the
/// wallet-event adapter
/// ([`spawn_wallet_event_adapter`](crate::changeset::core_bridge::spawn_wallet_event_adapter))
/// on every `TransactionDetected` event, so a payment from a contact
/// lands in the receiver's payment history the moment SPV sees the
/// transaction. The recurring [`IdentityWallet::reconcile_incoming_payments`]
/// sweep covers anything this misses (relaunch restore, dropped events).
///
/// Idempotent per txid — re-detections of the same transaction
/// (mempool → in-block → chain-locked) hit the existing-entry guard.
pub(crate) async fn record_incoming_dashpay_payments(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    record: &key_wallet::managed_account::transaction_record::TransactionRecord,
) {
    use key_wallet::managed_account::transaction_record::OutputRole;
    use std::collections::BTreeMap;

    // Candidate outputs: received by us, with a decodable address.
    // Change outputs can't be DashPay-incoming (they pay back to our
    // standard accounts), so only `Received` is considered.
    let candidates: Vec<(dashcore::Address, u64)> = record
        .output_details
        .iter()
        .filter(|d| matches!(d.role, OutputRole::Received))
        .filter_map(|d| Some((d.address.clone()?, d.value)))
        .collect();
    if candidates.is_empty() {
        return;
    }
    let txid = record.txid.to_string();

    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
        return;
    };

    let mut totals: BTreeMap<(Identifier, Identifier), u64> = BTreeMap::new();
    for (address, value) in candidates {
        if let Some(m) =
            DashPayView::<crate::broadcaster::SpvBroadcaster>::match_in_collection(info, &address)
        {
            *totals
                .entry((m.user_identity_id, m.friend_identity_id))
                .or_default() += value;
        }
    }

    record_received_payment_totals(
        info,
        persister,
        totals
            .into_iter()
            .map(|((owner, contact), amount_duffs)| (owner, contact, txid.clone(), amount_duffs)),
        "live",
    );
}

/// Record a `Received` [`PaymentEntry`] for each `(owner, contact, txid,
/// amount_duffs)` total, applying the shared txid-dedup guard so an existing
/// entry for a txid (including the owner's own `Sent` record when both
/// identities live in one wallet) is never overwritten.
///
/// Shared by the live-detection path ([`record_incoming_dashpay_payments`],
/// per-tx totals) and the reconcile sweep
/// ([`IdentityWallet::reconcile_incoming_payments`], per-account totals), so
/// the dedup guard and `PaymentEntry` construction cannot drift between them.
/// A failed persist is logged and skipped — the reconcile sweep re-derives it
/// from UTXOs next pass. `context` labels the log line for the calling path.
/// Returns the number of newly recorded entries.
fn record_received_payment_totals(
    info: &mut PlatformWalletInfo,
    persister: &crate::wallet::persister::WalletPersister,
    totals: impl IntoIterator<Item = (Identifier, Identifier, String, u64)>,
    context: &'static str,
) -> usize {
    let mut recorded = 0usize;
    for (owner, contact, txid, amount_duffs) in totals {
        let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
            continue;
        };
        if managed.dashpay().payments.contains_key(&txid) {
            continue;
        }
        tracing::info!(
            owner = %owner,
            contact = %contact,
            %txid,
            amount_duffs,
            context,
            "Recording incoming DashPay payment"
        );
        // Self-healing path: a failed persist is re-derived from UTXOs
        // on the next reconcile sweep, so log and continue.
        if let Err(e) = managed.record_dashpay_payment(
            txid,
            crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_received(
                contact,
                amount_duffs,
                None,
            ),
            persister,
        ) {
            tracing::warn!(error = %e, context, "Failed to persist incoming payment; will retry next sweep");
        }
        recorded += 1;
    }
    recorded
}

/// Order-independent digest of an enumerated wallet transaction table:
/// SHA-256 over the sorted `(txid, spends_wallet_input)` rows.
///
/// This is what the sent-payment reconstruction sweep stamps per contact —
/// the pass certifies exactly the rows it enumerated, nothing beyond them.
/// The funded flag is part of the digest on purpose: a host correcting a
/// row's wallet-funded attribution changes the table's meaning for the
/// sweep without adding or removing a txid, and must re-trigger it.
/// In-memory only, never persisted — no cross-version stability required.
fn wallet_tx_table_digest(listed: &[crate::changeset::traits::ListedCoreTxid]) -> [u8; 32] {
    use dashcore::hashes::{sha256, Hash, HashEngine};

    let mut rows: Vec<([u8; 32], bool)> = listed
        .iter()
        .map(|entry| (*entry.txid.as_byte_array(), entry.spends_wallet_input))
        .collect();
    rows.sort_unstable();
    let mut engine = sha256::Hash::engine();
    for (txid, funded) in rows {
        engine.input(&txid);
        engine.input(&[funded as u8]);
    }
    sha256::Hash::from_engine(engine).to_byte_array()
}

fn sent_payment_status_for_record(
    record: &key_wallet::managed_account::transaction_record::TransactionRecord,
) -> crate::wallet::identity::types::dashpay::payment::PaymentStatus {
    use crate::wallet::identity::types::dashpay::payment::PaymentStatus;
    use key_wallet::transaction_checking::TransactionContext;

    if record.is_confirmed() || matches!(record.context, TransactionContext::InstantSend(_)) {
        PaymentStatus::Confirmed
    } else {
        PaymentStatus::Pending
    }
}

/// Advance a sender's `Sent` [`PaymentEntry`] from `Pending` to
/// `Confirmed` once its broadcast transaction reaches finality.
///
/// [`IdentityWallet::send_payment`] records the outgoing entry as
/// `Pending` at broadcast time and nothing else advances it. The wallet
/// re-emits the sender's own transaction as it moves through mempool →
/// InstantSend → in-block → chain-locked, so when a re-detection reports
/// the transaction final the matching entry is flipped in place.
///
/// An **InstantSend lock counts as final** for DashPay display: it is
/// effectively irreversible, so the user sees `Confirmed` without waiting
/// for the surrounding block. A bare mempool re-detection (no IS lock, not
/// yet mined) leaves the entry `Pending` — which it genuinely still is.
/// Idempotent: once `Confirmed`, later re-detections find nothing to
/// change and skip the persistence round.
pub(crate) async fn confirm_sent_dashpay_payment(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    record: &key_wallet::managed_account::transaction_record::TransactionRecord,
) {
    use key_wallet::transaction_checking::TransactionContext;
    // Mined (InBlock / InChainLockedBlock) OR InstantSend-locked advances
    // the entry. A plain mempool sighting does not.
    let is_instant_send = matches!(record.context, TransactionContext::InstantSend(_));
    if !record.is_confirmed() && !is_instant_send {
        return;
    }
    confirm_sent_payment_by_txid(
        wallet_manager,
        wallet_id,
        persister,
        &record.txid.to_string(),
    )
    .await;
}

/// Confirm a sender's `Sent` [`PaymentEntry`] by txid alone, for a
/// [`WalletEvent::TransactionInstantLocked`](key_wallet_manager::WalletEvent::TransactionInstantLocked)
/// that applies an InstantSend lock to a previously-seen transaction.
/// That event carries no [`TransactionRecord`](key_wallet::managed_account::transaction_record::TransactionRecord),
/// only the txid; an IS lock is treated as final for DashPay display, so
/// this flips a matching `Pending` `Sent` entry to `Confirmed`. Idempotent
/// (the underlying flip skips entries already past `Pending`).
pub(crate) async fn confirm_sent_dashpay_payment_by_txid(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &dashcore::Txid,
) {
    confirm_sent_payment_by_txid(wallet_manager, wallet_id, persister, &txid.to_string()).await;
}

/// Flip the `Pending` `Sent` [`PaymentEntry`] under `txid` (if any) to
/// `Confirmed`, in place, preserving amount/memo/counterparty.
///
/// No-op when no entry exists for `txid`, it is not a `Sent` entry, or it
/// is already past `Pending` (so repeated confirmed re-detections are
/// idempotent and skip the persistence round). Separated from the event
/// glue above so the state transition is unit-testable without
/// constructing a full `TransactionRecord`.
async fn confirm_sent_payment_by_txid(
    wallet_manager: &Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    wallet_id: &WalletId,
    persister: &crate::wallet::persister::WalletPersister,
    txid: &str,
) {
    use crate::wallet::identity::types::dashpay::payment::{PaymentDirection, PaymentStatus};

    let mut wm = wallet_manager.write().await;
    let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
        return;
    };

    // The sent transaction belongs to one managed identity; find the
    // `Pending` `Sent` entry under this txid and confirm it in place.
    for owner in info.identity_manager.identity_ids() {
        let Some(managed) = info.identity_manager.managed_identity_mut(&owner) else {
            continue;
        };
        let confirmed = match managed.dashpay().payments.get(txid) {
            Some(entry)
                if entry.direction == PaymentDirection::Sent
                    && entry.status == PaymentStatus::Pending =>
            {
                let mut updated = entry.clone();
                updated.status = PaymentStatus::Confirmed;
                updated
            }
            _ => continue,
        };
        tracing::info!(owner = %owner, %txid, "Confirming sent DashPay payment");
        if let Err(e) = managed.record_dashpay_payment(txid.to_string(), confirmed, persister) {
            tracing::warn!(
                error = %e,
                "Failed to persist sent-payment confirmation; will retry on next detection"
            );
        }
        // txid is unique — only one identity can hold this entry.
        break;
    }
}

// ---------------------------------------------------------------------------
// Send payment to contact
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayView<'_, B> {
    /// Send a Core payment to a DashPay contact.
    ///
    /// Derives the next payment address from the contact's `DashpayExternalAccount`
    /// address pool, builds and broadcasts the transaction via the injected
    /// broadcaster, and records the [`PaymentEntry`] on the sender's
    /// [`ManagedIdentity`].
    ///
    /// # Prerequisite
    ///
    /// `register_external_contact_account` must have been called first so the
    /// watch-only account (and hence its address pool) is available in the
    /// wallet manager. Returns [`PlatformWalletError::InvalidIdentityData`] if
    /// no external account exists for this contact pair.
    ///
    /// # Arguments
    ///
    /// * `from_identity_id` - Our identity that is sending the payment.
    /// * `to_contact_id`    - The contact's identity.
    /// * `amount_duffs`     - Amount to send in duffs (1 DASH = 1e8 duffs).
    /// * `memo`             - Optional free-text memo to attach to the entry.
    /// * `signer`           - Keychain-backed [`key_wallet::signer::Signer`]
    ///   that produces each funding input's ECDSA signature on demand. The
    ///   wallet seed is never made resident — every signature is derived and
    ///   wiped inside the signer (mirrors `core_wallet::send_to_addresses`).
    /// * `provider`         - [`ContactCryptoProvider`] used to drain any
    ///   deferred contact-crypto build for this contact before the send. The
    ///   original sender's external-account build is enqueued by the signerless
    ///   sweep and completed only by a later `drain_pending_contact_crypto`;
    ///   draining here (with a signer present) builds the account on demand so
    ///   the very first send after establishing a contact succeeds instead of
    ///   failing the external-account lookup below.
    ///
    /// # Returns
    ///
    /// The `Txid` of the broadcast transaction, the newly created
    /// [`PaymentEntry`] recording the outgoing payment, and the exact
    /// network fee in duffs (inputs − outputs) of the broadcast
    /// transaction.
    pub async fn send_payment<S, C>(
        &self,
        from_identity_id: &Identifier,
        to_contact_id: &Identifier,
        amount_duffs: u64,
        memo: Option<String>,
        signer: &S,
        provider: &C,
    ) -> Result<
        (
            dashcore::Txid,
            crate::wallet::identity::types::dashpay::payment::PaymentEntry,
            u64, // network fee in duffs, from the broadcast transaction
        ),
        PlatformWalletError,
    >
    where
        S: key_wallet::signer::Signer,
        C: crate::wallet::identity::network::contact_requests::ContactCryptoProvider + Sync,
    {
        use key_wallet::account::account_collection::DashpayAccountKey;
        use key_wallet::wallet::managed_wallet_info::coin_selection::SelectionStrategy;
        use key_wallet::wallet::managed_wallet_info::transaction_builder::TransactionBuilder;
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let account_index: u32 = 0;

        // Complete any deferred contact-crypto build before resolving the
        // external account. The signerless sweep enqueues the original
        // sender's `RegisterExternal` op and never builds it inline; with a
        // signer present here we drain first so the account exists for the
        // lookup below. Idempotent and a cheap no-op when the queue is empty.
        // Run BEFORE acquiring the wallet-manager write guard — the drain
        // re-acquires that (non-reentrant) lock internally.
        self.drain_pending_contact_crypto(provider).await;

        let (payment_address, used_flip_changeset, tx, fee) = {
            let mut wm = self.wallet_manager.write().await;

            // Resolve the external account's xpub so we can derive addresses.
            let contact_xpub = {
                // Look up the external account in the *immutable* AccountCollection on
                // `Wallet`. The ManagedAccountCollection only stores the managed state;
                // the xpub lives on the immutable Account in `wallet.accounts`.
                // For a watch-only external account we stored the contact's xpub directly
                // as `account_xpub` on the Account struct — look it up via DashpayAccountKey.
                let wallet = wm.get_wallet(&self.wallet_id).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id))
                })?;
                wallet
                    .accounts
                    .dashpay_external_accounts
                    .get(&DashpayAccountKey {
                        index: account_index,
                        user_identity_id: from_identity_id.to_buffer(),
                        friend_identity_id: to_contact_id.to_buffer(),
                    })
                    .map(|a| a.account_xpub)
                    .ok_or_else(|| {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "No DashpayExternalAccount found for contact {} — call \
                             register_external_contact_account first",
                            to_contact_id
                        ))
                    })?
            };

            let (wallet, info) = wm
                .get_wallet_and_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;

            // Derive the next unused address from the external account's address pool.
            let key = DashpayAccountKey {
                index: account_index,
                user_identity_id: from_identity_id.to_buffer(),
                friend_identity_id: to_contact_id.to_buffer(),
            };
            let external_account = info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .get_mut(&key)
                .ok_or_else(|| {
                    PlatformWalletError::InvalidIdentityData(format!(
                        "No managed DashpayExternalAccount found for contact {}",
                        to_contact_id
                    ))
                })?;

            let payment_address = external_account
                .next_address(Some(&contact_xpub), true)
                .map_err(|e| PlatformWalletError::TransactionBuild(e.to_string()))?;

            // `next_address`/`next_unused` only *selects* (and, when the
            // gap-window is exhausted, generates) the next unused address —
            // it does NOT flip its `used` flag (that lives solely on
            // `AddressPool::mark_used`). DIP-15 per-payment rotation requires
            // that once we commit this address to a payment it never be
            // handed out again, so mark it used now, on the resident external
            // account, before the snapshot below captures the pool. A
            // `false` return would mean the address vanished from the pool
            // between derivation and this call — a real invariant break, so
            // fail loud rather than silently ship an un-rotated address.
            if !external_account.mark_address_used(&payment_address) {
                return Err(PlatformWalletError::TransactionBuild(format!(
                    "derived payment address {payment_address} is not in the external \
                     account pool — cannot mark it used"
                )));
            }

            // Snapshot the used-flag flip (and any gap-window extension)
            // `next_address` + `mark_address_used` just applied to the
            // external account's pool, as an owned changeset. The snapshot is
            // captured here — while the pool is still borrowed under the
            // guard — so the persisted state is exactly the flip just made,
            // but the `persister.store` call itself is deferred until after
            // this write guard is released (see below, before the broadcast).
            // The host persistence callback must not run while the
            // wallet-manager write lock is held: a slow host write would stall
            // every other wallet accessor for its duration, and a host store
            // that re-entered any manager API would deadlock the non-reentrant
            // lock.
            let external_account_type = key_wallet::account::AccountType::DashpayExternalAccount {
                index: account_index,
                user_identity_id: from_identity_id.to_buffer(),
                friend_identity_id: to_contact_id.to_buffer(),
            };
            let used_flip_changeset = crate::changeset::PlatformWalletChangeSet {
                account_address_pools: crate::changeset::account_address_pool_entries(
                    external_account_type,
                    external_account.managed_account_type().address_pools(),
                ),
                ..Default::default()
            };

            let current_height = info.core_wallet.synced_height();

            let managed_account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 managed account 0 not found".to_string(),
                    )
                })?;
            let account = wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .ok_or_else(|| {
                    PlatformWalletError::TransactionBuild(
                        "BIP-44 account 0 not found in wallet".to_string(),
                    )
                })?;

            let builder = TransactionBuilder::new()
                .set_current_height(current_height)
                .set_selection_strategy(SelectionStrategy::LargestFirst)
                .set_funding(managed_account, account)
                .add_output(&payment_address, amount_duffs);

            // Sign through the injected signer (blanket
            // `impl<S: Signer> TransactionSigner for S`) rather than the
            // resident `wallet`, so funding-input signatures are produced
            // from Keychain-derived keys without a resident seed.
            // `build_signed` returns the fee the transaction actually
            // pays — Σ(selected input values) − Σ(output values), a
            // dropped sub-dust change remainder included — since
            // rust-dashcore#872 (pinned above). No caller-side
            // recomputation needed.
            let (tx, fee) = match builder
                .build_signed(signer, |addr| {
                    managed_account.address_derivation_path(&addr)
                })
                .await
            {
                Ok(built) => built,
                Err(e) => {
                    // Return the consumed address to the pool. Nothing was
                    // signed to completion, persisted (the used-flip store
                    // below is unreachable from here) or broadcast, so the
                    // mark exists only in this process's memory and the
                    // address was never exposed on-chain — un-marking it
                    // cannot break DIP-15 rotation. Leaving it consumed
                    // would let every failed build (insufficient funds
                    // retried by the user, a signer refusing) advance the
                    // next index by one with no bound; enough failures
                    // before one successful payment would put that payment
                    // beyond any gap-limit walk a restore-from-seed can
                    // perform, permanently hiding it from sent-payment
                    // reconstruction. Used indices must chain within the
                    // gap limit, so consumption is committed only once a
                    // fully signed transaction exists.
                    if let Some(external_account) = info
                        .core_wallet
                        .accounts
                        .dashpay_external_accounts
                        .get_mut(&key)
                    {
                        for pool in external_account
                            .managed_account_type_mut()
                            .address_pools_mut()
                        {
                            let Some(&index) = pool.address_index.get(&payment_address) else {
                                continue;
                            };
                            pool.used_indices.remove(&index);
                            if let Some(address_info) = pool.addresses.get_mut(&index) {
                                address_info.state =
                                    key_wallet::managed_account::address_pool::AddressState::Available;
                            }
                            pool.highest_used = pool.used_indices.iter().max().copied();
                        }
                    }
                    return Err(PlatformWalletError::TransactionBuild(e.to_string()));
                }
            };

            (payment_address, used_flip_changeset, tx, fee)
        };

        // Persist the payment-address used flip now that the wallet-manager
        // write guard is released, but BEFORE the broadcast below. Durability
        // must precede broadcast: once the transaction is on the network the
        // address is publicly spent-to, and a relaunch that rehydrated the
        // pool from a stale snapshot would re-hand the SAME address to the
        // next payment — breaking DIP-15 per-payment rotation and linking the
        // payments on-chain. A store failure aborts the send pre-broadcast
        // (nothing has hit the network); the consumed in-memory address only
        // leaves a one-address gap that the pool's gap window absorbs on
        // retry — bounded, because a signed transaction exists here, unlike
        // the unbounded build-failure case rolled back above.
        self.persister.store(used_flip_changeset).map_err(|e| {
            PlatformWalletError::Persistence(format!(
                "failed to persist payment-address used flip: {e}"
            ))
        })?;

        // --- 3. Broadcast the transaction, releasing the build's UTXO
        // reservation if the broadcast is definitively rejected pre-send. ---
        let txid = crate::wallet::reservations::broadcast_releasing_on_rejection(
            self.broadcaster.as_ref(),
            &self.wallet_manager,
            &self.wallet_id,
            key_wallet::account::account_type::StandardAccountType::BIP44Account,
            0,
            &tx,
        )
        .await?;

        tracing::info!(
            from_identity = %from_identity_id,
            to_contact = %to_contact_id,
            amount_duffs,
            %txid,
            payment_address = %payment_address,
            "DashPay payment broadcast"
        );

        // --- 4. Record the outgoing payment on the sender's ManagedIdentity. ---
        let entry = crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_sent(
            *to_contact_id,
            amount_duffs,
            memo,
        );
        {
            let mut wm = self.wallet_manager.write().await;
            // A missing wallet info or unmanaged `from_identity_id` is a real
            // state error, not a benign no-op: the tx is already broadcast
            // on-chain but the local Sent entry + memo has no on-chain
            // recovery, so swallowing the lookup miss here would lose the
            // user's payment record with no signal. Fail loud (same rationale
            // as the persist-failure propagation just below) so the UI can
            // report the partial outcome (sent, but not recorded).
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let managed = info
                .identity_manager
                .managed_identity_mut(from_identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*from_identity_id))?;
            managed
                .record_dashpay_payment(txid.to_string(), entry.clone(), &self.persister)
                .map_err(|e| {
                    PlatformWalletError::Persistence(format!(
                        "payment broadcast but not recorded locally: {e}"
                    ))
                })?;
        }

        Ok((txid, entry, fee))
    }
}

#[cfg(test)]
mod tests {
    //! Receiver-side payment persistence tests.
    //!
    //! These pin the three pieces that make incoming DashPay payments
    //! survive across app relaunches (without them, a recipient's received
    //! payments show "Payments (0)"):
    //!
    //! 1. `register_contact_account` must PERSIST the account
    //!    registration, so the `DashpayReceivingFunds` account is
    //!    rebuilt at next load and its persisted UTXOs route instead
    //!    of being dropped (`dropped_no_account`).
    //! 2. `reconcile_incoming_payments` must derive missing
    //!    `Received` PaymentEntries from the receival accounts' UTXO
    //!    sets (recovers history after restore and any missed live
    //!    events).
    //! 3. Reconcile must be idempotent and never clobber an existing
    //!    entry for the same txid.

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    /// Persister that records every store round so tests can assert on
    /// exactly what would reach the host (SwiftData) for a given flow.
    #[derive(Default)]
    struct RecordingPersister {
        stores: Mutex<Vec<(WalletId, PlatformWalletChangeSet)>>,
    }

    impl PlatformWalletPersistence for RecordingPersister {
        fn store(
            &self,
            wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stores.lock().unwrap().push((wallet_id, changeset));
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Persister that answers `get_core_tx_record` from a configurable
    /// in-memory map, so a test can stage the persisted core transaction
    /// state the sent-payment reconcile reads. `store`/`flush` are no-ops
    /// (unless a store-failure budget is armed); `load` returns the
    /// default state.
    #[derive(Default)]
    struct RecordStorePersister {
        records: Mutex<
            std::collections::BTreeMap<
                dashcore::Txid,
                key_wallet::managed_account::transaction_record::TransactionRecord,
            >,
        >,
        /// Txids the enumeration lists but `get_core_tx_record` answers
        /// `Ok(None)` for — the FFI shape for "row exists, record not
        /// available yet" (missing bytes, undecodable, pending InstantSend).
        listed_but_unavailable: Mutex<std::collections::BTreeSet<dashcore::Txid>>,
        /// Txids the enumeration reports as NOT wallet-funded
        /// (`spends_wallet_input == false`).
        not_wallet_funded: Mutex<std::collections::BTreeSet<dashcore::Txid>>,
        /// `Some(n)` lets the next `n` `store` calls succeed and fails every
        /// later one until the budget is disarmed (`None` = always succeed).
        allow_stores_then_fail: Mutex<Option<usize>>,
        list_wallet_core_txids_calls: Mutex<usize>,
        get_core_tx_record_calls: Mutex<usize>,
    }

    impl PlatformWalletPersistence for RecordStorePersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            let mut budget = self.allow_stores_then_fail.lock().unwrap();
            match budget.as_mut() {
                Some(0) => Err(PersistenceError::backend("injected store failure")),
                Some(remaining) => {
                    *remaining -= 1;
                    Ok(())
                }
                None => Ok(()),
            }
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
            txid: &dashcore::Txid,
        ) -> Result<
            Option<key_wallet::managed_account::transaction_record::TransactionRecord>,
            PersistenceError,
        > {
            *self.get_core_tx_record_calls.lock().unwrap() += 1;
            if self.listed_but_unavailable.lock().unwrap().contains(txid) {
                return Ok(None);
            }
            Ok(self.records.lock().unwrap().get(txid).cloned())
        }

        fn list_wallet_core_txids(
            &self,
            _wallet_id: WalletId,
        ) -> Result<Vec<crate::changeset::traits::ListedCoreTxid>, PersistenceError> {
            *self.list_wallet_core_txids_calls.lock().unwrap() += 1;
            let not_funded = self.not_wallet_funded.lock().unwrap();
            let unavailable = self.listed_but_unavailable.lock().unwrap();
            let listed: std::collections::BTreeSet<dashcore::Txid> = self
                .records
                .lock()
                .unwrap()
                .keys()
                .copied()
                .chain(unavailable.iter().copied())
                .collect();
            Ok(listed
                .into_iter()
                .map(|txid| crate::changeset::traits::ListedCoreTxid {
                    txid,
                    spends_wallet_input: !not_funded.contains(&txid),
                })
                .collect())
        }
    }

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    /// Seed-backed [`key_wallet::signer::Signer`] for the send-path tests.
    /// Derives every key from the test seed, so it is a faithful stand-in for
    /// the Keychain signer — but the send-payment tests fail before any input
    /// is signed (no external account / no spendable UTXOs), so in practice
    /// only the type bound matters here.
    struct SeedSigner {
        wallet: key_wallet::wallet::Wallet,
    }

    impl SeedSigner {
        fn new(seed: [u8; 64], network: Network) -> Self {
            Self {
                wallet: key_wallet::wallet::Wallet::from_seed_bytes(
                    seed,
                    network,
                    WalletAccountCreationOptions::None,
                )
                .expect("seed wallet"),
            }
        }
    }

    #[async_trait::async_trait]
    impl key_wallet::signer::Signer for SeedSigner {
        type Error = String;

        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            &[key_wallet::signer::SignerMethod::Digest]
        }

        async fn sign_ecdsa(
            &self,
            path: &key_wallet::bip32::DerivationPath,
            sighash: [u8; 32],
        ) -> Result<
            (
                dashcore::secp256k1::ecdsa::Signature,
                dashcore::secp256k1::PublicKey,
            ),
            String,
        > {
            let xprv = self
                .wallet
                .derive_extended_private_key(path)
                .map_err(|e| e.to_string())?;
            let secp = dashcore::secp256k1::Secp256k1::new();
            let msg = dashcore::secp256k1::Message::from_digest(sighash);
            let sig = secp.sign_ecdsa(&msg, &xprv.private_key);
            let pk = dashcore::secp256k1::PublicKey::from_secret_key(&secp, &xprv.private_key);
            Ok((sig, pk))
        }

        async fn public_key(
            &self,
            path: &key_wallet::bip32::DerivationPath,
        ) -> Result<dashcore::secp256k1::PublicKey, String> {
            let xprv = self
                .wallet
                .derive_extended_private_key(path)
                .map_err(|e| e.to_string())?;
            let secp = dashcore::secp256k1::Secp256k1::new();
            Ok(dashcore::secp256k1::PublicKey::from_secret_key(
                &secp,
                &xprv.private_key,
            ))
        }
    }

    #[async_trait::async_trait]
    impl key_wallet::signer::ExtendedPubKeySigner for SeedSigner {
        async fn extended_public_key(
            &self,
            path: &key_wallet::bip32::DerivationPath,
        ) -> Result<key_wallet::bip32::ExtendedPubKey, String> {
            self.wallet
                .derive_extended_public_key(path)
                .map_err(|e| e.to_string())
        }
    }

    /// Build a testnet wallet backed by an arbitrary persister `P`, for
    /// flows that need a persister beyond [`RecordingPersister`] (e.g. the
    /// sent-payment reconcile, which reads `get_core_tx_record`).
    async fn make_wallet_with<P: PlatformWalletPersistence + 'static>(
        persister: Arc<P>,
    ) -> (Arc<PlatformWalletManager<P>>, WalletId) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        // Wallet stays external-signable (no resident seed) — the production
        // posture: signing runs through the host signer, never a grafted seed.
        // Tests that need private-key ops derive via a Wallet-from-seed helper
        // (test_receiving_xpub) or a SeedCryptoProvider from the same mnemonic.
        (manager, wallet_id)
    }

    async fn make_wallet() -> (
        Arc<PlatformWalletManager<RecordingPersister>>,
        Arc<RecordingPersister>,
        WalletId,
    ) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(RecordingPersister::default());
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        // Wallet stays external-signable (no resident seed) — the production
        // posture: signing runs through the host signer, never a grafted seed.
        // Tests that need private-key ops derive via a Wallet-from-seed helper
        // (test_receiving_xpub) or a SeedCryptoProvider from the same mnemonic.
        (manager, persister, wallet_id)
    }

    /// Like [`make_wallet`], leaving the wallet external-signable
    /// (`has_seed() == false`) — the watch-only / seedless state the
    /// unattended sync sweep can hit before a Keychain unlock.
    async fn make_watch_only_wallet() -> (
        Arc<PlatformWalletManager<RecordingPersister>>,
        Arc<RecordingPersister>,
        WalletId,
    ) {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(RecordingPersister::default());
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();
        // Intentionally seedless: creation downgrades to external-signable, so
        // the wallet has no resident key material.
        (manager, persister, wallet_id)
    }

    /// The DashPay receiving (friendship) xpub for `(owner, contact)` at account
    /// 0, derived from `TEST_MNEMONIC` via a standalone `Wallet` — the same xpub
    /// the Keychain signer produces in production. Lets seedless-wallet tests
    /// supply `register_contact_account`'s now-mandatory xpub without a resident
    /// wallet.
    fn test_receiving_xpub(
        owner: &Identifier,
        contact: &Identifier,
    ) -> key_wallet::bip32::ExtendedPubKey {
        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let wallet = key_wallet::wallet::Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            WalletAccountCreationOptions::None,
        )
        .expect("seed wallet");
        crate::wallet::identity::crypto::dip14::derive_contact_xpub(
            &wallet,
            Network::Testnet,
            0,
            owner,
            contact,
        )
        .expect("derive receiving xpub")
        .xpub
    }

    fn bare_identity(id_bytes: [u8; 32]) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from(id_bytes),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    /// Insert a fake UTXO into the (owner, contact) receival account,
    /// paying `value_duffs` to the account's first pool address, and
    /// return the txid hex string used.
    async fn plant_receival_utxo(
        manager: &Arc<PlatformWalletManager<RecordingPersister>>,
        wallet_id: WalletId,
        owner: Identifier,
        contact: Identifier,
        txid_byte: u8,
        value_duffs: u64,
    ) -> String {
        use dashcore::hashes::Hash;
        let wallet = manager
            .get_wallet(&wallet_id)
            .await
            .expect("wallet registered");
        let iw = wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        let account = info
            .core_wallet
            .accounts
            .dashpay_receival_accounts
            .get_mut(&key)
            .expect("receival account registered");
        let address_info = {
            use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
            account
                .managed_account_type()
                .address_pools()
                .first()
                .expect("receival account has a pool")
                .addresses
                .values()
                .next()
                .expect("pool has at least one derived address")
                .clone()
        };
        let txid = dashcore::Txid::from_slice(&[txid_byte; 32]).expect("txid");
        let outpoint = dashcore::OutPoint { txid, vout: 0 };
        account.utxos.insert(
            outpoint,
            key_wallet::Utxo {
                outpoint,
                txout: dashcore::TxOut {
                    value: value_duffs,
                    script_pubkey: address_info.script_pubkey.clone(),
                },
                address: address_info.address.clone(),
                height: 100,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            },
        );
        txid.to_string()
    }

    async fn install_external_account(
        manager: &Arc<PlatformWalletManager<RecordStorePersister>>,
        wallet_id: WalletId,
        owner: Identifier,
        contact: Identifier,
    ) -> Vec<dashcore::Address> {
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::ManagedCoreFundsAccount;

        let wallet = manager
            .get_wallet(&wallet_id)
            .await
            .expect("wallet registered");
        let iw = wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        let (wallet, info) = wm
            .get_wallet_mut_and_info_mut(&wallet_id)
            .expect("wallet and info");

        let account_type = AccountType::DashpayExternalAccount {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        let account_xpub = test_receiving_xpub(&owner, &contact);
        let account = key_wallet::Account {
            parent_wallet_id: Some(wallet_id),
            account_type,
            network: Network::Testnet,
            account_xpub,
            is_watch_only: true,
        };
        let managed = ManagedCoreFundsAccount::from_account(&account);

        wallet
            .add_account(account_type, Some(account_xpub))
            .expect("add immutable external account");
        info.core_wallet
            .accounts
            .insert_funds_bearing_account(managed)
            .expect("add managed external account");

        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        let account = info
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("external account present");
        account
            .managed_account_type()
            .address_pools()
            .first()
            .expect("external account has a pool")
            .addresses
            .values()
            .take(2)
            .map(|info| info.address.clone())
            .collect()
    }

    async fn first_standard_wallet_address(
        manager: &Arc<PlatformWalletManager<RecordStorePersister>>,
        wallet_id: WalletId,
    ) -> dashcore::Address {
        let wallet = manager
            .get_wallet(&wallet_id)
            .await
            .expect("wallet registered");
        let iw = wallet.identity();
        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("wallet info");
        info.core_wallet
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("bip44 account 0")
            .managed_account_type()
            .address_pools()
            .first()
            .expect("standard external pool")
            .addresses
            .values()
            .next()
            .expect("at least one standard address")
            .address
            .clone()
    }

    fn tx_record_with_outputs(
        context: key_wallet::transaction_checking::TransactionContext,
        outputs: Vec<(
            dashcore::Address,
            u64,
            key_wallet::managed_account::transaction_record::OutputRole,
        )>,
    ) -> key_wallet::managed_account::transaction_record::TransactionRecord {
        use dashcore::{OutPoint, Transaction, TxIn, TxOut, Txid};
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::{
            OutputDetail, TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::TransactionType;

        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint::new(Txid::from([0x91; 32]), 0),
                ..Default::default()
            }],
            output: outputs
                .iter()
                .map(|(address, value, _)| TxOut {
                    value: *value,
                    script_pubkey: address.script_pubkey(),
                })
                .collect(),
            special_transaction_payload: None,
        };
        let output_details = outputs
            .into_iter()
            .enumerate()
            .map(|(index, (address, value, role))| OutputDetail {
                index: index as u32,
                role,
                address: Some(address),
                value,
            })
            .collect();
        TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            output_details,
            0,
        )
    }

    /// 1. Registering a contact receival account must persist an
    /// `AccountRegistrationEntry` — otherwise the account (and every
    /// UTXO routed to it) silently vanishes on the next app launch
    /// (`load: ... dropped_no_account`).
    #[tokio::test]
    async fn register_contact_account_persists_account_registration() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        persister.stores.lock().unwrap().clear();

        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            wallet
                .identity()
                .dashpay()
                .register_contact_account(
                    &owner,
                    &contact,
                    0,
                    test_receiving_xpub(&owner, &contact),
                )
                .await
                .expect("register_contact_account");
        }

        {
            let stores = persister.stores.lock().unwrap();
            let registered = stores.iter().any(|(_, cs)| {
                cs.account_registrations.iter().any(|entry| {
                    matches!(
                        entry.account_type,
                        key_wallet::account::AccountType::DashpayReceivingFunds {
                            user_identity_id,
                            friend_identity_id,
                            ..
                        } if user_identity_id == owner.to_buffer()
                            && friend_identity_id == contact.to_buffer()
                    )
                })
            });
            assert!(
                registered,
                "register_contact_account must emit an AccountRegistrationEntry \
                 so the DashpayReceivingFunds account survives relaunch"
            );
        }

        // Re-registering must be a no-op (no duplicate persistence round).
        persister.stores.lock().unwrap().clear();
        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            wallet
                .identity()
                .dashpay()
                .register_contact_account(
                    &owner,
                    &contact,
                    0,
                    test_receiving_xpub(&owner, &contact),
                )
                .await
                .expect("re-register is a no-op");
        }
        let stores = persister.stores.lock().unwrap();
        assert!(
            stores
                .iter()
                .all(|(_, cs)| cs.account_registrations.is_empty()),
            "re-registering an existing contact account must not re-persist"
        );
    }

    /// 2. Reconcile derives `Received` entries from receival-account
    /// UTXOs (restores payment history after relaunch / missed events),
    /// and 3. is idempotent across passes.
    #[tokio::test]
    async fn reconcile_records_received_payments_from_receival_utxos() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            let iw = wallet.identity();
            iw.dashpay()
                .register_contact_account(
                    &owner,
                    &contact,
                    0,
                    test_receiving_xpub(&owner, &contact),
                )
                .await
                .expect("register_contact_account");
            // The owner identity must be managed for the entry to land.
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0xAA; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add managed identity");
        }

        let txid = plant_receival_utxo(&manager, wallet_id, owner, contact, 0x07, 1_000_000).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();

        let recorded = iw
            .dashpay()
            .reconcile_incoming_payments()
            .await
            .expect("reconcile pass");
        assert_eq!(recorded, 1, "one missing Received entry must be recorded");

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let managed = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed identity");
            let entry = managed
                .dashpay()
                .payments
                .get(&txid)
                .expect("Received entry recorded under the UTXO's txid");
            assert_eq!(entry.counterparty_id, contact);
            assert_eq!(entry.amount_duffs, 1_000_000);
            assert_eq!(
                entry.direction,
                super::super::super::types::dashpay::payment::PaymentDirection::Received
            );
        }

        // Idempotency: a second pass records nothing new.
        let recorded_again = iw
            .dashpay()
            .reconcile_incoming_payments()
            .await
            .expect("second reconcile pass");
        assert_eq!(recorded_again, 0, "reconcile must be idempotent");
    }

    /// 3b. An existing entry under the same txid (e.g. the sender's
    /// own `Sent` record when both identities share one wallet) must
    /// not be clobbered by reconcile.
    #[tokio::test]
    async fn reconcile_does_not_clobber_existing_entry_for_same_txid() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            let iw = wallet.identity();
            iw.dashpay()
                .register_contact_account(
                    &owner,
                    &contact,
                    0,
                    test_receiving_xpub(&owner, &contact),
                )
                .await
                .expect("register_contact_account");
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0xAA; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add managed identity");
        }

        let txid = plant_receival_utxo(&manager, wallet_id, owner, contact, 0x09, 500_000).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();

        // Pre-record an entry under the same txid.
        let preexisting = crate::wallet::identity::types::dashpay::payment::PaymentEntry::new_sent(
            contact, 123, None,
        );
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            let managed = info
                .identity_manager
                .managed_identity_mut(&owner)
                .expect("managed identity");
            managed
                .record_dashpay_payment(
                    txid.clone(),
                    preexisting.clone(),
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("record");
        }

        let recorded = iw
            .dashpay()
            .reconcile_incoming_payments()
            .await
            .expect("reconcile pass");
        assert_eq!(recorded, 0, "existing txid entry must be left alone");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        let managed = info
            .identity_manager
            .managed_identity(&owner)
            .expect("managed identity");
        assert_eq!(
            managed.dashpay().payments.get(&txid),
            Some(&preexisting),
            "reconcile must not overwrite the pre-existing entry"
        );
    }

    /// The shared totals-consume helper both incoming paths route through
    /// (`record_incoming_dashpay_payments` live + `reconcile_incoming_payments`
    /// sweep) must apply the txid-dedup guard and construct `Received` entries:
    /// a total for a txid that already has an entry is skipped (never
    /// clobbered), a total for a fresh txid is recorded, and the returned count
    /// is exactly the number newly recorded.
    #[tokio::test]
    async fn record_received_payment_totals_dedups_by_txid() {
        use crate::wallet::identity::types::dashpay::payment::{PaymentDirection, PaymentEntry};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let wp = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let existing_txid = "aa".repeat(32);
        let fresh_txid = "bb".repeat(32);

        let mut wm = iw.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
        info.identity_manager
            .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &wp)
            .expect("add managed identity");

        // Pre-record a Sent entry under existing_txid (the both-identities-in-
        // one-wallet case that must survive an incoming total for the same tx).
        let preexisting = PaymentEntry::new_sent(contact, 123, None);
        info.identity_manager
            .managed_identity_mut(&owner)
            .expect("managed identity")
            .record_dashpay_payment(existing_txid.clone(), preexisting.clone(), &wp)
            .expect("record preexisting");

        let recorded = super::record_received_payment_totals(
            info,
            &wp,
            vec![
                (owner, contact, existing_txid.clone(), 500_000),
                (owner, contact, fresh_txid.clone(), 700_000),
            ],
            "test",
        );
        assert_eq!(recorded, 1, "only the fresh txid should be newly recorded");

        let managed = info
            .identity_manager
            .managed_identity(&owner)
            .expect("managed identity");
        assert_eq!(
            managed.dashpay().payments.get(&existing_txid),
            Some(&preexisting),
            "the existing txid entry must be left untouched (no clobber)"
        );
        let fresh = managed
            .dashpay()
            .payments
            .get(&fresh_txid)
            .expect("fresh entry recorded");
        assert_eq!(fresh.direction, PaymentDirection::Received);
        assert_eq!(fresh.amount_duffs, 700_000);
    }

    /// Persister that succeeds until `armed`, then fails every store —
    /// lets a test build state normally, then prove a later user-initiated
    /// write propagates a persist failure instead of swallowing it.
    #[derive(Default)]
    struct ToggleFailPersister {
        armed: std::sync::atomic::AtomicBool,
    }

    impl PlatformWalletPersistence for ToggleFailPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if self.armed.load(std::sync::atomic::Ordering::SeqCst) {
                Err(PersistenceError::backend("store armed to fail"))
            } else {
                Ok(())
            }
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// **C1 (Critical) — ignore must PROPAGATE a persist failure.**
    /// Ignore is local-only (no on-chain artifact), so a swallowed store
    /// error would resurface the ignored sender on the next launch with no
    /// signal. The user-initiated `ignore` path must return the error
    /// instead.
    ///
    /// The hazard: if `ignore_contact_sender` merely logged the store error
    /// and returned `Ok(())`, the ignore would be lost; it must return
    /// `Err(Persistence)`.
    #[tokio::test]
    async fn ignore_propagates_persist_failure() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let persister = Arc::new(ToggleFailPersister::default());
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::clone(&persister),
            handler,
        ));
        let mnemonic =
            Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
        let seed = mnemonic.to_seed("");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &seed,
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .expect("wallet creation");
        let wallet_id = wallet.wallet_id();

        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        // Setup (persister still succeeding): managed owner + an incoming
        // request to ignore.
        {
            let iw = wallet.identity();
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            let incoming =
                crate::wallet::identity::types::dashpay::contact_request::ContactRequest::new(
                    contact,
                    owner,
                    1,
                    2,
                    0,
                    vec![7u8; 96],
                    100,
                    0,
                );
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .add_incoming_contact_request(incoming, &p)
                .expect("setup persists");
        }

        // Arm the persister to fail, then ignore: must return Err, NOT Ok.
        persister
            .armed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let iw = wallet.identity();
        let result = iw.dashpay().ignore_contact_sender(&owner, &contact).await;
        assert!(
            matches!(result, Err(PlatformWalletError::Persistence(_))),
            "ignore must propagate a persist failure (got {result:?}), \
             else the ignore is lost and the sender resurfaces"
        );
    }

    /// DIP-15 §12.6: when a contact's receival account is registered after SPV
    /// has already scanned past the contact's funding height, the rescan
    /// reconcile lowers `synced_height` to `min(outgoing, incoming)` funding so
    /// the filter manager backfills the missed range — then a per-contact guard
    /// makes it single-shot per lifetime, so the recurring sweep does NOT
    /// re-lower the height and reset the in-flight backfill (which would keep it
    /// from ever completing).
    #[tokio::test]
    async fn rescan_lowers_synced_height_to_funding_floor_then_is_idempotent() {
        use crate::wallet::identity::{ContactRequest, EstablishedContact};
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        iw.dashpay()
            .register_contact_account(&owner, &contact, 0, test_receiving_xpub(&owner, &contact))
            .await
            .expect("register receival account");

        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            // outgoing funded at 200, incoming at 100 -> floor 100.
            let outgoing = ContactRequest::new(owner, contact, 0, 0, 0, vec![0u8; 96], 200, 0);
            let incoming = ContactRequest::new(contact, owner, 0, 0, 0, vec![0u8; 96], 100, 0);
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .apply_established_contact(EstablishedContact::new(contact, outgoing, incoming));
            // Simulate a forward sync to height 1000.
            info.core_wallet.update_synced_height(1000);
        }

        assert_eq!(
            iw.dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan"),
            Some(100),
            "first pass lowers to min(outgoing, incoming) funding height"
        );
        {
            let wm = iw.wallet_manager.read().await;
            assert_eq!(
                wm.get_wallet_info(&wallet_id)
                    .unwrap()
                    .core_wallet
                    .synced_height(),
                100,
                "synced_height lowered to the floor"
            );
        }

        assert_eq!(
            iw.dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan 2"),
            None,
            "already-rescanned contact must not re-trigger (no backfill thrash)"
        );
        {
            let wm = iw.wallet_manager.read().await;
            assert_eq!(
                wm.get_wallet_info(&wallet_id)
                    .unwrap()
                    .core_wallet
                    .synced_height(),
                100,
                "height stays at the floor after the no-op pass"
            );
        }
    }

    /// Register a receival account for `(owner, contact)` and insert an
    /// established contact funded at `out_height`/`in_height`. The owner managed
    /// identity is added on first use.
    async fn establish_receival_contact(
        manager: &Arc<PlatformWalletManager<RecordingPersister>>,
        persister: &Arc<RecordingPersister>,
        wallet_id: WalletId,
        owner: Identifier,
        contact: Identifier,
        out_height: u32,
        in_height: u32,
    ) {
        use crate::wallet::identity::{ContactRequest, EstablishedContact};
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(persister) as _);
        iw.dashpay()
            .register_contact_account(&owner, &contact, 0, test_receiving_xpub(&owner, &contact))
            .await
            .expect("register receival account");
        let mut wm = iw.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
        if info.identity_manager.managed_identity(&owner).is_none() {
            info.identity_manager
                .add_identity(bare_identity(owner.to_buffer()), 0, wallet_id, &p)
                .expect("add owner");
        }
        let outgoing = ContactRequest::new(owner, contact, 0, 0, 0, vec![0u8; 96], out_height, 0);
        let incoming = ContactRequest::new(contact, owner, 0, 0, 0, vec![0u8; 96], in_height, 0);
        info.identity_manager
            .managed_identity_mut(&owner)
            .expect("managed")
            .apply_established_contact(EstablishedContact::new(contact, outgoing, incoming));
    }

    async fn set_synced_height(
        manager: &Arc<PlatformWalletManager<RecordingPersister>>,
        wallet_id: WalletId,
        height: u32,
    ) {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let mut wm = wallet.identity().wallet_manager.write().await;
        wm.get_wallet_info_mut(&wallet_id)
            .expect("info")
            .core_wallet
            .update_synced_height(height);
    }

    async fn synced_height(
        manager: &Arc<PlatformWalletManager<RecordingPersister>>,
        wallet_id: WalletId,
    ) -> u32 {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let wm = wallet.identity().wallet_manager.read().await;
        wm.get_wallet_info(&wallet_id)
            .expect("info")
            .core_wallet
            .synced_height()
    }

    /// A contact established while the wallet was still catching up (funded at or
    /// after the current tip) is covered by the ongoing forward scan, so the
    /// rescan leaves `synced_height` alone — but it must MARK the contact so
    /// that, once the forward pointer later climbs past the contact's funding
    /// height, the recurring sweep does NOT redundantly rewind to an
    /// already-scanned range.
    #[tokio::test]
    async fn rescan_does_not_redundantly_rewind_a_forward_covered_contact() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        // Funded at 500, but we have only synced to 450 — below the funding
        // height, so a forward scan will cover it.
        establish_receival_contact(&manager, &persister, wallet_id, owner, contact, 500, 500).await;
        set_synced_height(&manager, wallet_id, 450).await;

        let iw_wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan"),
            None,
            "funded above the tip -> no backfill"
        );
        assert_eq!(
            synced_height(&manager, wallet_id).await,
            450,
            "height unchanged"
        );

        // Forward sync climbs past the funding height. The contact was marked,
        // so the next sweep must not re-lower to 500.
        set_synced_height(&manager, wallet_id, 600).await;
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan 2"),
            None,
            "forward-covered contact must not be redundantly rewound"
        );
        assert_eq!(
            synced_height(&manager, wallet_id).await,
            600,
            "no redundant rewind"
        );
    }

    /// Multiple contacts: the floor is the MINIMUM funding height across all
    /// not-yet-rescanned receival contacts (one rewind covers them all), and a
    /// later-discovered, older-funded contact re-lowers exactly once before the
    /// per-contact guard quiesces (drip-feed must not thrash).
    #[tokio::test]
    async fn rescan_uses_min_funding_across_contacts_and_drip_feed_settles() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let c_a = Identifier::from([0xA1; 32]);
        let c_b = Identifier::from([0xB2; 32]);
        let c_c = Identifier::from([0xC3; 32]);

        // Two contacts present at once (funded 300 and 100); tip at 1000.
        establish_receival_contact(&manager, &persister, wallet_id, owner, c_a, 300, 300).await;
        establish_receival_contact(&manager, &persister, wallet_id, owner, c_b, 100, 100).await;
        set_synced_height(&manager, wallet_id, 1000).await;

        let iw_wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan"),
            Some(100),
            "floor is the minimum funding height across all candidates"
        );
        assert_eq!(synced_height(&manager, wallet_id).await, 100);

        // Both are marked -> a second pass is a no-op.
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan 2"),
            None,
            "all candidates marked -> no re-trigger"
        );

        // A newly discovered, older-funded contact re-lowers exactly once...
        establish_receival_contact(&manager, &persister, wallet_id, owner, c_c, 50, 50).await;
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan 3"),
            Some(50),
            "a new older contact re-lowers to its funding height"
        );
        // ...then settles.
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan 4"),
            None,
            "drip-feed settles -> no further rewind"
        );
    }

    /// `synced_height == 0` means "scan from genesis / not started" — already a
    /// full historical scan, so the rescan is a no-op (the masking path the spec
    /// warns about).
    #[tokio::test]
    async fn rescan_is_a_noop_when_synced_height_is_zero() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        establish_receival_contact(&manager, &persister, wallet_id, owner, contact, 100, 100).await;
        set_synced_height(&manager, wallet_id, 0).await;

        let iw_wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        assert_eq!(
            iw_wallet
                .identity()
                .dashpay()
                .reconcile_dashpay_rescan()
                .await
                .expect("rescan"),
            None,
            "synced_height 0 -> no rescan"
        );
        assert_eq!(synced_height(&manager, wallet_id).await, 0);
    }

    /// A `Sent` payment must advance `Pending → Confirmed` once its
    /// transaction confirms on-chain. `send_payment` records it `Pending`
    /// and nothing else moved it, so before the confirm path was wired the
    /// entry was stuck `Pending` forever (sent payments never showed
    /// confirmed). Pins the flip, idempotency on re-detection, and that
    /// amount/memo are preserved.
    #[tokio::test]
    async fn confirm_flips_sent_payment_pending_to_confirmed() {
        use crate::wallet::identity::types::dashpay::payment::{
            PaymentDirection, PaymentEntry, PaymentStatus,
        };

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);
        let txid = "a".repeat(64);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();

        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.clone(),
                    PaymentEntry::new_sent(contact, 50_000, Some("dinner".into())),
                    &p,
                )
                .expect("record pending sent");
        }

        // Read the current entry under a short-lived read lock.
        async fn read_entry(
            iw: &crate::wallet::identity::IdentityWallet<crate::broadcaster::SpvBroadcaster>,
            wallet_id: &WalletId,
            owner: &Identifier,
            txid: &str,
        ) -> PaymentEntry {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(wallet_id).expect("info");
            info.identity_manager
                .managed_identity(owner)
                .unwrap()
                .dashpay()
                .payments
                .get(txid)
                .cloned()
                .expect("entry")
        }

        assert_eq!(
            read_entry(iw, &wallet_id, &owner, &txid).await.status,
            PaymentStatus::Pending,
            "precondition: entry starts Pending"
        );

        // A confirmed detection flips it to Confirmed, preserving fields.
        super::confirm_sent_payment_by_txid(&iw.wallet_manager, &wallet_id, &p, &txid).await;
        let entry = read_entry(iw, &wallet_id, &owner, &txid).await;
        assert_eq!(
            entry.status,
            PaymentStatus::Confirmed,
            "a confirmed tx must flip the Sent entry to Confirmed"
        );
        assert_eq!(entry.direction, PaymentDirection::Sent);
        assert_eq!(entry.amount_duffs, 50_000);
        assert_eq!(entry.memo.as_deref(), Some("dinner"), "memo preserved");

        // Idempotent: a second confirmed re-detection changes nothing.
        super::confirm_sent_payment_by_txid(&iw.wallet_manager, &wallet_id, &p, &txid).await;
        assert_eq!(
            read_entry(iw, &wallet_id, &owner, &txid).await.status,
            PaymentStatus::Confirmed
        );
    }

    /// A sent payment confirmed by a block must flip `Pending → Confirmed`.
    ///
    /// The wallet sees its *own* broadcast in the mempool first
    /// (`TransactionDetected`, context `Mempool`), where the confirm hook
    /// early-returns because the transaction is not yet confirmed. The
    /// transaction reaches a confirmed context only when a block mines it —
    /// delivered as [`key_wallet_manager::WalletEvent::BlockProcessed`] with
    /// the record in `updated` (a previously-known record that just
    /// confirmed). Routing the payment hooks only for `TransactionDetected`
    /// would leave the entry `Pending` forever. This drives the real adapter
    /// dispatch
    /// ([`run_dashpay_payment_hooks`](crate::wallet::identity::network::run_dashpay_payment_hooks))
    /// with a `BlockProcessed` event and pins the flip end-to-end, so a
    /// regression that re-narrows the routing to `TransactionDetected` is
    /// caught here. Also pins idempotency across a repeated block-processing
    /// round and that the `matured` bucket (coinbase maturity) never
    /// confirms a payment.
    #[tokio::test]
    async fn block_processed_confirms_sent_payment() {
        use dashcore::blockdata::transaction::Transaction;
        use dashcore::hashes::Hash;
        use dashcore::{BlockHash, TxIn};
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
        use key_wallet::WalletCoreBalance;
        use key_wallet_manager::WalletEvent;

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        // The sent transaction; `tx.txid()` is the payment-entry key, so the
        // entry and the confirming record agree on the same display-order
        // txid string the confirm path looks up.
        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(
                    dashcore::Txid::from_byte_array([0x5f; 32]),
                    0,
                ),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let txid = tx.txid();

        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_sent(contact, 100_000, Some("lunch".into())),
                    &p,
                )
                .expect("record pending sent");
        }

        // A block confirms the transaction; the wallet already knew it from
        // the mempool, so it rides `BlockProcessed.updated`.
        let confirmed = TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InBlock(BlockInfo::new(1_499_050, BlockHash::all_zeros(), 0)),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -100_000,
        );
        assert!(
            confirmed.is_confirmed(),
            "precondition: an InBlock record reports confirmed"
        );

        let event = WalletEvent::BlockProcessed {
            wallet_id,
            height: 1_499_050,
            chain_lock: None,
            inserted: Vec::new(),
            updated: vec![confirmed],
            matured: Vec::new(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };

        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;

        // Read the entry under a short-lived read lock so the re-fire below
        // can take the write lock.
        async fn read_status(
            iw: &crate::wallet::identity::IdentityWallet<crate::broadcaster::SpvBroadcaster>,
            wallet_id: &WalletId,
            owner: &Identifier,
            txid: &str,
        ) -> PaymentEntry {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(wallet_id).expect("info");
            info.identity_manager
                .managed_identity(owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(txid)
                .cloned()
                .expect("entry present under the sent txid")
        }

        let entry = read_status(iw, &wallet_id, &owner, &txid.to_string()).await;
        assert_eq!(
            entry.status,
            PaymentStatus::Confirmed,
            "a sent payment confirmed via BlockProcessed must flip Pending → Confirmed"
        );
        assert_eq!(entry.memo.as_deref(), Some("lunch"), "memo preserved");

        // Idempotent: a repeated block-processing round for the same txid
        // changes nothing (the confirm path skips entries past `Pending`).
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;
        assert_eq!(
            read_status(iw, &wallet_id, &owner, &txid.to_string())
                .await
                .status,
            PaymentStatus::Confirmed,
            "re-processing the same block must not change a Confirmed entry"
        );

        // A confirmed record arriving only in the `matured` bucket (coinbase
        // maturity) must NOT confirm a payment — `matured` is never a DashPay
        // payment, so it is excluded from the payment hooks.
        let matured_tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(
                    dashcore::Txid::from_byte_array([0xC0; 32]),
                    0,
                ),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let matured_txid = matured_tx.txid();
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    matured_txid.to_string(),
                    PaymentEntry::new_sent(contact, 7_000, None),
                    &p,
                )
                .expect("record pending sent");
        }
        let matured_record = TransactionRecord::new(
            matured_tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InChainLockedBlock(BlockInfo::new(
                1_499_060,
                BlockHash::all_zeros(),
                0,
            )),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -7_000,
        );
        let matured_event = WalletEvent::BlockProcessed {
            wallet_id,
            height: 1_499_060,
            chain_lock: None,
            inserted: Vec::new(),
            updated: Vec::new(),
            matured: vec![matured_record],
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &matured_event,
        )
        .await;
        assert_eq!(
            read_status(iw, &wallet_id, &owner, &matured_txid.to_string())
                .await
                .status,
            PaymentStatus::Pending,
            "a confirmed record in the `matured` bucket must not confirm a payment"
        );
    }

    /// An InstantSend lock applied to a previously-seen sent payment
    /// confirms it without waiting for a block. The lock arrives as
    /// `WalletEvent::TransactionInstantLocked` (no record, just a txid); an
    /// IS lock is final for DashPay display, so the entry flips
    /// `Pending → Confirmed`. Drives the real adapter dispatch.
    #[tokio::test]
    async fn instant_send_lock_confirms_sent_payment() {
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        use key_wallet::WalletCoreBalance;
        use key_wallet_manager::WalletEvent;

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);
        let txid = dashcore::Txid::from([0x5f; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_sent(contact, 50_000, None),
                    &p,
                )
                .expect("record pending sent");
        }

        let event = WalletEvent::TransactionInstantLocked {
            wallet_id,
            txid,
            instant_lock: InstantLock::default(),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
        };
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        let entry = info
            .identity_manager
            .managed_identity(&owner)
            .expect("managed")
            .dashpay()
            .payments
            .get(&txid.to_string())
            .cloned()
            .expect("entry present under the sent txid");
        assert_eq!(
            entry.status,
            PaymentStatus::Confirmed,
            "an InstantSend lock must confirm a sent payment"
        );
    }

    /// A transaction first seen *with* an InstantSend lock arrives as a
    /// `TransactionDetected` whose record context is `InstantSend`. The
    /// confirm gate accepts IS context (not just mined), so it flips the
    /// entry `Pending → Confirmed` — a plain mempool sighting would not.
    #[tokio::test]
    async fn instant_send_context_record_confirms_sent_payment() {
        use dashcore::blockdata::transaction::Transaction;
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        use dashcore::TxIn;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{TransactionContext, TransactionType};
        use key_wallet::WalletCoreBalance;
        use key_wallet_manager::WalletEvent;

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let tx = Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::new(dashcore::Txid::from([0x5e; 32]), 0),
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        };
        let txid = tx.txid();
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_sent(contact, 50_000, None),
                    &p,
                )
                .expect("record pending sent");
        }

        let record = TransactionRecord::new(
            tx,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            TransactionContext::InstantSend(InstantLock::default()),
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            -50_000,
        );
        assert!(
            !record.is_confirmed(),
            "precondition: an InstantSend record is not block-confirmed"
        );
        let event = WalletEvent::TransactionDetected {
            wallet_id,
            record: Box::new(record),
            balance: WalletCoreBalance::default(),
            account_balances: std::collections::BTreeMap::new(),
            addresses_derived: Vec::new(),
        };
        crate::wallet::identity::network::run_dashpay_payment_hooks(
            &iw.wallet_manager,
            &wallet_id,
            &p,
            &event,
        )
        .await;

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert_eq!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(&txid.to_string())
                .expect("entry")
                .status,
            PaymentStatus::Confirmed,
            "an InstantSend-context record must confirm a sent payment"
        );
    }

    /// `reconcile_sent_payments` recovers a `Pending` `Sent` payment whose
    /// live confirm event was missed: it flips the entry to `Confirmed` when
    /// the persisted core tx record reports the transaction final (mined or
    /// IS-locked), leaves a not-yet-final entry `Pending`, and is idempotent.
    #[tokio::test]
    async fn reconcile_sent_payments_confirms_from_persisted_record() {
        use dashcore::blockdata::transaction::Transaction;
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::account::account_type::StandardAccountType;
        use key_wallet::account::AccountType;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        // A persisted core tx record carrying only `txid` + `context` (the
        // contract `get_core_tx_record` guarantees).
        fn tx_record(txid: dashcore::Txid, context: TransactionContext) -> TransactionRecord {
            let tx = Transaction {
                version: 2,
                lock_time: 0,
                input: Vec::new(),
                output: Vec::new(),
                special_transaction_payload: None,
            };
            let mut record = TransactionRecord::new(
                tx,
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
            record.txid = txid;
            record
        }

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let mined_txid = dashcore::Txid::from([0x21; 32]);
        let mempool_txid = dashcore::Txid::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
            let managed = info
                .identity_manager
                .managed_identity_mut(&owner)
                .expect("managed");
            managed
                .record_dashpay_payment(
                    mined_txid.to_string(),
                    PaymentEntry::new_sent(contact, 1_000, None),
                    &p,
                )
                .expect("record mined-pending");
            managed
                .record_dashpay_payment(
                    mempool_txid.to_string(),
                    PaymentEntry::new_sent(contact, 2_000, None),
                    &p,
                )
                .expect("record mempool-pending");
        }
        {
            let mut recs = persister.records.lock().unwrap();
            recs.insert(
                mined_txid,
                tx_record(
                    mined_txid,
                    TransactionContext::InChainLockedBlock(BlockInfo::new(
                        100,
                        BlockHash::all_zeros(),
                        0,
                    )),
                ),
            );
            recs.insert(
                mempool_txid,
                tx_record(mempool_txid, TransactionContext::Mempool),
            );
        }

        let n = iw
            .dashpay()
            .reconcile_sent_payments()
            .await
            .expect("reconcile");
        assert_eq!(n, 1, "only the mined payment is confirmed this pass");

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let managed = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed");
            assert_eq!(
                managed
                    .dashpay()
                    .payments
                    .get(&mined_txid.to_string())
                    .expect("mined entry")
                    .status,
                PaymentStatus::Confirmed,
                "a mined tx record must confirm the sent payment"
            );
            assert_eq!(
                managed
                    .dashpay()
                    .payments
                    .get(&mempool_txid.to_string())
                    .expect("mempool entry")
                    .status,
                PaymentStatus::Pending,
                "a not-yet-final tx must leave the sent payment Pending"
            );
        }

        // Idempotent: a second pass confirms nothing new (the mined entry
        // is already Confirmed, the mempool one is still not final).
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments()
                .await
                .expect("second pass"),
            0,
            "reconcile must be idempotent"
        );
    }

    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_rebuilds_and_is_idempotent() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        use crate::wallet::identity::types::dashpay::payment::PaymentStatus;

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_addresses = install_external_account(&manager, wallet_id, owner, contact).await;
        assert!(
            contact_addresses.len() >= 2,
            "external account must pre-derive at least two addresses"
        );
        let change_address = first_standard_wallet_address(&manager, wallet_id).await;

        let record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(123, BlockHash::all_zeros(), 0)),
            vec![
                (contact_addresses[0].clone(), 25_000, OutputRole::Sent),
                (change_address, 90_000, OutputRole::Change),
                (contact_addresses[1].clone(), 10_000, OutputRole::Sent),
            ],
        );
        let txid = record.txid;
        persister.records.lock().unwrap().insert(txid, record);

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            1,
            "one reconstructed payment should be recorded"
        );

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let entry = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(&txid.to_string())
                .cloned()
                .expect("reconstructed sent payment");
            assert_eq!(entry.amount_duffs, 35_000, "sum all contact outputs only");
            assert_eq!(entry.status, PaymentStatus::Confirmed);
        }

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("second pass"),
            0,
            "reconstruction must be idempotent"
        );
    }

    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_does_not_overwrite_existing_entry() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        use crate::wallet::identity::types::dashpay::payment::{PaymentEntry, PaymentStatus};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_address = install_external_account(&manager, wallet_id, owner, contact)
            .await
            .remove(0);
        let record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(55, BlockHash::all_zeros(), 0)),
            vec![(contact_address, 50_000, OutputRole::Sent)],
        );
        let txid = record.txid;
        persister.records.lock().unwrap().insert(txid, record);

        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    txid.to_string(),
                    PaymentEntry::new_received(contact, 7_500, Some("keep me".into())),
                    &p,
                )
                .expect("preexisting received entry");
        }

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            0,
            "an existing txid entry must win the dedup guard"
        );

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let entry = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(&txid.to_string())
                .cloned()
                .expect("entry still present");
            assert_eq!(entry.amount_duffs, 7_500);
            assert_eq!(entry.status, PaymentStatus::Confirmed);
            assert_eq!(entry.memo.as_deref(), Some("keep me"));
        }
    }

    /// Reconstruction must work on the record shape the FFI actually hands
    /// back. `PlatformWalletPersistence::get_core_tx_record` rebuilds a record
    /// from the host's raw transaction bytes and fills only `transaction`,
    /// `txid` and `context` — `output_details` is always empty. The test
    /// helper populates both, which is why a version of this sweep that read
    /// `output_details` passed every unit test and matched nothing on device
    /// (`outputs_scanned=0`, `matched_txids=0` against 49 records read).
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_matches_without_output_details() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        use crate::wallet::identity::types::dashpay::payment::PaymentDirection;

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_address = install_external_account(&manager, wallet_id, owner, contact)
            .await
            .remove(0);
        let mut record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(77, BlockHash::all_zeros(), 0)),
            vec![(contact_address, 250_000, OutputRole::Sent)],
        );
        // Exactly what the FFI returns: scripts on the decoded transaction,
        // nothing in the details vec.
        record.output_details.clear();
        let txid = record.txid;
        persister.records.lock().unwrap().insert(txid, record);

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            1,
            "matching must not depend on `output_details`, which the FFI leaves empty"
        );

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let entry = info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(&txid.to_string())
                .cloned()
                .expect("reconstructed entry");
            assert_eq!(entry.direction, PaymentDirection::Sent);
            assert_eq!(entry.amount_duffs, 250_000);
            assert_eq!(entry.counterparty_id, contact);
        }
    }

    /// A contact we have also *received* from must still get its sends
    /// reconstructed. `reconcile_incoming_payments` runs first and fills the
    /// payments map with `Received` entries; a skip-guard that only asked
    /// "any payment with this contact?" read that as "already reconstructed"
    /// and permanently hid the outgoing history for every two-way contact.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_reconstructs_for_contact_with_received_history(
    ) {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        use crate::wallet::identity::types::dashpay::payment::{
            PaymentDirection, PaymentEntry, PaymentStatus,
        };

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_address = install_external_account(&manager, wallet_id, owner, contact)
            .await
            .remove(0);
        let record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(55, BlockHash::all_zeros(), 0)),
            vec![(contact_address, 50_000, OutputRole::Sent)],
        );
        let sent_txid = record.txid;
        persister.records.lock().unwrap().insert(sent_txid, record);

        // An unrelated incoming payment from the same contact, as the incoming
        // reconcile would have left it — a different txid, so the per-txid
        // dedup guard is not what is under test here.
        let received_txid = "11".repeat(32);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("managed")
                .record_dashpay_payment(
                    received_txid.clone(),
                    PaymentEntry::new_received(contact, 7_500, None),
                    &p,
                )
                .expect("preexisting received entry");
        }

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            1,
            "received history with a contact must not suppress the sent sweep"
        );

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let payments = &info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments;
            let entry = payments.get(&sent_txid.to_string()).expect("sent entry");
            assert_eq!(entry.direction, PaymentDirection::Sent);
            assert_eq!(entry.amount_duffs, 50_000);
            assert_eq!(entry.status, PaymentStatus::Confirmed);
            // The incoming entry is untouched.
            let received = payments.get(&received_txid).expect("received entry");
            assert_eq!(received.direction, PaymentDirection::Received);
            assert_eq!(received.amount_duffs, 7_500);
        }
    }

    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_keeps_mempool_entries_pending() {
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::TransactionContext;

        use crate::wallet::identity::types::dashpay::payment::PaymentStatus;

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_address = install_external_account(&manager, wallet_id, owner, contact)
            .await
            .remove(0);
        let record = tx_record_with_outputs(
            TransactionContext::Mempool,
            vec![(contact_address, 11_000, OutputRole::Sent)],
        );
        let txid = record.txid;
        persister.records.lock().unwrap().insert(txid, record);

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            1
        );

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert_eq!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(&txid.to_string())
                .expect("entry")
                .status,
            PaymentStatus::Pending,
            "a mempool tx must reconstruct as Pending until the confirm sweep flips it"
        );
    }

    /// An empty enumeration is inconclusive, so the sweep must keep retrying.
    ///
    /// After a restore the recurring `dashpay_sync()` can fire before the host
    /// has repopulated its transaction table. Treating that zero-txid answer as
    /// "nothing to reconstruct" would stamp the per-launch guard and end
    /// recovery for the rest of the process.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_retries_after_empty_enumeration() {
        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let _ = install_external_account(&manager, wallet_id, owner, contact).await;

        for pass in 1..=2 {
            assert_eq!(
                iw.dashpay()
                    .reconcile_sent_payments_from_tx_history()
                    .await
                    .expect("reconcile"),
                0,
                "an empty tx history should produce no reconstructed payments (pass {pass})"
            );
        }
        assert_eq!(
            *persister.list_wallet_core_txids_calls.lock().unwrap(),
            2,
            "an empty enumeration must not be taken as conclusive"
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            0,
            "with no txids there should be no per-record reads"
        );
    }

    /// A stretch of unused indices must not stop the walk.
    ///
    /// `send_payment` marks the chosen contact address used before
    /// `build_signed` and never rolls that back when the build fails, so a
    /// contact's real payment can sit past a hole no on-chain output bridges.
    /// The match-driven extension alone cannot cross that hole — it only
    /// extends past an address it has already seen paid — so the seed window
    /// has to be wider than one gap limit.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_crosses_a_full_unused_gap() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let _ = install_external_account(&manager, wallet_id, owner, contact).await;

        // An address a full gap limit past the materialized frontier, with
        // NOTHING paid in between — the hole a run of failed builds leaves.
        let (beyond_gap, materialized_max) = {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            let key = DashpayAccountKey {
                index: 0,
                user_identity_id: owner.to_buffer(),
                friend_identity_id: contact.to_buffer(),
            };
            let account = info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .get(&key)
                .expect("external account");
            let pools = account.managed_account_type().address_pools();
            let pool = *pools.first().expect("pool");
            let materialized_max = pool.addresses.keys().copied().max().unwrap_or(0);
            let target = materialized_max + pool.gap_limit + 1;
            let mut scan = pool.clone();
            let key_source = key_wallet::KeySource::Public(test_receiving_xpub(&owner, &contact));
            scan.generate_addresses(target + 1, &key_source, true)
                .expect("derive past the hole");
            (
                scan.addresses
                    .get(&target)
                    .expect("target derived")
                    .address
                    .clone(),
                materialized_max,
            )
        };

        let record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(88, BlockHash::all_zeros(), 0)),
            vec![(beyond_gap, 60_000, OutputRole::Sent)],
        );
        let txid = record.txid;
        persister.records.lock().unwrap().insert(txid, record);

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            1,
            "a payment past a full unused gap (materialized up to {materialized_max}) must still be found"
        );
        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            assert_eq!(
                info.identity_manager
                    .managed_identity(&owner)
                    .expect("managed")
                    .dashpay()
                    .payments
                    .get(&txid.to_string())
                    .expect("reconstructed entry")
                    .amount_duffs,
                60_000
            );
        }
    }

    /// A sweep certifies only the history that existed at the height it ran
    /// against, so the contact becomes eligible again as soon as the scan
    /// advances.
    ///
    /// The alternative — a bare "already swept" flag — ends recovery on
    /// whatever prefix of the transaction table happened to be visible. That
    /// prefix is not under our control: `dashpay_sync` runs this sweep before
    /// `reconcile_dashpay_rescan`, and on an initial or forward-only scan rows
    /// keep arriving with every new block.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_resweeps_when_the_table_changes() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let addresses = install_external_account(&manager, wallet_id, owner, contact).await;
        let own_address = first_standard_wallet_address(&manager, wallet_id).await;

        // First pass: one unrelated transaction exists, so the scan is
        // conclusive for the table it enumerated and stamps its digest.
        let unrelated = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(10, BlockHash::all_zeros(), 0)),
            vec![(own_address, 1_000, OutputRole::Sent)],
        );
        persister
            .records
            .lock()
            .unwrap()
            .insert(unrelated.txid, unrelated);
        iw.dashpay()
            .reconcile_sent_payments_from_tx_history()
            .await
            .expect("first sweep");
        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            assert!(
                info.identity_manager
                    .managed_identity(&owner)
                    .expect("managed")
                    .dashpay()
                    .sent_payment_reconcile_swept_table
                    .contains_key(&contact),
                "the sweep records the table digest it certified"
            );
        }

        // A payment row lands AFTER the certified pass — a rescan backfill
        // delivering history, or the wallet-event adapter committing
        // asynchronously. No chain-height advance is involved: the row's
        // arrival alone changes the table digest, so the very next sweep
        // recovers it. (The prior height-stamped guard ignored rows like
        // this until another block happened to arrive.)
        let late = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(700, BlockHash::all_zeros(), 0)),
            vec![(addresses[0].clone(), 40_000, OutputRole::Sent)],
        );
        let late_txid = late.txid;
        persister.records.lock().unwrap().insert(late_txid, late);
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("sweep after a new row"),
            1,
            "a changed table must re-open the contact immediately"
        );
        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            assert!(info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .contains_key(&late_txid.to_string()));
        }

        // Unchanged table: the new stamp holds — one enumeration, no record
        // reads, nothing recorded.
        let reads_before = *persister.get_core_tx_record_calls.lock().unwrap();
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("sweep on the unchanged table"),
            0,
            "an unchanged table must not re-run the walk"
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            reads_before,
            "an unchanged table must not re-read any records"
        );
    }

    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_skips_repeat_sweeps_after_a_real_scan() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let _ = install_external_account(&manager, wallet_id, owner, contact).await;

        // A transaction that pays someone else: the enumeration is non-empty,
        // so the scan is conclusive even though it reconstructs nothing.
        let unrelated = dashcore::Address::p2pkh(
            &dashcore::PublicKey::from_slice(&[
                0x02, 0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE,
                0x87, 0x0B, 0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81,
                0x5B, 0x16, 0xF8, 0x17, 0x98,
            ])
            .expect("valid compressed pubkey"),
            Network::Testnet,
        );
        let record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(11, BlockHash::all_zeros(), 0)),
            vec![(unrelated, 1_000, OutputRole::Sent)],
        );
        persister
            .records
            .lock()
            .unwrap()
            .insert(record.txid, record);

        for pass in 1..=2 {
            assert_eq!(
                iw.dashpay()
                    .reconcile_sent_payments_from_tx_history()
                    .await
                    .expect("reconcile"),
                0,
                "nothing pays this contact (pass {pass})"
            );
        }
        assert_eq!(
            *persister.list_wallet_core_txids_calls.lock().unwrap(),
            2,
            "every sweep pays exactly one txid enumeration to detect table changes"
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            1,
            "the second pass must early-exit on the digest before any tx-record fetch"
        );
    }

    /// A failed persist for one payment must be retried even when a sibling
    /// payment to the same contact was written successfully — "the contact
    /// has a `Sent` entry" is not a completion marker.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_retries_failed_write_after_sibling_success() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_addresses = install_external_account(&manager, wallet_id, owner, contact).await;
        let record_a = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(101, BlockHash::all_zeros(), 0)),
            vec![(contact_addresses[0].clone(), 10_000, OutputRole::Sent)],
        );
        let record_b = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(102, BlockHash::all_zeros(), 0)),
            vec![(contact_addresses[1].clone(), 20_000, OutputRole::Sent)],
        );
        let txid_a = record_a.txid;
        let txid_b = record_b.txid;
        {
            let mut recs = persister.records.lock().unwrap();
            recs.insert(txid_a, record_a);
            recs.insert(txid_b, record_b);
        }

        // First sweep: one write lands, the second fails.
        *persister.allow_stores_then_fail.lock().unwrap() = Some(1);
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("first sweep"),
            1,
            "exactly one write should survive the injected failure"
        );

        // Second sweep, persistence healthy again: the stranded payment must
        // be recorded even though the contact already has a `Sent` entry.
        *persister.allow_stores_then_fail.lock().unwrap() = None;
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("second sweep"),
            1,
            "the failed sibling write must be retried on the next sweep"
        );

        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let payments = &info
                .identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments;
            assert!(payments.contains_key(&txid_a.to_string()));
            assert!(payments.contains_key(&txid_b.to_string()));
        }

        // Both recorded → the digest is stamped; an unchanged table costs
        // later sweeps one enumeration and zero record reads.
        let reads_after_success = *persister.get_core_tx_record_calls.lock().unwrap();
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("third sweep"),
            0
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            reads_after_success,
            "the third sweep must early-exit on the digest before any tx-record fetch"
        );
    }

    /// A listed txid whose record comes back `Ok(None)` means "not available
    /// yet" (the FFI collapses backend failures, missing bytes and pending
    /// InstantSend rows into a miss), so the sweep must not certify the
    /// contact as complete until every listed wallet-funded record was read.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_retries_when_listed_record_is_unavailable() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_addresses = install_external_account(&manager, wallet_id, owner, contact).await;
        let available = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(77, BlockHash::all_zeros(), 0)),
            vec![(contact_addresses[0].clone(), 25_000, OutputRole::Sent)],
        );
        let available_txid = available.txid;
        persister
            .records
            .lock()
            .unwrap()
            .insert(available_txid, available);
        let ghost_txid = dashcore::Txid::from([0x42; 32]);
        persister
            .listed_but_unavailable
            .lock()
            .unwrap()
            .insert(ghost_txid);

        // First sweep records what it can read but must NOT stamp the guard:
        // one listed wallet-funded record was unavailable.
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("first sweep"),
            1
        );

        // The record becomes readable (e.g. the InstantSend row mined) and
        // turns out to pay the contact too — the retry must pick it up.
        persister
            .listed_but_unavailable
            .lock()
            .unwrap()
            .remove(&ghost_txid);
        let late = {
            let mut record = tx_record_with_outputs(
                TransactionContext::InBlock(BlockInfo::new(78, BlockHash::all_zeros(), 0)),
                vec![(contact_addresses[1].clone(), 30_000, OutputRole::Sent)],
            );
            record.txid = ghost_txid;
            record
        };
        persister.records.lock().unwrap().insert(ghost_txid, late);
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("second sweep"),
            1,
            "an unavailable listed record must keep the sweep retrying"
        );

        // Now conclusive: the digest is stamped and record reads stop while
        // the table stays unchanged.
        let reads_after_success = *persister.get_core_tx_record_calls.lock().unwrap();
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("third sweep"),
            0
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            reads_after_success,
            "the third sweep must early-exit on the digest before any tx-record fetch"
        );
    }

    /// Historical sends land past the initial gap window after a
    /// restore-from-seed: the resident pool materializes only the first
    /// `gap_limit` addresses, while real usage chained further. The sweep
    /// must extend its derivation window (matched index + gap limit) instead
    /// of matching only what the pool already holds.
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_finds_payments_past_initial_gap_window() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::address_pool::KeySource;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let _ = install_external_account(&manager, wallet_id, owner, contact).await;

        // Derive addresses past the resident window from the same xpub the
        // account was installed with, exactly like a live wallet whose sends
        // consumed the early indices before the restore.
        let (near_address, far_address, far_index) = {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            let key = DashpayAccountKey {
                index: 0,
                user_identity_id: owner.to_buffer(),
                friend_identity_id: contact.to_buffer(),
            };
            let pools = info
                .core_wallet
                .accounts
                .dashpay_external_accounts
                .get(&key)
                .expect("external account")
                .managed_account_type()
                .address_pools();
            let pool = (*pools.first().expect("pool")).clone();
            let near_index = pool
                .highest_generated
                .expect("resident pool has a generated window");
            let far_index = near_index + pool.gap_limit;
            let mut extended = pool;
            let key_source = KeySource::Public(test_receiving_xpub(&owner, &contact));
            extended
                .generate_addresses(extended.gap_limit + 1, &key_source, true)
                .expect("extend clone past the resident window");
            (
                extended.addresses[&near_index].address.clone(),
                extended.addresses[&far_index].address.clone(),
                far_index,
            )
        };

        let near_record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(201, BlockHash::all_zeros(), 0)),
            vec![(near_address, 10_000, OutputRole::Sent)],
        );
        let far_record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(202, BlockHash::all_zeros(), 0)),
            vec![(far_address, 20_000, OutputRole::Sent)],
        );
        let far_txid = far_record.txid;
        {
            let mut recs = persister.records.lock().unwrap();
            recs.insert(near_record.txid, near_record);
            recs.insert(far_txid, far_record);
        }

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            2,
            "the payment at derivation index {far_index} must be found by the range walk"
        );
        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert_eq!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("managed")
                .dashpay()
                .payments
                .get(&far_txid.to_string())
                .expect("payment past the initial window")
                .amount_duffs,
            20_000
        );
    }

    /// A transaction the wallet did not fund — a third party paying the
    /// watched contact address — must never be recorded as `Sent`, and
    /// skipping it is conclusive (the guard still stamps).
    #[tokio::test]
    async fn reconcile_sent_payments_from_tx_history_skips_transactions_wallet_did_not_fund() {
        use dashcore::hashes::Hash;
        use dashcore::BlockHash;
        use key_wallet::managed_account::transaction_record::OutputRole;
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext};

        let persister = Arc::new(RecordStorePersister::default());
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&persister)).await;
        let owner = Identifier::from([0xAA; 32]);
        let contact = Identifier::from([0xBB; 32]);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(bare_identity([0xAA; 32]), 0, wallet_id, &p)
                .expect("add owner");
        }

        let contact_address = install_external_account(&manager, wallet_id, owner, contact)
            .await
            .remove(0);
        let record = tx_record_with_outputs(
            TransactionContext::InBlock(BlockInfo::new(300, BlockHash::all_zeros(), 0)),
            vec![(contact_address, 40_000, OutputRole::Sent)],
        );
        let txid = record.txid;
        persister.records.lock().unwrap().insert(txid, record);
        persister.not_wallet_funded.lock().unwrap().insert(txid);

        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("reconcile"),
            0,
            "a third-party payment to the contact must not become our Sent history"
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            0,
            "a non-wallet-funded tx must be skipped without a record read"
        );
        {
            let wm = iw.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("info");
            assert!(
                info.identity_manager
                    .managed_identity(&owner)
                    .expect("managed")
                    .dashpay()
                    .payments
                    .is_empty(),
                "no payment entry may be fabricated"
            );
        }

        // Skipping unfunded transactions is conclusive — steady state holds.
        assert_eq!(
            iw.dashpay()
                .reconcile_sent_payments_from_tx_history()
                .await
                .expect("second sweep"),
            0
        );
        assert_eq!(
            *persister.get_core_tx_record_calls.lock().unwrap(),
            0,
            "the second sweep must early-exit on the digest before any tx-record fetch"
        );
    }

    /// The seedless drain path: `register_external_contact_account` with a
    /// **precomputed** ECDH shared secret (the Keychain signer computed it; the
    /// scalar never entered this crate) decrypts the contact's xpub and builds
    /// the `DashpayExternalAccount` — same result as the resident path. Pins the
    /// reuse that lets the deferred-crypto drain complete an external-account
    /// build once a signer is available. The contact identity is `bare` here,
    /// proving the `Some` path skips the peer-key derivation entirely.
    #[tokio::test]
    async fn register_external_with_precomputed_shared_key_builds_account() {
        let (manager, persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner_id = Identifier::from([0x11; 32]);
        let contact_id = Identifier::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
        }

        // A real 69-byte compact xpub encrypted under a known shared key — the
        // wire shape a contact would have sent us.
        let shared_key = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let compact = {
            let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
                .expect("mnemonic")
                .to_seed("");
            let w = key_wallet::wallet::Wallet::from_seed_bytes(
                seed,
                Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("seed wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                &w,
                Network::Testnet,
                0,
                &owner_id,
                &contact_id,
            )
            .expect("derive a valid compact xpub")
            .compact
            .to_bytes()
        };
        let encrypted =
            platform_encryption::encrypt_extended_public_key(&shared_key, &iv, &compact);

        // Bare contact identity: the `Some` path must NOT touch the contact's
        // encryption key (the signer derives the secret out-of-crate).
        let contact = bare_identity([0x22; 32]);
        let registration = iw
            .dashpay()
            .register_external_contact_account(
                &owner_id,
                &contact,
                &encrypted,
                zeroize::Zeroizing::new(shared_key),
            )
            .await
            .expect("register external with a signer-derived shared key");
        assert_eq!(
            registration,
            crate::wallet::identity::network::contacts::ExternalAccountRegistration::Built,
            "a fresh registration must report Built (AlreadyExisted must not stamp the marker)"
        );

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        use key_wallet::account::account_collection::DashpayAccountKey;
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner_id.to_buffer(),
            friend_identity_id: contact_id.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_external_accounts
                .contains_key(&key),
            "the precomputed-shared-key path must build the external account (the drain's path)"
        );
    }

    /// Build a wallet with one owner identity (`[0x11;32]`) and one established
    /// contact (`[0x22;32]`) whose incoming / outgoing requests carry the given
    /// already-encrypted account-label ciphertexts. Returns the manager + the
    /// owner/contact ids; the caller fetches the wallet to drive the helper.
    async fn wallet_with_labeled_contact(
        incoming_label: Option<Vec<u8>>,
        outgoing_label: Option<Vec<u8>>,
    ) -> (
        Arc<PlatformWalletManager<RecordingPersister>>,
        WalletId,
        Identifier,
        Identifier,
    ) {
        use crate::wallet::identity::{ContactRequest, EstablishedContact};

        let (manager, persister, wallet_id) = make_wallet().await;
        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);
        let p = WalletPersister::new(wallet_id, Arc::clone(&persister) as _);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
        info.identity_manager
            .add_identity(bare_identity([0x11; 32]), 0, wallet_id, &p)
            .expect("add owner");
        let mut outgoing = ContactRequest::new(owner, contact, 0, 0, 0, vec![0u8; 96], 200, 0);
        outgoing.encrypted_account_label = outgoing_label;
        let mut incoming = ContactRequest::new(contact, owner, 0, 0, 0, vec![0u8; 96], 100, 0);
        incoming.encrypted_account_label = incoming_label;
        info.identity_manager
            .managed_identity_mut(&owner)
            .expect("managed")
            .apply_established_contact(EstablishedContact::new(contact, outgoing, incoming));
        drop(wm);

        (manager, wallet_id, owner, contact)
    }

    /// Read back the stored `contact_account_label` for the test contact.
    async fn stored_label(
        manager: &PlatformWalletManager<RecordingPersister>,
        wallet_id: &WalletId,
        owner: &Identifier,
        contact: &Identifier,
    ) -> Option<String> {
        let wallet = manager.get_wallet(wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let wm = iw.wallet_manager.read().await;
        let label = wm
            .get_wallet_info(wallet_id)
            .and_then(|info| info.identity_manager.managed_identity(owner))
            .and_then(|m| m.dashpay().established_contacts().get(contact))
            .and_then(|c| c.contact_account_label.clone());
        label
    }

    /// DIP-15 §8.5 receive-side surfacing: the contact's `encryptedAccountLabel`
    /// on their INCOMING request is decrypted with the ECDH shared key and
    /// stored as `contact_account_label`. Re-running the helper is idempotent.
    /// (Before the helper existed, the field stayed `None` — red→green.)
    #[tokio::test]
    async fn store_contact_account_label_surfaces_incoming_label() {
        let shared = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let ct = platform_encryption::encrypt_account_label(&shared, &iv, "Main wallet");
        let (manager, wallet_id, owner, contact) =
            wallet_with_labeled_contact(Some(ct), None).await;

        let drive = || async {
            let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
            wallet
                .identity()
                .dashpay()
                .store_contact_account_label(&owner, &contact, &shared)
                .await;
        };
        drive().await;
        assert_eq!(
            stored_label(&manager, &wallet_id, &owner, &contact).await,
            Some("Main wallet".to_string()),
            "the contact's incoming account label must be decrypted and surfaced"
        );
        // Idempotent: a second pass yields the same value, no panic.
        drive().await;
        assert_eq!(
            stored_label(&manager, &wallet_id, &owner, &contact).await,
            Some("Main wallet".to_string()),
        );
    }

    /// The label is direction-specific: the surfaced value comes from the
    /// contact's INCOMING request, never our OUTGOING one (which carries a
    /// label *we* chose). Pins that an outgoing label can't win.
    #[tokio::test]
    async fn store_contact_account_label_uses_incoming_not_outgoing() {
        let shared = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let theirs = platform_encryption::encrypt_account_label(&shared, &iv, "Their account");
        let ours = platform_encryption::encrypt_account_label(&shared, &iv, "My own account");
        let (manager, wallet_id, owner, contact) =
            wallet_with_labeled_contact(Some(theirs), Some(ours)).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        wallet
            .identity()
            .dashpay()
            .store_contact_account_label(&owner, &contact, &shared)
            .await;

        assert_eq!(
            stored_label(&manager, &wallet_id, &owner, &contact).await,
            Some("Their account".to_string()),
            "the surfaced label must come from the contact's INCOMING request"
        );
    }

    /// No label on the incoming request → nothing surfaced.
    #[tokio::test]
    async fn store_contact_account_label_no_label_is_none() {
        let shared = [0x55u8; 32];
        let (manager, wallet_id, owner, contact) = wallet_with_labeled_contact(None, None).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        wallet
            .identity()
            .dashpay()
            .store_contact_account_label(&owner, &contact, &shared)
            .await;

        assert_eq!(
            stored_label(&manager, &wallet_id, &owner, &contact).await,
            None,
        );
    }

    /// Cosmetic-failure policy: an undecryptable ciphertext leaves the label
    /// unset — never breaks the channel, never surfaces garbage.
    #[tokio::test]
    async fn store_contact_account_label_undecryptable_is_none() {
        let shared = [0x55u8; 32];
        // 50 bytes = 16-byte IV + 34-byte body (not block-aligned) → decrypt errors.
        let (manager, wallet_id, owner, contact) =
            wallet_with_labeled_contact(Some(vec![0u8; 50]), None).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        wallet
            .identity()
            .dashpay()
            .store_contact_account_label(&owner, &contact, &shared)
            .await;

        assert_eq!(
            stored_label(&manager, &wallet_id, &owner, &contact).await,
            None,
            "an undecryptable label must be left unset, not surfaced as garbage"
        );
    }

    /// AES-CBC has no integrity, so a corrupt / non-conforming-sender ciphertext
    /// can *decrypt* to valid UTF-8 that contains control characters. Unlike the
    /// undecryptable case above (the `Err` arm), this exercises the `Ok`-with-
    /// garbage **sanitize** branch: a printable-looking-but-control-laden label
    /// must be coerced to `None` so the UI shows nothing rather than garbage.
    #[tokio::test]
    async fn store_contact_account_label_control_chars_coerced_to_none() {
        let shared = [0x55u8; 32];
        let iv = [0x11u8; 16];
        // Decrypts cleanly to a string carrying a control character (bell).
        let ct = platform_encryption::encrypt_account_label(&shared, &iv, "bad\u{07}label");
        let (manager, wallet_id, owner, contact) =
            wallet_with_labeled_contact(Some(ct), None).await;

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        wallet
            .identity()
            .dashpay()
            .store_contact_account_label(&owner, &contact, &shared)
            .await;

        assert_eq!(
            stored_label(&manager, &wallet_id, &owner, &contact).await,
            None,
            "a label decrypting to control characters must be suppressed, not surfaced"
        );
    }

    /// The seedless drain's RegisterReceiving path: `register_contact_account`
    /// with a **precomputed** receiving xpub (the Keychain signer derived our
    /// friendship key) builds the `DashpayReceivingFunds` account without
    /// touching the wallet seed. Pins the reuse the drain needs when the
    /// receiving account was never persisted (restore / first-time edge).
    #[tokio::test]
    async fn register_contact_account_with_precomputed_xpub_builds_account() {
        let (manager, _persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        // A valid ExtendedPubKey to supply as the signer would.
        let supplied_xpub = test_receiving_xpub(&owner, &contact);

        iw.dashpay()
            .register_contact_account(&owner, &contact, 0, supplied_xpub)
            .await
            .expect("register receiving account with a precomputed xpub");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        use key_wallet::account::account_collection::DashpayAccountKey;
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_receival_accounts
                .contains_key(&key),
            "the precomputed-xpub path must build the receiving account (the drain's RegisterReceiving)"
        );
    }

    /// End-to-end drain of a `RegisterReceiving` entry on a SEEDLESS wallet: the
    /// `SeedCryptoProvider` (the faithful test stand-in for the Keychain signer)
    /// supplies the receiving xpub, the drain builds the receiving account with
    /// that EXACT signer-derived xpub, and the entry is cleared from the queue.
    /// Pins that a wallet with no resident seed becomes payable purely through
    /// the signer-backed drain.
    #[tokio::test]
    async fn drain_completes_register_receiving_and_clears_queue() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, persister, wallet_id) = make_watch_only_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        // The signer's seed (the faithful test stand-in derives from it).
        let seed = {
            let mnemonic =
                Mnemonic::from_phrase(TEST_MNEMONIC, Language::English).expect("valid mnemonic");
            mnemonic.to_seed("")
        };

        // Register the owner, then enqueue a RegisterReceiving op (as the
        // seedless sweep would) onto its per-identity queue.
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("owner resident")
                .dashpay_pending_contact_crypto_mut()
                .push(PendingContactCrypto {
                    owner_identity_id: owner,
                    contact_id: contact,
                    op: PendingContactCryptoOp::RegisterReceiving,
                    enqueued_at_ms: 0,
                });
        }

        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let drained = iw.dashpay().drain_pending_contact_crypto(&provider).await;
        assert_eq!(drained, 1, "the RegisterReceiving entry must be drained");

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("owner resident")
                .dashpay()
                .pending_contact_crypto
                .is_empty(),
            "the queue must be cleared after a successful drain"
        );
        use key_wallet::account::account_collection::DashpayAccountKey;
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: contact.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_receival_accounts
                .contains_key(&key),
            "the seedless drain must build the receiving account via the signer provider"
        );
    }

    /// `pending_contact_crypto_count` (the "waiting to finish setup" banner
    /// source) MUST sum across BOTH identity buckets. An `AutoAccept` op can
    /// legitimately sit on an out-of-wallet identity, so a count walking only the
    /// wallet-owned bucket would silently under-count it — the R1 regression the
    /// per-identity move risks. Calls the real async method (not an inlined copy),
    /// so a bucket-dropping regression fails HERE.
    #[tokio::test]
    async fn pending_contact_crypto_count_method_spans_both_identity_buckets() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};

        let (manager, persister, wallet_id) = make_watch_only_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owned = Identifier::from([0x41; 32]);
        let watched = Identifier::from([0x42; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            // Owned identity with a RegisterReceiving op.
            info.identity_manager
                .add_identity(
                    bare_identity([0x41; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owned");
            info.identity_manager
                .managed_identity_mut(&owned)
                .expect("owned resident")
                .dashpay_pending_contact_crypto_mut()
                .push(PendingContactCrypto {
                    owner_identity_id: owned,
                    contact_id: Identifier::from([0x01; 32]),
                    op: PendingContactCryptoOp::RegisterReceiving,
                    enqueued_at_ms: 0,
                });
            // OUT-OF-WALLET identity with an AutoAccept op — the case an
            // owned-bucket-only count would silently miss.
            info.identity_manager
                .add_out_of_wallet_identity(
                    bare_identity([0x42; 32]),
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add out-of-wallet");
            info.identity_manager
                .managed_identity_mut(&watched)
                .expect("watched resident")
                .dashpay_pending_contact_crypto_mut()
                .push(PendingContactCrypto {
                    owner_identity_id: watched,
                    contact_id: Identifier::from([0x02; 32]),
                    op: PendingContactCryptoOp::AutoAccept,
                    enqueued_at_ms: 0,
                });
        }

        assert_eq!(
            iw.dashpay().pending_contact_crypto_count().await,
            2,
            "count must aggregate across both identity buckets, including the \
             out-of-wallet AutoAccept"
        );
    }

    /// The drain snapshot MUST reach BOTH identity buckets. A `RegisterReceiving`
    /// op on an OUT-OF-WALLET identity (the provider-only drain completes it with
    /// no signer; its arm doesn't gate on the HD index) must be processed and
    /// cleared — if the drain snapshotted only the wallet-owned bucket, this entry
    /// would be silently skipped (`drained == 0`). Pins the drain half of R1.
    #[tokio::test]
    async fn drain_processes_out_of_wallet_identity_queue() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, persister, wallet_id) = make_watch_only_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let watched = Identifier::from([0x42; 32]);
        let contact = Identifier::from([0x22; 32]);
        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");

        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_out_of_wallet_identity(
                    bare_identity([0x42; 32]),
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add out-of-wallet");
            info.identity_manager
                .managed_identity_mut(&watched)
                .expect("watched resident")
                .dashpay_pending_contact_crypto_mut()
                .push(PendingContactCrypto {
                    owner_identity_id: watched,
                    contact_id: contact,
                    op: PendingContactCryptoOp::RegisterReceiving,
                    enqueued_at_ms: 0,
                });
        }

        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let drained = iw.dashpay().drain_pending_contact_crypto(&provider).await;
        assert_eq!(
            drained, 1,
            "the drain must snapshot the out-of-wallet bucket and process its entry"
        );

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert!(
            info.identity_manager
                .managed_identity(&watched)
                .expect("watched resident")
                .dashpay()
                .pending_contact_crypto
                .is_empty(),
            "the out-of-wallet identity's queue must be cleared after the drain"
        );
    }

    /// A `RegisterExternal` entry the drain cannot complete (here: the owner
    /// isn't wallet-owned, so no HD index → it bails before any network fetch)
    /// must be **left queued**, never dropped or crashed — so a later drain can
    /// retry. Pins the deferral safety of the external op without needing a
    /// configured mock fetch.
    #[tokio::test]
    async fn drain_leaves_register_external_it_cannot_complete() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};
        use crate::wallet::identity::network::contact_requests::ContactCryptoProvider;

        let (manager, persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();
        let owner = Identifier::from([0x11; 32]);
        let contact = Identifier::from([0x22; 32]);

        // Owner is resident but OUT-OF-WALLET (no HD index), so the
        // RegisterExternal drain bails before reaching the fetch, leaving the
        // entry queued.
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_out_of_wallet_identity(
                    bare_identity([0x11; 32]),
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add out-of-wallet owner");
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("owner resident")
                .dashpay_pending_contact_crypto_mut()
                .push(PendingContactCrypto {
                    owner_identity_id: owner,
                    contact_id: contact,
                    op: PendingContactCryptoOp::RegisterExternal {
                        encrypted_public_key: vec![7u8; 96],
                        our_decryption_key_index: 0,
                        contact_encryption_key_index: 0,
                    },
                    enqueued_at_ms: 0,
                });
        }

        struct UnusedProvider;
        #[async_trait::async_trait]
        impl ContactCryptoProvider for UnusedProvider {
            async fn receiving_xpub(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
            ) -> Result<key_wallet::bip32::ExtendedPubKey, crate::error::PlatformWalletError>
            {
                Err(crate::error::PlatformWalletError::InvalidIdentityData(
                    "unused in this test".to_string(),
                ))
            }
            async fn ecdh_shared_secret(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
                _peer: &dashcore::secp256k1::PublicKey,
            ) -> Result<zeroize::Zeroizing<[u8; 32]>, crate::error::PlatformWalletError>
            {
                Ok(zeroize::Zeroizing::new([0u8; 32]))
            }
            async fn export_auto_accept_private_key(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
            ) -> Result<dashcore::secp256k1::SecretKey, crate::error::PlatformWalletError>
            {
                unimplemented!("auto-accept QR is a send-path method, not exercised by the drain")
            }
            async fn export_invitation_private_key(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
            ) -> Result<dashcore::secp256k1::SecretKey, crate::error::PlatformWalletError>
            {
                unimplemented!(
                    "invitation create is a send-path method, not exercised by the drain"
                )
            }
            async fn account_reference(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
                _compact_xpub: &[u8],
                _account_index: u32,
                _version: u32,
            ) -> Result<u32, crate::error::PlatformWalletError> {
                unimplemented!("accountReference is a send-path method, not exercised by the drain")
            }
            async fn unmask_account_reference(
                &self,
                _path: &key_wallet::bip32::DerivationPath,
                _compact_xpub: &[u8],
                _account_reference: u32,
            ) -> Result<(u32, u32), crate::error::PlatformWalletError> {
                unimplemented!("accountReference is a send-path method, not exercised by the drain")
            }
            async fn contact_info_seal(
                &self,
                _root_path: &key_wallet::bip32::DerivationPath,
                _derivation_index: u32,
                _contact_id: &[u8; 32],
                _private_data_plaintext: &[u8],
                _private_data_iv: &[u8; 16],
            ) -> Result<
                crate::wallet::identity::network::ContactInfoSealed,
                crate::error::PlatformWalletError,
            > {
                unimplemented!("contactInfo is not exercised by this drain test")
            }
            async fn contact_info_open(
                &self,
                _root_path: &key_wallet::bip32::DerivationPath,
                _derivation_index: u32,
                _enc_to_user_id: &[u8; 32],
                _private_data_blob: &[u8],
            ) -> Result<
                crate::wallet::identity::network::ContactInfoOpened,
                crate::error::PlatformWalletError,
            > {
                unimplemented!("contactInfo is not exercised by this drain test")
            }
        }

        let drained = iw
            .dashpay()
            .drain_pending_contact_crypto(&UnusedProvider)
            .await;
        assert_eq!(
            drained, 0,
            "an un-completable RegisterExternal entry must not be counted as drained"
        );
        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert_eq!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("owner resident")
                .dashpay()
                .pending_contact_crypto
                .len(),
            1,
            "the deferred entry must remain in the queue for a later drain"
        );
    }

    /// Confused-deputy guard (security MUST-FIX): the `ContactInfoDecrypt` drain
    /// refuses an identity this wallet does not own — it errors at the ownership
    /// check BEFORE any fetch/decrypt, so a poisoned/mis-attributed queue entry
    /// can never drive a decrypt under the wrong identity.
    #[tokio::test]
    async fn contact_info_decrypt_drain_rejects_non_owned_identity() {
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;
        let (manager, _persister, wallet_id) = make_watch_only_wallet().await;
        let iw = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = iw.identity();
        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("mnemonic")
            .to_seed("");
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let not_ours = Identifier::from([0x99; 32]);
        let err = iw
            .dashpay()
            .drain_contact_info_decrypt(&not_ours, &provider)
            .await
            .expect_err("a non-owned identity must be rejected");
        assert!(
            matches!(err, PlatformWalletError::InvalidIdentityData(_)),
            "confused-deputy guard must reject a non-owned identity, got {err:?}"
        );
    }

    /// The signerless sweep on a seedless wallet ENQUEUES a per-owner
    /// `ContactInfoDecrypt` op (no inline decrypt, no network) for the drain to
    /// complete when a signer is available.
    #[tokio::test]
    async fn seedless_sweep_enqueues_contact_info_decrypt() {
        use crate::changeset::PendingContactCryptoKind;
        let (manager, persister, wallet_id) = make_watch_only_wallet().await;
        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet.identity();
        let owner = Identifier::from([0x11; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
        }
        let applied = iw.dashpay().sync_contact_infos().await.expect("sync");
        assert_eq!(
            applied, 0,
            "seedless sweep applies nothing inline — it defers"
        );
        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        assert!(
            info.identity_manager
                .managed_identity(&owner)
                .expect("owner resident")
                .dashpay()
                .pending_contact_crypto
                .iter()
                .any(|e| e.op.kind() == PendingContactCryptoKind::ContactInfoDecrypt),
            "the seedless sweep must enqueue a ContactInfoDecrypt op for the owner"
        );
    }

    // -----------------------------------------------------------------------
    // send_payment drains the deferred contact-crypto queue first
    // -----------------------------------------------------------------------
    //
    // The original sender's external-account build is enqueued by the
    // signerless sweep and completed by `drain_pending_contact_crypto`. The
    // drain runs at the start of `send_payment` (with the signer-backed
    // provider) so the external account is built before the send resolves it —
    // otherwise the first `send_payment` after establishing a contact fails the
    // external-account lookup even though the build is queued and ready.
    //
    // Coverage is split across two tests because the mock SDK's
    // `expect_fetch::<Identity>` is single-shot and the `RegisterExternal`
    // drain branch fetches the contact identity through it — a second fetch in
    // a run returns a stale identity whose key yields a different ECDH secret,
    // so the in-process drain of a `RegisterExternal` op cannot be made to
    // build the external account deterministically under the mock.
    //   * `send_payment_runs_pending_contact_crypto_drain` pins the mechanism:
    //     `send_payment` runs the drain before the external lookup (verified
    //     with a `RegisterReceiving` op, which the faithful `SeedCryptoProvider`
    //     completes with no fetch).
    //   * `send_payment_passes_external_lookup_once_account_built` pins the
    //     complementary half: once the external account exists, the lookup
    //     passes and the send proceeds past it.
    // The full drain-then-send of a queued `RegisterExternal` is exercised
    // end-to-end by the live DashPay e2e flow.

    /// `send_payment` drains the deferred contact-crypto queue before it
    /// resolves the external account. A `RegisterExternal` op can't be built
    /// from under the single-shot mock fetch (see the module comment above), so
    /// the drain's invocation is pinned with a `RegisterReceiving` op the
    /// `SeedCryptoProvider` completes with no fetch: after a (still-failing)
    /// `send_payment`, that queued op is drained. Without the send-path drain
    /// the op stays queued.
    #[tokio::test]
    async fn send_payment_runs_pending_contact_crypto_drain() {
        use crate::changeset::{PendingContactCrypto, PendingContactCryptoOp};
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;
        use key_wallet::account::account_collection::DashpayAccountKey;

        let (manager, persister, wallet_id) = make_watch_only_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner = Identifier::from([0x11; 32]);
        // The contact whose RegisterReceiving op the drain should complete.
        let queued_contact = Identifier::from([0x33; 32]);
        // The (different) contact we try to pay — it has no external account,
        // so the send fails AFTER the drain has run.
        let pay_contact = Identifier::from([0x22; 32]);

        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");

        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
            // A RegisterReceiving op the provider-only drain can complete.
            info.identity_manager
                .managed_identity_mut(&owner)
                .expect("owner resident")
                .dashpay_pending_contact_crypto_mut()
                .push(PendingContactCrypto {
                    owner_identity_id: owner,
                    contact_id: queued_contact,
                    op: PendingContactCryptoOp::RegisterReceiving,
                    enqueued_at_ms: 0,
                });
        }

        // A signer + provider derived from the same test seed — the production
        // posture (Keychain signer present on the send path).
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let signer = SeedSigner::new(seed, Network::Testnet);

        // The send fails (no external account for `pay_contact`), but the drain
        // it runs first must have completed the queued RegisterReceiving op.
        let result = iw
            .dashpay()
            .send_payment(&owner, &pay_contact, 10_000, None, &signer, &provider)
            .await;
        assert!(
            result.is_err(),
            "the send must still fail for an unbuilt external account"
        );

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        let receiving_key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner.to_buffer(),
            friend_identity_id: queued_contact.to_buffer(),
        };
        assert!(
            info.core_wallet
                .accounts
                .dashpay_receival_accounts
                .contains_key(&receiving_key),
            "send_payment must drain the pending contact-crypto queue before \
             failing — the queued RegisterReceiving op stays unbuilt without the drain"
        );
        assert!(
            !info
                .identity_manager
                .managed_identity(&owner)
                .expect("owner resident")
                .dashpay()
                .pending_contact_crypto
                .iter()
                .any(|e| e.contact_id == queued_contact),
            "the drained RegisterReceiving entry must be cleared from the queue"
        );
    }

    /// Once the external account IS built, `send_payment` gets PAST the
    /// external-account lookup: it no longer returns the
    /// "No DashpayExternalAccount found" error and instead fails later, at
    /// funding/transaction build (the seedless test wallet has no UTXOs). Pins
    /// the precondition the drain clears.
    #[tokio::test]
    async fn send_payment_passes_external_lookup_once_account_built() {
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner_id = Identifier::from([0x11; 32]);
        let contact_id = Identifier::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
        }

        // Build the external account via the faithful precomputed-shared-key
        // path (what the drain ultimately calls) so the send's lookup finds it.
        let shared_key = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let compact = {
            let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
                .expect("mnemonic")
                .to_seed("");
            let w = key_wallet::wallet::Wallet::from_seed_bytes(
                seed,
                Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("seed wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                &w,
                Network::Testnet,
                0,
                &owner_id,
                &contact_id,
            )
            .expect("derive a valid compact xpub")
            .compact
            .to_bytes()
        };
        let encrypted =
            platform_encryption::encrypt_extended_public_key(&shared_key, &iv, &compact);
        let contact = bare_identity([0x22; 32]);
        iw.dashpay()
            .register_external_contact_account(
                &owner_id,
                &contact,
                &encrypted,
                zeroize::Zeroizing::new(shared_key),
            )
            .await
            .expect("register external account");

        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let signer = SeedSigner::new(seed, Network::Testnet);

        let err = iw
            .dashpay()
            .send_payment(&owner_id, &contact_id, 10_000, None, &signer, &provider)
            .await
            .expect_err("seedless test wallet has no UTXOs, so the build must fail");

        // The point: the external-account lookup PASSED. The error is a later
        // funding/build failure, NOT the missing-account precondition. A
        // `TransactionBuild` / funding error is the expected post-lookup
        // failure for a wallet with no spendable UTXOs; only an
        // `InvalidIdentityData` carrying the missing-account message would mean
        // the lookup still failed.
        if let PlatformWalletError::InvalidIdentityData(msg) = &err {
            assert!(
                !msg.contains("No DashpayExternalAccount found"),
                "send_payment must get PAST the external-account lookup once \
                 the account is built, got: {msg}"
            );
        }
    }

    /// A failed `build_signed` must return the consumed payment address to
    /// the pool. Without the rollback every failed build (insufficient
    /// funds, a refusing signer) permanently advances the next index by one:
    /// enough failures before one successful payment put that payment past
    /// any gap-limit walk a restore-from-seed can perform, and sent-payment
    /// reconstruction never finds it.
    #[tokio::test]
    async fn send_payment_failed_build_returns_the_address_to_the_pool() {
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner_id = Identifier::from([0x11; 32]);
        let contact_id = Identifier::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
        }

        let shared_key = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let compact = {
            let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
                .expect("mnemonic")
                .to_seed("");
            let w = key_wallet::wallet::Wallet::from_seed_bytes(
                seed,
                Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("seed wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                &w,
                Network::Testnet,
                0,
                &owner_id,
                &contact_id,
            )
            .expect("derive a valid compact xpub")
            .compact
            .to_bytes()
        };
        let encrypted =
            platform_encryption::encrypt_extended_public_key(&shared_key, &iv, &compact);
        let contact = bare_identity([0x22; 32]);
        iw.dashpay()
            .register_external_contact_account(
                &owner_id,
                &contact,
                &encrypted,
                zeroize::Zeroizing::new(shared_key),
            )
            .await
            .expect("register external account");

        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let signer = SeedSigner::new(seed, Network::Testnet);

        // Two failed builds in a row: without rollback each one consumes an
        // index and the pool's used range marches forward off-chain.
        for attempt in 1..=2 {
            iw.dashpay()
                .send_payment(&owner_id, &contact_id, 10_000, None, &signer, &provider)
                .await
                .expect_err(
                    "seedless test wallet has no UTXOs, so the build must fail \
                     (attempt {attempt})",
                );
            let _ = attempt;
        }

        let wm = iw.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("info");
        let key = DashpayAccountKey {
            index: 0,
            user_identity_id: owner_id.to_buffer(),
            friend_identity_id: contact_id.to_buffer(),
        };
        let account = info
            .core_wallet
            .accounts
            .dashpay_external_accounts
            .get(&key)
            .expect("external account present");
        let pools = account.managed_account_type().address_pools();
        let pool = pools.first().expect("external pool");
        assert!(
            pool.used_indices.is_empty(),
            "a failed build must not leave any address consumed, found {:?}",
            pool.used_indices
        );
        assert_eq!(
            pool.highest_used, None,
            "no on-chain use happened, so the pool's used high-water must stay unset"
        );
    }

    /// The `send_payment` used-flag flip persist must run only AFTER the
    /// wallet-manager write guard is released (and before the broadcast).
    ///
    /// The host persistence callback is synchronous, so holding the manager's
    /// write lock across it would stall every other wallet accessor for the
    /// host write's duration, and a host store that re-entered a manager API
    /// would deadlock the non-reentrant lock. This pins that the used-flip
    /// changeset is persisted AND that the write lock was already released
    /// when it was — a `try_read` on the shared manager succeeds iff no writer
    /// holds it. Against the pre-fix ordering (store inside the write-guarded
    /// block) the lock-released assertion fails; with the persist deferred
    /// until after the guard drops it holds.
    #[tokio::test]
    async fn send_payment_persists_external_pool_used_flip_after_releasing_lock() {
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;
        use key_wallet::account::AccountType;

        // Probe persister: on the used-flip store (the changeset carrying an
        // external account's address pools), records whether the shared
        // wallet-manager write lock was already released — a `try_read`
        // succeeds iff no writer holds it. A `Weak` back-reference avoids a
        // persister <-> manager cycle.
        type ManagerLock = tokio::sync::RwLock<
            key_wallet_manager::WalletManager<crate::wallet::platform_wallet::PlatformWalletInfo>,
        >;
        struct LockProbePersister {
            manager: Mutex<Option<std::sync::Weak<ManagerLock>>>,
            /// `Some(true)`: an external-pool store was seen with the write
            /// lock released; `Some(false)`: seen but the lock was still held;
            /// `None`: never seen.
            external_pool_store_unlocked: Mutex<Option<bool>>,
        }

        impl PlatformWalletPersistence for LockProbePersister {
            fn store(
                &self,
                _wallet_id: WalletId,
                changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                let carries_external_pool = changeset
                    .account_address_pools
                    .iter()
                    .any(|e| matches!(e.account_type, AccountType::DashpayExternalAccount { .. }));
                if carries_external_pool {
                    let unlocked = self
                        .manager
                        .lock()
                        .unwrap()
                        .as_ref()
                        .and_then(|w| w.upgrade())
                        .map(|m| m.try_read().is_ok())
                        .unwrap_or(false);
                    *self.external_pool_store_unlocked.lock().unwrap() = Some(unlocked);
                }
                Ok(())
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let probe = Arc::new(LockProbePersister {
            manager: Mutex::new(None),
            external_pool_store_unlocked: Mutex::new(None),
        });
        let (manager, wallet_id) = make_wallet_with(Arc::clone(&probe)).await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();
        // Point the probe at the shared manager lock the send will contend on.
        *probe.manager.lock().unwrap() = Some(Arc::downgrade(&iw.wallet_manager));

        let owner_id = Identifier::from([0x11; 32]);
        let contact_id = Identifier::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&probe) as _),
                )
                .expect("add owner");
        }

        // Build the external account (same faithful precomputed-shared-key
        // path as `send_payment_passes_external_lookup_once_account_built`).
        let shared_key = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let compact = {
            let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
                .expect("mnemonic")
                .to_seed("");
            let w = key_wallet::wallet::Wallet::from_seed_bytes(
                seed,
                Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("seed wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                &w,
                Network::Testnet,
                0,
                &owner_id,
                &contact_id,
            )
            .expect("derive a valid compact xpub")
            .compact
            .to_bytes()
        };
        let encrypted =
            platform_encryption::encrypt_extended_public_key(&shared_key, &iv, &compact);
        let contact = bare_identity([0x22; 32]);
        iw.dashpay()
            .register_external_contact_account(
                &owner_id,
                &contact,
                &encrypted,
                zeroize::Zeroizing::new(shared_key),
            )
            .await
            .expect("register external account");

        // Isolate the assertion to the send path: forget any external-pool
        // store the registration round may have made (that one runs under the
        // registration write guard).
        *probe.external_pool_store_unlocked.lock().unwrap() = None;

        // Fund BIP-44 account 0 so the send builds, signs, and broadcasts —
        // reaching the used-flip persist, which now fires only on the path to
        // broadcast (a funding-build failure returns before it).
        fund_bip44_account_0(&manager, wallet_id, 0xA1, 60_000).await;

        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let signer = SeedSigner::new(seed, Network::Testnet);

        let iw_send = with_accepting_broadcaster(iw);
        iw_send
            .dashpay()
            .send_payment(&owner_id, &contact_id, 50_000, None, &signer, &provider)
            .await
            .expect("funded + signable send must succeed through the accepting broadcaster");

        // The used-flip changeset was persisted (Some(_)) AND the store ran
        // after the write guard was released (Some(true)). Some(false) means
        // it still ran under the held write guard (the pre-fix ordering);
        // None means the flip was never persisted.
        let observed = *probe.external_pool_store_unlocked.lock().unwrap();
        assert_eq!(
            observed,
            Some(true),
            "send_payment must persist the external-account used flip AFTER \
             releasing the wallet-manager write lock (observed: {observed:?})"
        );
    }

    /// Broadcaster stub that accepts every transaction, so a send-path test
    /// can reach `send_payment`'s return value. The production `identity()`
    /// pins [`SpvBroadcaster`], whose runtime is never started in tests
    /// (`broadcast` returns `Rejected { "SPV client not started" }`); the
    /// `IdentityWallet<B>` broadcaster generic is the sanctioned injection
    /// seam, exercised here with the same-crate `pub(crate)` fields. The
    /// build + sign + fee computation all run for real — only the network
    /// transport is stubbed.
    struct AcceptingBroadcaster;

    #[async_trait::async_trait]
    impl crate::broadcaster::TransactionBroadcaster for AcceptingBroadcaster {
        async fn broadcast(
            &self,
            transaction: &dashcore::Transaction,
        ) -> Result<dashcore::Txid, crate::broadcaster::BroadcastError> {
            Ok(transaction.txid())
        }
    }

    /// Re-specialize a live `IdentityWallet<SpvBroadcaster>` onto the
    /// accepting broadcaster, sharing every other Arc (wallet manager, SDK,
    /// asset locks, persister, sdk_writer) so the two handles operate on the
    /// same wallet state.
    fn with_accepting_broadcaster(
        real: &crate::wallet::identity::IdentityWallet<crate::broadcaster::SpvBroadcaster>,
    ) -> crate::wallet::identity::IdentityWallet<AcceptingBroadcaster> {
        crate::wallet::identity::IdentityWallet {
            sdk: Arc::clone(&real.sdk),
            wallet_manager: Arc::clone(&real.wallet_manager),
            wallet_id: real.wallet_id,
            asset_locks: Arc::clone(&real.asset_locks),
            persister: real.persister.clone(),
            broadcaster: Arc::new(AcceptingBroadcaster),
            sdk_writer: Arc::clone(&real.sdk_writer),
        }
    }

    /// Plant a single spendable UTXO of `value_duffs` on BIP-44 account 0's
    /// first pool address (a real derived address, so its derivation path is
    /// resolvable and [`SeedSigner`] can sign the funding input).
    async fn fund_bip44_account_0<P: PlatformWalletPersistence + 'static>(
        manager: &Arc<PlatformWalletManager<P>>,
        wallet_id: WalletId,
        txid_byte: u8,
        value_duffs: u64,
    ) {
        use dashcore::hashes::Hash;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        let wallet = manager
            .get_wallet(&wallet_id)
            .await
            .expect("wallet registered");
        let iw = wallet.identity();
        let mut wm = iw.wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("BIP-44 managed account 0");
        let address_info = account
            .managed_account_type()
            .address_pools()
            .first()
            .expect("BIP-44 account has an address pool")
            .addresses
            .values()
            .next()
            .expect("pool has at least one derived address")
            .clone();
        let txid = dashcore::Txid::from_slice(&[txid_byte; 32]).expect("txid");
        let outpoint = dashcore::OutPoint { txid, vout: 0 };
        account.utxos.insert(
            outpoint,
            key_wallet::Utxo {
                outpoint,
                txout: dashcore::TxOut {
                    value: value_duffs,
                    script_pubkey: address_info.script_pubkey.clone(),
                },
                address: address_info.address.clone(),
                height: 100,
                is_coinbase: false,
                is_confirmed: true,
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            },
        );
    }

    /// Create a testnet wallet, add the sender identity, and register the
    /// external contact account for `(owner, contact)` via the faithful
    /// precomputed-shared-key path — the shared setup the two full-send fee
    /// tests build on (mirrors
    /// `send_payment_passes_external_lookup_once_account_built` up to the send).
    async fn register_sender_and_external_account() -> (
        Arc<PlatformWalletManager<RecordingPersister>>,
        WalletId,
        Identifier,
        Identifier,
    ) {
        let (manager, persister, wallet_id) = make_wallet().await;
        let wallet_arc = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = wallet_arc.identity();

        let owner_id = Identifier::from([0x11; 32]);
        let contact_id = Identifier::from([0x22; 32]);
        {
            let mut wm = iw.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("info");
            info.identity_manager
                .add_identity(
                    bare_identity([0x11; 32]),
                    0,
                    wallet_id,
                    &WalletPersister::new(wallet_id, Arc::clone(&persister) as _),
                )
                .expect("add owner");
        }

        let shared_key = [0x55u8; 32];
        let iv = [0x11u8; 16];
        let compact = {
            let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
                .expect("mnemonic")
                .to_seed("");
            let w = key_wallet::wallet::Wallet::from_seed_bytes(
                seed,
                Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("seed wallet");
            crate::wallet::identity::crypto::dip14::derive_contact_xpub(
                &w,
                Network::Testnet,
                0,
                &owner_id,
                &contact_id,
            )
            .expect("derive a valid compact xpub")
            .compact
            .to_bytes()
        };
        let encrypted =
            platform_encryption::encrypt_extended_public_key(&shared_key, &iv, &compact);
        let contact = bare_identity([0x22; 32]);
        iw.dashpay()
            .register_external_contact_account(
                &owner_id,
                &contact,
                &encrypted,
                zeroize::Zeroizing::new(shared_key),
            )
            .await
            .expect("register external account");

        (manager, wallet_id, owner_id, contact_id)
    }

    /// A fully-successful `send_payment` whose exact change would be dust
    /// (≤ 546 duffs) must fold that remainder into the reported fee instead
    /// of dropping it silently. With one funded UTXO of `V` and a payment of
    /// `A` engineered so the change is dust, the builder emits a single
    /// output (no change) and the returned fee is `V − A` — the exact
    /// Σ(inputs) − Σ(outputs), strictly larger than the builder's
    /// size-based fee by the dropped dust. Pins the bug this PR fixed.
    #[tokio::test]
    async fn send_payment_reports_exact_fee_folding_dropped_dust_change() {
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, wallet_id, owner_id, contact_id) =
            register_sender_and_external_account().await;

        // One UTXO of V = A + 526. The size-based fee for 1 input + 1 output
        // is ~192 duffs and the coin selector reserves 226 (it sizes with a
        // phantom change output); the resulting change (~300) is below the
        // 546-duff dust threshold, so the builder drops it and emits ONLY the
        // payment output. The real fee is therefore V − A = 526, NOT the
        // ~192-duff size fee.
        let amount = 50_000u64;
        let funded = amount + 526;
        fund_bip44_account_0(&manager, wallet_id, 0xA1, funded).await;

        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let signer = SeedSigner::new(seed, Network::Testnet);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = with_accepting_broadcaster(wallet.identity());

        let (_txid, entry, fee) = iw
            .dashpay()
            .send_payment(&owner_id, &contact_id, amount, None, &signer, &provider)
            .await
            .expect("funded + signable send must succeed through the accepting broadcaster");

        // Exact fee = Σ(selected input values) − Σ(output values). The single
        // funded UTXO is the only input, and because the dust change was
        // dropped the only output is the payment itself, so Σout == amount.
        assert_eq!(
            fee,
            funded - amount,
            "the reported fee must be Σin − Σout, folding the dropped dust \
             change into the fee (V − A)"
        );
        // fee == funded − amount can ONLY hold when Σout == amount, i.e. no
        // change output was emitted: any change output would make Σout larger
        // and the fee strictly smaller. So this equality alone proves the
        // dust-drop.
        assert_eq!(
            entry.amount_duffs, amount,
            "recorded payment amount is the send amount"
        );
    }

    /// The control case: when the change clears the dust threshold the
    /// builder DOES emit a change output, and Σ(inputs) − Σ(outputs) then
    /// equals the ordinary size-based fee — the change output absorbs the
    /// remainder, so nothing is folded in. Confirms the fee computation is
    /// not blindly inflating every send.
    #[tokio::test]
    async fn send_payment_reports_size_fee_when_change_is_emitted() {
        use crate::wallet::identity::network::contact_requests::SeedCryptoProvider;

        let (manager, wallet_id, owner_id, contact_id) =
            register_sender_and_external_account().await;

        // One UTXO of V = A + 1226. Change = V − A − size_fee = 1226 − 226 =
        // 1000 > 546, so the builder emits a 1000-duff change output. Σout =
        // A + 1000, so the fee is V − Σout = 226 — the size-based fee for a
        // 1-input, 2-output type-3 tx at 1 duff/byte (base 78 + 148 input),
        // with NO dust folded in.
        let amount = 50_000u64;
        let funded = amount + 1226;
        fund_bip44_account_0(&manager, wallet_id, 0xB2, funded).await;

        let seed = Mnemonic::from_phrase(TEST_MNEMONIC, Language::English)
            .expect("valid mnemonic")
            .to_seed("");
        let provider = SeedCryptoProvider::from_seed(seed, Network::Testnet);
        let signer = SeedSigner::new(seed, Network::Testnet);

        let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
        let iw = with_accepting_broadcaster(wallet.identity());

        let (_txid, _entry, fee) = iw
            .dashpay()
            .send_payment(&owner_id, &contact_id, amount, None, &signer, &provider)
            .await
            .expect("funded + signable send must succeed through the accepting broadcaster");

        // The change output absorbed the remainder: the fee is the plain
        // size-based fee, NOT the full V − A.
        assert!(
            fee < funded - amount,
            "with a change output emitted the fee must be smaller than V − A \
             (the change absorbs the remainder), got fee={fee}, V−A={}",
            funded - amount
        );
        assert_eq!(
            fee, 226,
            "Σin − Σout equals the size-based fee for a 1-input, 2-output \
             type-3 tx at 1 duff/byte when change is emitted"
        );
    }
}
