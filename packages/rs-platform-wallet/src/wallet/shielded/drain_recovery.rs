//! Restart recovery for a drain's exact Type18 attempts. Completion requires
//! both authenticated exact output sightings and proof-verified full consumption
//! of the original outpoint. Neither evidence alone establishes completion.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::time::Duration;

use dash_sdk::platform::shielded::{try_decrypt_note, try_recover_outgoing_note};
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::query_types::ShieldedEncryptedNote;
use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
use dashcore::hashes::{sha256d, Hash};
use dashcore::OutPoint;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::identity::state_transition::AssetLockProved;
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable, Signable};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::shield_from_asset_lock_transition::accessors::ShieldFromAssetLockTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionSingleSigned};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::error::{
    as_asset_lock_proof_cl_height_too_low, is_asset_lock_proof_transaction_height_invalid,
    is_instant_lock_proof_invalid, PlatformWalletError,
};
use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
use crate::wallet::PlatformWallet;

use super::keys::AccountViewingKeys;
use super::store::{ShieldedStore, SubwalletId};
use super::NetworkShieldedCoordinator;

fn invalid(reason: impl std::fmt::Display) -> PlatformWalletError {
    PlatformWalletError::Persistence(format!("invalid wallet drain recovery evidence: {reason}"))
}

/// Recover the complete nonzero output set from the locally signed request,
/// not from arbitrary activity metadata. The asset-lock signature authenticates
/// the outpoint and bundle together. Missing/wrong keys cannot yield success.
fn attempt_outputs(
    lock: &TrackedAssetLock,
    bytes: &[u8],
    keys: &BTreeMap<u32, AccountViewingKeys>,
) -> Result<BTreeSet<(u32, [u8; 32])>, PlatformWalletError> {
    if lock.operation_id.is_none()
        || lock.funding_type != AssetLockFundingType::AssetLockShieldedAddressTopUp
        || lock.transaction.txid() != lock.out_point.txid
    {
        return Err(invalid("wrong operation, funding role or transaction"));
    }
    let st = StateTransition::deserialize_from_bytes_untrusted(bytes).map_err(invalid)?;
    let StateTransition::ShieldFromAssetLock(transition) = &st else {
        return Err(invalid("expected a signed Type18 transition"));
    };
    if transition.asset_lock_proof().out_point() != Some(lock.out_point) {
        return Err(invalid(
            "attempt belongs to a different asset-lock outpoint",
        ));
    }
    let Some(TransactionPayload::AssetLockPayloadType(payload)) =
        &lock.transaction.special_transaction_payload
    else {
        return Err(invalid("tracked transaction has no asset-lock payload"));
    };
    let credit_output = payload
        .credit_outputs
        .get(lock.out_point.vout as usize)
        .ok_or_else(|| invalid("asset-lock credit output is missing"))?;
    if credit_output.value != lock.amount
        || transition.value_balance() == 0
        || lock
            .amount
            .checked_mul(CREDITS_PER_DUFF)
            .is_none_or(|amount| transition.value_balance() > amount)
    {
        return Err(invalid("attempt value does not match the tracked lock"));
    }
    let public_key_hash = credit_output
        .script_pubkey
        .p2pkh_public_key_hash_bytes()
        .ok_or_else(|| invalid("asset-lock credit output is not P2PKH"))?;
    let digest = sha256d::Hash::hash(&st.signable_bytes().map_err(invalid)?);
    dashcore::signer::verify_hash_signature(
        digest.as_byte_array(),
        transition.signature().as_slice(),
        public_key_hash,
    )
    .map_err(invalid)?;

    let mut outputs = BTreeSet::new();
    let mut recovered_value = 0u64;
    for action in transition.actions() {
        let wire = ShieldedEncryptedNote {
            cmx: action.cmx.to_vec(),
            nullifier: action.nullifier.to_vec(),
            cv_net: action.cv_net.to_vec(),
            encrypted_note: action.encrypted_note.clone(),
        };
        for (account, keys) in keys {
            let note = try_decrypt_note(&keys.prepared_ivk, &wire)
                .map(|(note, _)| note)
                .or_else(|| {
                    try_recover_outgoing_note(&keys.outgoing_viewing_key, &wire)
                        .map(|(note, _, _)| note)
                });
            if let Some(note) = note {
                if note.value().inner() > 0 {
                    if !outputs.insert((*account, action.cmx)) {
                        return Err(invalid("duplicate output commitment"));
                    }
                    recovered_value = recovered_value
                        .checked_add(note.value().inner())
                        .ok_or_else(|| invalid("output value overflow"))?;
                }
                break;
            }
        }
    }
    if outputs.is_empty() || recovered_value != transition.value_balance() {
        return Err(invalid(
            "viewing keys cannot recover the complete funded output set",
        ));
    }
    Ok(outputs)
}

/// Replace only scan-derived aggregates that contain this exact attempt's
/// outputs. Reclassify the remaining scan data so unrelated same-anchor
/// receipts retain their own history row instead of being lost or counted twice.
fn drain_history_changeset(
    store: &impl ShieldedStore,
    wallet_id: [u8; 32],
    keys: &BTreeMap<u32, AccountViewingKeys>,
    actions: &[dpp::shielded::SerializedAction],
    outputs: &BTreeSet<(u32, [u8; 32])>,
) -> Result<crate::changeset::ShieldedChangeSet, PlatformWalletError> {
    use super::activity::{
        compute_activity_id, derive_unowned_activity_from_scan_data, ScanDeriveInput,
        ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
    };
    let mut history = crate::changeset::ShieldedChangeSet::default();
    for (account, views) in keys {
        let cmxs: BTreeSet<_> = outputs
            .iter()
            .filter_map(|(owner, cmx)| (owner == account).then_some(*cmx))
            .collect();
        if cmxs.is_empty() {
            continue;
        }
        let id = SubwalletId::new(wallet_id, *account);
        let entry_id = compute_activity_id(&cmxs.iter().copied().collect::<Vec<_>>());
        let mut amount = 0u64;
        for action in actions.iter().filter(|action| cmxs.contains(&action.cmx)) {
            let wire = ShieldedEncryptedNote {
                cmx: action.cmx.to_vec(),
                nullifier: action.nullifier.to_vec(),
                cv_net: action.cv_net.to_vec(),
                encrypted_note: action.encrypted_note.clone(),
            };
            let note = try_decrypt_note(&views.prepared_ivk, &wire)
                .map(|(note, _)| note)
                .or_else(|| {
                    try_recover_outgoing_note(&views.outgoing_viewing_key, &wire)
                        .map(|(note, _, _)| note)
                })
                .ok_or_else(|| invalid("history output no longer recoverable"))?;
            amount = amount
                .checked_add(note.value().inner())
                .ok_or_else(|| invalid("history amount overflow"))?;
        }
        let existing = store.get_activity(id, 0, usize::MAX).map_err(invalid)?;
        let notes = store.get_all_notes(id).map_err(invalid)?;
        let outgoing = store.get_outgoing_notes(id).map_err(invalid)?;
        let mut retained = BTreeMap::new();
        for entry in &existing {
            if entry.id != entry_id && entry.note_cmxs.iter().any(|cmx| cmxs.contains(cmx)) {
                if entry.created_at_ms != 0
                    || entry.block_height.is_some()
                    || !matches!(
                        entry.kind,
                        ShieldedActivityKind::Received
                            | ShieldedActivityKind::Sent
                            | ShieldedActivityKind::ShieldedSpend
                    )
                {
                    return Err(invalid("exact drain overlaps another authored activity"));
                }
                history.remove_activity_entry(id, entry.id);
            } else {
                retained.extend(entry.note_cmxs.iter().map(|cmx| (*cmx, entry.id)));
            }
        }
        let canonical = ShieldedActivityEntry {
            id: entry_id,
            kind: ShieldedActivityKind::ShieldFromAssetLock,
            direction: ShieldedDirection::In,
            amount,
            fee: None,
            counterparty: None,
            memo: None,
            block_height: None,
            status: ShieldedActivityStatus::Confirmed,
            // Recovery does not know the original transaction's wall-clock time.
            created_at_ms: existing
                .iter()
                .find(|entry| entry.id == entry_id)
                .map_or(0, |entry| entry.created_at_ms),
            min_note_position: notes
                .iter()
                .filter(|note| cmxs.contains(&note.cmx))
                .map(|note| note.position)
                .min(),
            note_cmxs: cmxs.iter().copied().collect(),
            spent_nullifiers: vec![],
        };
        retained.extend(cmxs.iter().map(|cmx| (*cmx, entry_id)));
        let own_addresses = outgoing
            .iter()
            .filter(|note| super::coordinator::is_own_orchard_recipient(views, &note.recipient))
            .map(|note| note.recipient.clone())
            .collect();
        let input = ScanDeriveInput {
            notes,
            outgoing,
            own_addresses,
        };
        let derived = derive_unowned_activity_from_scan_data(&input, &retained);
        for entry in derived.new_entries {
            history.record_activity_entry(id, entry);
        }
        history.record_activity_entry(id, canonical);
    }
    Ok(history)
}

impl PlatformWallet {
    /// Replay the newest byte-identical attempt first. A recognized proof
    /// freshness rejection permits the normal same-outpoint proof rebuild;
    /// every other verdict remains ambiguous until exact outputs are observed.
    /// Returns true when an existing attempt still awaits authenticated scan.
    pub(super) async fn replay_wallet_drain_attempt(
        &self,
        out_point: OutPoint,
        settings: Option<PutSettings>,
    ) -> Result<bool, PlatformWalletError> {
        let Some(lock) = self
            .asset_locks
            .list_tracked_locks()
            .await
            .into_iter()
            .find(|lock| lock.out_point == out_point && lock.operation_id.is_some())
        else {
            return Ok(false);
        };
        let Some(bytes) = lock.shielded_funding_attempts.last() else {
            return Ok(false);
        };
        let keys = self
            .shielded_keys
            .read()
            .await
            .clone()
            .ok_or_else(|| invalid("shielded viewing keys are not bound"))?;
        attempt_outputs(&lock, bytes, &keys)?;
        // Also retries a prior failed store/flush before any network effect.
        self.asset_locks
            .persist_wallet_drain_attempt(out_point, bytes.clone())
            .await?;
        let st = StateTransition::deserialize_from_bytes_untrusted(bytes).map_err(invalid)?;
        let StateTransition::ShieldFromAssetLock(transition) = &st else {
            return Err(invalid("expected a signed Type18 transition"));
        };
        // Preserve historical evidence, but do not keep replaying a format the
        // active protocol no longer accepts. The caller already resolved this
        // operation to FromExistingAssetLock, so rebuilding uses the same lock.
        if !transition
            .verify_protocol_version(self.sdk.version().protocol_version)
            .map_err(invalid)?
        {
            return Ok(false);
        }
        if let Err(error) = st.broadcast(&self.sdk, settings).await {
            if is_instant_lock_proof_invalid(&error)
                || as_asset_lock_proof_cl_height_too_low(&error).is_some()
                || is_asset_lock_proof_transaction_height_invalid(&error)
            {
                if let StateTransition::ShieldFromAssetLock(ref transition) = st {
                    self.asset_locks
                        .invalidate_rejected_chain_proof(
                            &out_point,
                            transition.asset_lock_proof(),
                            &error,
                        )
                        .await;
                }
                return Ok(false);
            }
            tracing::info!(%out_point, %error, "exact drain replay remains unresolved until its outputs are observed");
        }
        Ok(true)
    }

    /// Called while holding the wallet's shield single-flight guard.
    pub(super) async fn persist_drain_shielded_attempt(
        &self,
        out_point: OutPoint,
        transition: &StateTransition,
    ) -> Result<(), PlatformWalletError> {
        let Some(lock) = self
            .asset_locks
            .list_tracked_locks()
            .await
            .into_iter()
            .find(|lock| lock.out_point == out_point && lock.operation_id.is_some())
        else {
            return Ok(());
        };
        let bytes = transition.serialize_to_bytes().map_err(invalid)?;
        {
            let keys = self.shielded_keys.read().await;
            let keys = keys
                .as_ref()
                .ok_or_else(|| invalid("shielded viewing keys are not bound"))?;
            attempt_outputs(&lock, &bytes, keys)?;
        }
        self.asset_locks
            .persist_wallet_drain_attempt(out_point, bytes)
            .await
    }

    /// The store's note/OVK sightings come exclusively from authenticated sync.
    /// Keep all attempts, including unsuccessful retries, until and after a
    /// durable consumed tombstone exists. A malformed attempt fails closed.
    pub(super) async fn reconcile_wallet_drain(
        &self,
        coordinator: &NetworkShieldedCoordinator,
        out_point: OutPoint,
    ) -> Result<bool, PlatformWalletError> {
        self.reconcile_wallet_drain_with_consumption_check(
            coordinator,
            out_point,
            |transition| async move {
                // Status is read-only and must not inherit the long submission
                // deadline. Bound the streaming wait as well as each request.
                let timeout = Duration::from_secs(3);
                let mut settings = PutSettings::default();
                settings.request_settings.connect_timeout = Some(timeout);
                settings.request_settings.timeout = Some(timeout);
                settings.request_settings.retries = Some(0);
                settings.wait_timeout = Some(timeout);
                tokio::time::timeout(
                    timeout,
                    transition.wait_for_affected_state::<StateTransitionProofResult>(
                        &self.sdk,
                        Some(settings),
                    ),
                )
                .await
                .map_err(|_| dash_sdk::Error::Generic("drain consumption proof timed out".into()))?
            },
        )
        .await
    }

    async fn reconcile_wallet_drain_with_consumption_check<F, Fut>(
        &self,
        coordinator: &NetworkShieldedCoordinator,
        out_point: OutPoint,
        check_consumption: F,
    ) -> Result<bool, PlatformWalletError>
    where
        F: FnOnce(StateTransition) -> Fut,
        Fut: Future<Output = Result<StateTransitionProofResult, dash_sdk::Error>>,
    {
        let Some(lock) = self
            .asset_locks
            .list_tracked_locks()
            .await
            .into_iter()
            .find(|lock| lock.out_point == out_point && lock.operation_id.is_some())
        else {
            return Ok(false);
        };
        if lock.status == AssetLockStatus::Consumed {
            return Ok(true);
        }
        if lock.shielded_funding_attempts.is_empty() {
            return Ok(false);
        }
        let keys = self
            .shielded_keys
            .read()
            .await
            .clone()
            .ok_or_else(|| invalid("shielded viewing keys are not bound"))?;
        let attempts = lock
            .shielded_funding_attempts
            .iter()
            .map(|bytes| attempt_outputs(&lock, bytes, &keys))
            .collect::<Result<Vec<_>, _>>()?;
        let observed = {
            let store = coordinator.store().read().await;
            let mut observed = BTreeSet::new();
            for account in keys.keys() {
                let id = SubwalletId::new(self.wallet_id(), *account);
                for note in store.get_all_notes(id).map_err(invalid)? {
                    observed.insert((*account, note.cmx));
                }
                for note in store.get_outgoing_notes(id).map_err(invalid)? {
                    observed.insert((*account, note.cmx));
                }
            }
            observed
        };
        let Some((matched_outputs, bytes)) = attempts
            .iter()
            .zip(&lock.shielded_funding_attempts)
            .find(|(outputs, _)| outputs.is_subset(&observed))
        else {
            return Ok(false);
        };
        // The SDK verifies the affected-state proof against this exact signed
        // transition's outpoint, already validated by attempt_outputs above.
        // Orchard commitments alone do not bind an asset-lock outpoint.
        let transition =
            StateTransition::deserialize_from_bytes_untrusted(bytes).map_err(invalid)?;
        let StateTransition::ShieldFromAssetLock(attempt) = &transition else {
            return Err(invalid("expected the matched Type18 attempt"));
        };
        // Every retry randomizes its Orchard bundle. History must describe the
        // persisted attempt whose exact outputs were observed, not the latest
        // submitted attempt, which may never have executed.
        let actions = attempt.actions().to_vec();
        match check_consumption(transition).await {
            Ok(
                StateTransitionProofResult::VerifiedAssetLockConsumed(
                    StoredAssetLockInfo::FullyConsumed,
                )
                | StateTransitionProofResult::VerifiedAssetLockConsumedWithAddressInfos(
                    StoredAssetLockInfo::FullyConsumed,
                    _,
                ),
            ) => {}
            Ok(_) => return Ok(false),
            Err(error) => {
                tracing::info!(%out_point, %error, "drain awaits authenticated full consumption of its original outpoint");
                return Ok(false);
            }
        }
        let operation_id = lock
            .operation_id
            .ok_or_else(|| invalid("operation UUID missing"))?;
        // Serialize with scan derivation through its host persistence round.
        // Build a replacement without mutating the memory cache: if the atomic
        // history+tombstone write fails, a retry still sees the original rows.
        let _checkpoint = coordinator.activity_checkpoint.lock().await;
        let history = {
            let store = coordinator.store().read().await;
            drain_history_changeset(&*store, self.wallet_id(), &keys, &actions, matched_outputs)?
        };
        self.asset_locks
            .complete_wallet_drain(out_point, operation_id, history.clone())
            .await?;
        let mut store = coordinator.store().write().await;
        for (id, removed) in history.activity_removed {
            for entry_id in removed {
                store.remove_activity(id, &entry_id).map_err(invalid)?;
            }
        }
        for (id, entries) in history.activity_entries {
            for entry in entries {
                store.save_activity(id, &entry).map_err(invalid)?;
            }
        }
        tracing::info!(%out_point, "wallet drain completed from authenticated exact outputs and full original-outpoint consumption");
        Ok(true)
    }

    pub(super) async fn reconcile_wallet_drains(
        &self,
        coordinator: &NetworkShieldedCoordinator,
    ) -> Result<(), PlatformWalletError> {
        for lock in self.asset_locks.list_tracked_locks().await {
            if lock.operation_id.is_some() && lock.status != AssetLockStatus::Consumed {
                self.reconcile_wallet_drain(coordinator, lock.out_point)
                    .await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broadcaster::SpvBroadcaster;
    use crate::changeset::{
        AssetLockEntry, ClientStartState, ClientWalletStartState, PersistenceCapabilities,
        PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::PlatformEventManager;
    use crate::spv::SpvRuntime;
    use crate::test_support::funded_wallet_manager;
    use crate::wallet::asset_lock::{AssetLockOperationContext, AssetLockOperationState};
    use crate::wallet::shielded::activity::{
        compute_activity_id, ShieldedActivityKind, ShieldedActivityStatus,
    };
    use crate::wallet::shielded::{FileBackedShieldedStore, OrchardKeySet, ShieldedNote};
    use dashcore::blockdata::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::{Network, ScriptBuf, Transaction, TxOut};
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use dpp::prelude::AssetLockProof;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::version::PlatformVersion;
    use grovedb_commitment_tree::{
        DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, Nullifier, OrchardDomain,
        RandomSeed, Rho, ValueCommitTrapdoor, ValueCommitment,
    };
    use key_wallet::account::StandardAccountType;
    use key_wallet::wallet::ManagedWalletInfo;
    use key_wallet::Wallet;
    use rand::{rngs::OsRng, RngCore};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::Notify;

    fn encrypted_action(keys: &AccountViewingKeys) -> SerializedAction {
        let recipient = keys.default_address;
        let ovk = keys.outgoing_viewing_key.clone();
        let value_credits = 1_000;
        let memo = [0; 36];
        let mut rng = OsRng;

        let (nf, rho) = loop {
            let mut b = [0u8; 32];
            rng.fill_bytes(&mut b);
            if let (Some(nf), Some(rho)) = (
                Nullifier::from_bytes(&b).into_option(),
                Rho::from_bytes(&b).into_option(),
            ) {
                break (nf, rho);
            }
        };
        let rseed = loop {
            let mut b = [0u8; 32];
            rng.fill_bytes(&mut b);
            if let Some(rseed) = RandomSeed::from_bytes(b, &rho).into_option() {
                break rseed;
            }
        };
        let value = NoteValue::from_raw(value_credits);
        let note = Note::from_parts(recipient, value, rho, rseed)
            .into_option()
            .expect("valid note parts");
        let cmx = ExtractedNoteCommitment::from(note.commitment());

        let rcv = loop {
            let mut b = [0u8; 32];
            rng.fill_bytes(&mut b);
            if let Some(rcv) = ValueCommitTrapdoor::from_bytes(b).into_option() {
                break rcv;
            }
        };
        let cv = ValueCommitment::derive(value - NoteValue::from_raw(0), rcv);

        let ne =
            grovedb_commitment_tree::OrchardNoteEncryption::<DashMemo>::new(Some(ovk), note, memo);
        let epk = OrchardDomain::<DashMemo>::epk_bytes(ne.epk());
        let enc = ne.encrypt_note_plaintext();
        let out = ne.encrypt_outgoing_plaintext(&cv, &cmx, &mut rng);

        let mut encrypted_note = Vec::with_capacity(216);
        encrypted_note.extend_from_slice(&epk.0);
        encrypted_note.extend_from_slice(enc.as_ref());
        encrypted_note.extend_from_slice(&out);
        assert_eq!(encrypted_note.len(), 216, "wire note must be 216 bytes");

        SerializedAction {
            cmx: cmx.to_bytes(),
            nullifier: nf.to_bytes(),
            cv_net: cv.to_bytes(),
            rk: [0; 32],
            spend_auth_sig: [0; 64],
            encrypted_note,
        }
    }

    fn evidence_fixture() -> (
        TrackedAssetLock,
        BTreeMap<u32, AccountViewingKeys>,
        StateTransition,
    ) {
        evidence_fixture_at(PlatformVersion::latest())
    }

    fn evidence_fixture_at(
        platform_version: &PlatformVersion,
    ) -> (
        TrackedAssetLock,
        BTreeMap<u32, AccountViewingKeys>,
        StateTransition,
    ) {
        let keys = OrchardKeySet::from_seed(&[0x42; 32], Network::Testnet, 0)
            .unwrap()
            .viewing_keys();
        let secret = dashcore::secp256k1::SecretKey::from_slice(&[0x11; 32]).unwrap();
        let public = dashcore::PublicKey::new(dashcore::secp256k1::PublicKey::from_secret_key(
            &dashcore::secp256k1::Secp256k1::new(),
            &secret,
        ));
        let transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(
                AssetLockPayload {
                    version: 1,
                    credit_outputs: vec![TxOut {
                        value: 10_000,
                        script_pubkey: ScriptBuf::new_p2pkh(&public.pubkey_hash()),
                    }],
                },
            )),
        };
        let out_point = OutPoint::new(transaction.txid(), 0);
        let proof = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 42,
            out_point,
        });
        let actions = vec![encrypted_action(&keys), encrypted_action(&keys)];
        // Actual encrypted outputs and ECDSA signature, with no expensive Halo2
        // proof: admission is outside this test; sightings model verified sync.
        let st = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
            proof.clone(),
            &secret.secret_bytes(),
            actions,
            2_000,
            [0; 32],
            vec![],
            [0; 64],
            None,
            platform_version,
        )
        .unwrap();
        (
            TrackedAssetLock {
                operation_id: Some([0x33; 16]),
                shielded_funding_attempts: Vec::new(),
                out_point,
                transaction,
                account_index: 0,
                funding_type: AssetLockFundingType::AssetLockShieldedAddressTopUp,
                identity_index: 0,
                amount: 10_000,
                status: AssetLockStatus::RecoveredFromChain,
                proof: Some(proof),
            },
            BTreeMap::from([(0, keys)]),
            st,
        )
    }

    struct RecoveryPersistence {
        wallet: Wallet,
        info: ManagedWalletInfo,
        committed: Mutex<BTreeMap<OutPoint, AssetLockEntry>>,
        staged: Mutex<BTreeMap<OutPoint, AssetLockEntry>>,
        fail_store: AtomicBool,
        fail_flush: AtomicBool,
        fail_load: AtomicBool,
        before_store: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
        activity: Mutex<BTreeMap<SubwalletId, crate::changeset::ShieldedSubwalletStartState>>,
        staged_activity: Mutex<Option<crate::changeset::ShieldedChangeSet>>,
        fail_activity: AtomicBool,
        omit_activity_capability: AtomicBool,
    }
    impl PlatformWalletPersistence for RecoveryPersistence {
        fn persistence_capabilities(&self) -> PersistenceCapabilities {
            let bits = PersistenceCapabilities::WALLET_DRAIN.bits();
            PersistenceCapabilities::from_bits_retain(
                if self.omit_activity_capability.load(Ordering::SeqCst) {
                    bits & !PersistenceCapabilities::SHIELDED_ACTIVITY_RECONCILIATION.bits()
                } else {
                    bits
                },
            )
        }
        fn store(
            &self,
            _wallet_id: [u8; 32],
            cs: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if let Some(before_store) = self.before_store.lock().unwrap().as_ref() {
                before_store();
            }
            if self.fail_store.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend("store failed"));
            }
            if cs
                .shielded
                .as_ref()
                .is_some_and(|cs| !cs.activity_entries.is_empty())
                && self.fail_activity.load(Ordering::SeqCst)
            {
                return Err(PersistenceError::backend("activity write failed"));
            }
            *self.staged_activity.lock().unwrap() = cs.shielded;
            if let Some(cs) = cs.asset_locks {
                self.staged.lock().unwrap().extend(cs.asset_locks);
            }
            Ok(())
        }
        fn flush(&self, _wallet_id: [u8; 32]) -> Result<(), PersistenceError> {
            if self.fail_flush.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend("flush failed"));
            }
            if let Some(cs) = self.staged_activity.lock().unwrap().take() {
                let mut activity = self.activity.lock().unwrap();
                for (id, ids) in cs.activity_removed {
                    activity
                        .entry(id)
                        .or_default()
                        .activity
                        .retain(|entry| !ids.contains(&entry.id));
                }
                for (id, entries) in cs.activity_entries {
                    let rows = &mut activity.entry(id).or_default().activity;
                    for entry in entries {
                        rows.retain(|old| old.id != entry.id);
                        rows.push(entry);
                    }
                }
            }
            self.committed
                .lock()
                .unwrap()
                .extend(self.staged.lock().unwrap().clone());
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            if self.fail_load.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend("load failed"));
            }
            let locks = self
                .committed
                .lock()
                .unwrap()
                .iter()
                .map(|(out_point, row)| {
                    (
                        *out_point,
                        TrackedAssetLock {
                            operation_id: row.operation_id,
                            shielded_funding_attempts: row.shielded_funding_attempts.clone(),
                            out_point: *out_point,
                            transaction: row.transaction.clone(),
                            account_index: row.account_index,
                            funding_type: row.funding_type,
                            identity_index: row.identity_index,
                            amount: row.amount_duffs,
                            status: row.status.clone(),
                            proof: row.proof.clone(),
                        },
                    )
                })
                .collect();
            Ok(ClientStartState {
                shielded: crate::changeset::ShieldedSyncStartState {
                    per_subwallet: self.activity.lock().unwrap().clone(),
                    ..Default::default()
                },
                wallets: BTreeMap::from([(
                    self.wallet.compute_wallet_id(),
                    ClientWalletStartState {
                        wallet: self.wallet.clone(),
                        wallet_info: self.info.clone(),
                        identity_manager: Default::default(),
                        unused_asset_locks: BTreeMap::from([(0, locks)]),
                        unconfirmed_outgoing_txs: vec![],
                    },
                )]),
                ..Default::default()
            })
        }
    }

    struct TestDirectory(std::path::PathBuf);
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    async fn wallet_fixture(
        lock: &TrackedAssetLock,
        keys: BTreeMap<u32, AccountViewingKeys>,
    ) -> (
        PlatformWallet,
        Arc<RecoveryPersistence>,
        NetworkShieldedCoordinator,
        TestDirectory,
    ) {
        let (manager, id, generation, _) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let (stored_wallet, info) = {
            let manager = manager.read().await;
            let (wallet, info) = manager.get_wallet_and_info(&id).unwrap();
            (wallet.clone(), info.core_wallet.clone())
        };
        let persistence = Arc::new(RecoveryPersistence {
            wallet: stored_wallet,
            info,
            committed: Mutex::new(BTreeMap::from([(lock.out_point, lock.into())])),
            staged: Mutex::new(BTreeMap::new()),
            fail_store: AtomicBool::new(false),
            fail_flush: AtomicBool::new(false),
            fail_load: AtomicBool::new(false),
            before_store: Mutex::new(None),
            activity: Mutex::new(BTreeMap::new()),
            staged_activity: Mutex::new(None),
            fail_activity: AtomicBool::new(false),
            omit_activity_capability: AtomicBool::new(false),
        });
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_version(PlatformVersion::latest())
                .build()
                .unwrap(),
        );
        let spv = Arc::new(SpvRuntime::new(
            manager.clone(),
            Arc::new(PlatformEventManager::new(vec![])),
        ));
        let wallet = PlatformWallet::new(
            sdk.clone(),
            id,
            manager.clone(),
            generation,
            Arc::new(Notify::new()),
            persistence.clone(),
            Arc::new(SpvBroadcaster::new(spv)),
        );
        manager
            .write()
            .await
            .get_wallet_info_mut(&id)
            .unwrap()
            .tracked_asset_locks
            .insert(lock.out_point, lock.clone());
        *wallet.shielded_keys.write().await = Some(keys);
        let dir = TestDirectory(
            std::env::temp_dir().join(format!("drain_recovery_{}", rand::random::<u64>())),
        );
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("shielded.sqlite");
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        let coordinator = NetworkShieldedCoordinator::new(sdk, Network::Testnet, path, store);
        (wallet, persistence, coordinator, dir)
    }

    async fn restart_wallet(
        old: PlatformWallet,
        persistence: Arc<RecoveryPersistence>,
    ) -> PlatformWallet {
        let restored = persistence
            .load()
            .unwrap()
            .wallets
            .remove(&old.wallet_id())
            .unwrap();
        old.wallet_manager
            .write()
            .await
            .get_wallet_info_mut(&old.wallet_id())
            .unwrap()
            .tracked_asset_locks = restored
            .unused_asset_locks
            .into_values()
            .flatten()
            .collect();
        let spv = Arc::new(SpvRuntime::new(
            old.wallet_manager.clone(),
            Arc::new(PlatformEventManager::new(vec![])),
        ));
        let wallet = PlatformWallet::new(
            old.sdk.clone(),
            old.wallet_id(),
            old.wallet_manager.clone(),
            old.generation.clone(),
            Arc::new(Notify::new()),
            persistence,
            Arc::new(SpvBroadcaster::new(spv)),
        );
        *wallet.shielded_keys.write().await = old.shielded_keys.read().await.clone();
        wallet
    }

    async fn observe(
        coordinator: &NetworkShieldedCoordinator,
        wallet_id: [u8; 32],
        account: u32,
        cmx: [u8; 32],
        position: u64,
    ) {
        coordinator
            .store()
            .write()
            .await
            .save_note(
                SubwalletId::new(wallet_id, account),
                &ShieldedNote {
                    position,
                    cmx,
                    nullifier: [position as u8; 32],
                    block_height: 42,
                    is_spent: false,
                    value: 1_000,
                    note_data: vec![],
                },
            )
            .unwrap();
    }

    async fn reconcile_fully_consumed(
        wallet: &PlatformWallet,
        coordinator: &NetworkShieldedCoordinator,
        out_point: OutPoint,
    ) -> Result<bool, PlatformWalletError> {
        wallet
            .reconcile_wallet_drain_with_consumption_check(
                coordinator,
                out_point,
                |transition| async move {
                    let StateTransition::ShieldFromAssetLock(transition) = transition else {
                        panic!("expected Type18");
                    };
                    assert_eq!(transition.asset_lock_proof().out_point(), Some(out_point));
                    Ok(StateTransitionProofResult::VerifiedAssetLockConsumed(
                        StoredAssetLockInfo::FullyConsumed,
                    ))
                },
            )
            .await
    }

    async fn assert_exact_drain_history(
        wallet: &PlatformWallet,
        coordinator: &NetworkShieldedCoordinator,
        outputs: &BTreeSet<(u32, [u8; 32])>,
    ) {
        let history = coordinator
            .store()
            .read()
            .await
            .get_activity(SubwalletId::new(wallet.wallet_id(), 0), 0, 10)
            .unwrap();
        assert_eq!(history.len(), 1, "recovery upserts one canonical transfer");
        let entry = &history[0];
        let cmxs = outputs.iter().map(|(_, cmx)| *cmx).collect::<Vec<_>>();
        assert_eq!(entry.id, compute_activity_id(&cmxs));
        assert_eq!(entry.kind, ShieldedActivityKind::ShieldFromAssetLock);
        assert_eq!(entry.status, ShieldedActivityStatus::Confirmed);
        assert_eq!(entry.amount, 2_000);
        assert_eq!(
            entry.note_cmxs.iter().copied().collect::<BTreeSet<_>>(),
            cmxs.into_iter().collect()
        );
    }

    #[tokio::test]
    async fn should_release_viewing_key_lock_before_the_persistence_callback() {
        let (lock, keys, st) = evidence_fixture();
        let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
        let keys = wallet.shielded_keys.clone();
        *persistence.before_store.lock().unwrap() = Some(Box::new(move || {
            assert!(
                keys.try_write().is_ok(),
                "viewing-key writers must not wait on persistence callbacks"
            );
        }));
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .unwrap();
        assert_eq!(
            persistence.committed.lock().unwrap()[&lock.out_point]
                .shielded_funding_attempts
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn should_release_registration_after_returned_prebuild_load_failure() {
        let (mut lock, keys, _) = evidence_fixture();
        lock.operation_id = None;
        let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
        let coordinator = Arc::new(coordinator);
        let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let prover = super::super::prover::CachedOrchardProver::new();
        let operation = Arc::new(AssetLockOperationContext::new([0x78; 16]));
        for failed_load in [true, false] {
            persistence.fail_load.store(failed_load, Ordering::SeqCst);
            let result = wallet
                .shielded_fund_from_asset_lock(
                    &coordinator,
                    crate::wallet::asset_lock::AssetLockFunding::DrainWalletBalance {
                        account_index: 0,
                        minimum_lock_duffs: None,
                        operation: operation.clone(),
                    },
                    vec![],
                    &signer,
                    &prover,
                    None,
                    0,
                    None,
                    None,
                )
                .await;
            if failed_load {
                assert!(matches!(result, Err(PlatformWalletError::Persistence(_))));
                assert_eq!(operation.snapshot().state, AssetLockOperationState::Pending);
            } else {
                // Reaching ordinary recipient validation proves the abandoned
                // registration no longer blocks this same-context UUID retry.
                assert!(matches!(
                    result,
                    Err(PlatformWalletError::AddressOperation(_))
                ));
            }
        }
        assert_eq!(
            wallet
                .asset_locks
                .wallet_drain_operation_status(operation.operation_id)
                .await
                .unwrap()
                .state,
            AssetLockOperationState::NotSent
        );
        assert!(operation.snapshot().out_point.is_none());
    }

    #[tokio::test]
    async fn should_require_both_exact_outputs_and_fully_consumed_original_outpoint() {
        let (lock, keys, st) = evidence_fixture();
        let outputs = attempt_outputs(&lock, &st.serialize_to_bytes().unwrap(), &keys).unwrap();
        let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .unwrap();
        // A successful affected-state response alone cannot complete a fresh
        // submit or a recovery: its exact outputs must have been scanned too.
        assert!(
            !reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .unwrap()
        );
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(
                &coordinator,
                wallet.wallet_id(),
                *account,
                *cmx,
                index as u64,
            )
            .await;
        }
        let partial = dpp::asset_lock::reduced_asset_lock_value::AssetLockValue::new(
            1_000_000,
            vec![],
            500_000,
            vec![],
            dpp::version::PlatformVersion::latest(),
        )
        .unwrap();
        // The bundle can be replayed with another lock: sightings do not prove
        // that this original lock is fully consumed. Absence, partial use,
        // unrelated response variants and unavailable proofs stay pending.
        for result in [
            Ok(StateTransitionProofResult::VerifiedAssetLockConsumed(
                StoredAssetLockInfo::NotPresent,
            )),
            Ok(StateTransitionProofResult::VerifiedAssetLockConsumed(
                StoredAssetLockInfo::PartiallyConsumed(partial),
            )),
            Ok(StateTransitionProofResult::VerifiedShieldedNullifiers(
                vec![],
            )),
            Err(dash_sdk::Error::Generic(
                "consumption proof unavailable".into(),
            )),
        ] {
            assert!(!wallet
                .reconcile_wallet_drain_with_consumption_check(
                    &coordinator,
                    lock.out_point,
                    |transition| async move {
                        let StateTransition::ShieldFromAssetLock(transition) = transition else {
                            panic!("expected Type18")
                        };
                        assert_eq!(
                            transition.asset_lock_proof().out_point(),
                            Some(lock.out_point)
                        );
                        result
                    },
                )
                .await
                .unwrap());
            assert_ne!(
                persistence.committed.lock().unwrap()[&lock.out_point].status,
                AssetLockStatus::Consumed
            );
            assert!(
                coordinator
                    .store()
                    .read()
                    .await
                    .get_activity(SubwalletId::new(wallet.wallet_id(), 0), 0, 10)
                    .unwrap()
                    .is_empty(),
                "one-sided evidence must not create confirmed history"
            );
        }
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .unwrap()
        );
        assert_exact_drain_history(&wallet, &coordinator, &outputs).await;
    }

    #[tokio::test]
    async fn should_release_cancelled_drain_waiting_for_the_shield_guard() {
        let (mut lock, keys, _) = evidence_fixture();
        lock.operation_id = None;
        let (wallet, _, coordinator, _dir) = wallet_fixture(&lock, keys).await;
        let coordinator = Arc::new(coordinator);
        let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let prover = super::super::prover::CachedOrchardProver::new();
        let operation = Arc::new(AssetLockOperationContext::new([0x77; 16]));
        let blocker = wallet.shield_guard.lock().await;
        let mut future = Box::pin(wallet.shielded_fund_from_asset_lock(
            &coordinator,
            crate::wallet::asset_lock::AssetLockFunding::DrainWalletBalance {
                account_index: 0,
                minimum_lock_duffs: None,
                operation: operation.clone(),
            },
            vec![],
            &signer,
            &prover,
            None,
            0,
            None,
            None,
        ));
        assert!(futures::poll!(&mut future).is_pending());
        assert_eq!(
            wallet
                .asset_locks
                .wallet_drain_operation_status(operation.operation_id)
                .await
                .unwrap()
                .state,
            AssetLockOperationState::Pending
        );
        drop(future);
        drop(blocker);
        assert_eq!(operation.snapshot().state, AssetLockOperationState::Pending);
        assert_eq!(
            wallet
                .asset_locks
                .wallet_drain_operation_status(operation.operation_id)
                .await
                .unwrap()
                .state,
            AssetLockOperationState::NotSent
        );
        let _registration = wallet.register_wallet_drain_operation(operation.clone());
        assert_eq!(
            wallet
                .asset_locks
                .begin_wallet_drain(&operation)
                .await
                .unwrap(),
            None
        );
        assert!(operation.snapshot().out_point.is_none());
    }

    #[tokio::test]
    async fn should_reconcile_drain_after_dispatch_crash_using_an_earlier_exact_attempt() {
        let (lock, keys, st) = evidence_fixture();
        let outputs = attempt_outputs(&lock, &st.serialize_to_bytes().unwrap(), &keys).unwrap();
        let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys.clone()).await;
        // Dispatch is permitted only after these exact signed bytes are durable.
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .unwrap();
        assert_eq!(
            persistence.committed.lock().unwrap()[&lock.out_point].shielded_funding_attempts,
            vec![st.serialize_to_bytes().unwrap()]
        );
        // A later randomized retry must never erase the first dispatched attempt.
        let (_, _, later) = evidence_fixture();
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &later)
            .await
            .unwrap();
        let wallet = restart_wallet(wallet, persistence.clone()).await;
        assert_eq!(
            wallet.asset_locks.list_tracked_locks().await[0]
                .shielded_funding_attempts
                .len(),
            2
        );
        assert_eq!(
            wallet
                .shielded_wallet_drain_operation_status(&coordinator, [0x33; 16])
                .await
                .unwrap()
                .state,
            AssetLockOperationState::Pending
        );
        // Another wallet/account, and even a partial matching set, cannot confirm.
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(&coordinator, [0x99; 32], *account, *cmx, index as u64).await;
            observe(&coordinator, wallet.wallet_id(), 9, *cmx, index as u64).await;
        }
        assert!(!wallet
            .reconcile_wallet_drain(&coordinator, lock.out_point)
            .await
            .unwrap());
        let (account, first) = outputs.iter().next().unwrap();
        observe(&coordinator, wallet.wallet_id(), *account, *first, 0).await;
        assert!(!wallet
            .reconcile_wallet_drain(&coordinator, lock.out_point)
            .await
            .unwrap());
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(
                &coordinator,
                wallet.wallet_id(),
                *account,
                *cmx,
                index as u64,
            )
            .await;
        }
        // History and terminal tombstone commit together, before the memory
        // cache is updated and with no key/store guard held by callbacks.
        let activity_store = coordinator.store().clone();
        let key_slot = wallet.shielded_keys.clone();
        let subwallet = SubwalletId::new(wallet.wallet_id(), 0);
        *persistence.before_store.lock().unwrap() = Some(Box::new(move || {
            assert!(key_slot.try_write().is_ok());
            assert_eq!(
                activity_store
                    .try_read()
                    .unwrap()
                    .get_activity(subwallet, 0, 10)
                    .unwrap()
                    .len(),
                0
            );
        }));
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .unwrap()
        );
        assert_exact_drain_history(&wallet, &coordinator, &outputs).await;
        let status = wallet
            .shielded_wallet_drain_operation_status(&coordinator, [0x33; 16])
            .await
            .unwrap();
        assert_eq!(status.state, AssetLockOperationState::Completed);
        assert_eq!(status.out_point, Some(lock.out_point));
        assert_eq!(
            persistence.committed.lock().unwrap()[&lock.out_point].status,
            AssetLockStatus::Consumed
        );
        let wallet = restart_wallet(wallet, persistence).await;
        assert_eq!(
            wallet
                .shielded_wallet_drain_operation_status(&coordinator, [0x33; 16])
                .await
                .unwrap()
                .state,
            AssetLockOperationState::Completed
        );
        let next = Arc::new(AssetLockOperationContext::new([0x44; 16]));
        let _registration = wallet.register_wallet_drain_operation(next.clone());
        assert!(
            wallet
                .asset_locks
                .begin_wallet_drain(&next)
                .await
                .unwrap()
                .is_none(),
            "a new explicitly requested drain is no longer blocked"
        );
    }

    #[tokio::test]
    async fn should_fail_closed_on_attempt_or_completion_persistence_failure() {
        let (lock, keys, st) = evidence_fixture();
        let outputs = attempt_outputs(&lock, &st.serialize_to_bytes().unwrap(), &keys).unwrap();
        let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
        persistence.fail_store.store(true, Ordering::SeqCst);
        assert!(wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .is_err());
        assert!(persistence.committed.lock().unwrap()[&lock.out_point]
            .shielded_funding_attempts
            .is_empty());
        persistence.fail_store.store(false, Ordering::SeqCst);
        persistence.fail_flush.store(true, Ordering::SeqCst);
        assert!(wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .is_err());
        assert!(persistence.committed.lock().unwrap()[&lock.out_point]
            .shielded_funding_attempts
            .is_empty());
        persistence.fail_flush.store(false, Ordering::SeqCst);
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .unwrap();
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(
                &coordinator,
                wallet.wallet_id(),
                *account,
                *cmx,
                index as u64,
            )
            .await;
        }
        persistence.fail_flush.store(true, Ordering::SeqCst);
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .is_err()
        );
        assert_ne!(
            wallet.asset_locks.list_tracked_locks().await[0].status,
            AssetLockStatus::Consumed
        );
        assert_ne!(
            persistence.committed.lock().unwrap()[&lock.out_point].status,
            AssetLockStatus::Consumed
        );
        assert!(coordinator
            .store()
            .read()
            .await
            .get_activity(SubwalletId::new(wallet.wallet_id(), 0), 0, 10)
            .unwrap()
            .is_empty());
        persistence.fail_flush.store(false, Ordering::SeqCst);
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .unwrap()
        );
        assert_exact_drain_history(&wallet, &coordinator, &outputs).await;
    }

    struct FixedDrainSigner;
    #[async_trait::async_trait]
    impl key_wallet::signer::Signer for FixedDrainSigner {
        type Error = String;
        fn supported_methods(&self) -> &[key_wallet::signer::SignerMethod] {
            &[key_wallet::signer::SignerMethod::Digest]
        }
        async fn public_key(
            &self,
            _: &key_wallet::bip32::DerivationPath,
        ) -> Result<dashcore::secp256k1::PublicKey, String> {
            Ok(dashcore::secp256k1::PublicKey::from_secret_key(
                &dashcore::secp256k1::Secp256k1::new(),
                &dashcore::secp256k1::SecretKey::from_slice(&[0x11; 32]).unwrap(),
            ))
        }
        async fn sign_ecdsa(
            &self,
            path: &key_wallet::bip32::DerivationPath,
            digest: [u8; 32],
        ) -> Result<
            (
                dashcore::secp256k1::ecdsa::Signature,
                dashcore::secp256k1::PublicKey,
            ),
            String,
        > {
            let signature = dashcore::secp256k1::Secp256k1::new().sign_ecdsa(
                &dashcore::secp256k1::Message::from_digest(digest),
                &dashcore::secp256k1::SecretKey::from_slice(&[0x11; 32]).unwrap(),
            );
            Ok((signature, self.public_key(path).await?))
        }
    }
    #[async_trait::async_trait]
    impl key_wallet::signer::ExtendedPubKeySigner for FixedDrainSigner {
        async fn extended_public_key(
            &self,
            _: &key_wallet::bip32::DerivationPath,
        ) -> Result<key_wallet::bip32::ExtendedPubKey, String> {
            Err("not used by asset-lock signing".into())
        }
    }

    #[tokio::test]
    async fn should_validate_actual_outer_external_signer_for_each_protocol_generation() {
        for version in [PlatformVersion::get(13).unwrap(), PlatformVersion::latest()] {
            let (lock, keys, template) = evidence_fixture_at(version);
            let StateTransition::ShieldFromAssetLock(attempt) = template else {
                panic!("Type18 expected");
            };
            let signed = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
                attempt.asset_lock_proof().clone(),
                &key_wallet::bip32::DerivationPath::default(),
                &FixedDrainSigner,
                attempt.actions().to_vec(),
                attempt.value_balance(),
                [0; 32],
                vec![],
                [0; 64],
                None,
                version,
            )
            .await
            .unwrap();
            assert_eq!(
                attempt_outputs(&lock, &signed.serialize_to_bytes().unwrap(), &keys)
                    .unwrap()
                    .len(),
                2
            );
            assert!(matches!(
                (&signed, version.protocol_version),
                (
                    StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(_)),
                    13
                ) | (
                    StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V1(_)),
                    14
                )
            ));
            let mut altered = signed;
            if let StateTransition::ShieldFromAssetLock(attempt) = &mut altered {
                attempt.set_value_balance(attempt.value_balance() + 1);
            }
            assert!(attempt_outputs(&lock, &altered.serialize_to_bytes().unwrap(), &keys).is_err());
        }
    }

    #[tokio::test]
    async fn should_recover_outer_v0_and_v1_and_retire_only_replay_format() {
        let v13 = PlatformVersion::get(13).unwrap();
        let (mut lock, keys, old) = evidence_fixture_at(v13);
        assert!(matches!(
            &old,
            StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(_))
        ));
        let old_bytes = old.serialize_to_bytes().unwrap();
        let outputs = attempt_outputs(&lock, &old_bytes, &keys).unwrap();
        lock.shielded_funding_attempts.push(old_bytes.clone());
        let (wallet, persistence, coordinator, _dir) = wallet_fixture(&lock, keys).await;
        let wallet = restart_wallet(wallet, persistence.clone()).await;
        // No SDK network expectation: retired V0 is not dispatched to PV14.
        assert!(!wallet
            .replay_wallet_drain_attempt(lock.out_point, None)
            .await
            .unwrap());
        let (new_lock, new_keys, current) = evidence_fixture();
        assert_eq!(new_lock.out_point, lock.out_point);
        assert!(matches!(
            &current,
            StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V1(_))
        ));
        assert!(attempt_outputs(&lock, &current.serialize_to_bytes().unwrap(), &new_keys).is_ok());
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &current)
            .await
            .unwrap();
        let operation = Arc::new(AssetLockOperationContext::new(lock.operation_id.unwrap()));
        let _registration = wallet.register_wallet_drain_operation(operation.clone());
        assert_eq!(
            wallet
                .asset_locks
                .begin_wallet_drain(&operation)
                .await
                .unwrap(),
            Some(lock.out_point)
        );
        let committed = persistence.committed.lock().unwrap().clone();
        assert_eq!(
            committed.len(),
            1,
            "protocol upgrade must never create a second Core lock"
        );
        assert_eq!(
            committed[&lock.out_point].shielded_funding_attempts,
            vec![old_bytes, current.serialize_to_bytes().unwrap()]
        );
        // Historical V0 is still usable completion evidence after V1 is appended.
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(
                &coordinator,
                wallet.wallet_id(),
                *account,
                *cmx,
                index as u64,
            )
            .await;
        }
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .unwrap()
        );
        assert_exact_drain_history(&wallet, &coordinator, &outputs).await;
    }

    #[tokio::test]
    async fn should_refuse_drain_without_viewing_keys_before_core_funding() {
        for empty_binding in [false, true] {
            let (mut lock, keys, _) = evidence_fixture();
            lock.operation_id = None;
            let recipient = dpp::address_funds::OrchardAddress::from(keys[&0].default_address);
            let (wallet, persistence, coordinator, _dir) =
                wallet_fixture(&lock, keys.clone()).await;
            *wallet.shielded_keys.write().await = empty_binding.then(BTreeMap::new);
            let coordinator = Arc::new(coordinator);
            let (_, _, _, signer) = funded_wallet_manager(StandardAccountType::BIP44Account).await;
            let prover = super::super::prover::CachedOrchardProver::new();
            let operation = Arc::new(AssetLockOperationContext::new([0x97; 16]));
            let error = wallet
                .shielded_fund_from_asset_lock(
                    &coordinator,
                    crate::wallet::asset_lock::AssetLockFunding::DrainWalletBalance {
                        account_index: 0,
                        minimum_lock_duffs: None,
                        operation: operation.clone(),
                    },
                    vec![(recipient, None)],
                    &signer,
                    &prover,
                    None,
                    0,
                    None,
                    None,
                )
                .await
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("requires bound shielded viewing keys"),
                "{error}"
            );
            assert_eq!(operation.snapshot().state, AssetLockOperationState::NotSent);
            assert!(operation.snapshot().out_point.is_none());
            assert_eq!(persistence.committed.lock().unwrap().len(), 1);
            assert!(persistence.staged.lock().unwrap().is_empty());
            assert_eq!(wallet.asset_locks.list_tracked_locks().await.len(), 1);
            // Binding keys leaves the same UUID available for a fresh attempt.
            *wallet.shielded_keys.write().await = Some(keys);
            let _registration = wallet.register_wallet_drain_operation(operation.clone());
            assert_eq!(
                wallet
                    .asset_locks
                    .begin_wallet_drain(&operation)
                    .await
                    .unwrap(),
                None
            );
        }
    }

    #[tokio::test]
    async fn should_refuse_drain_without_atomic_activity_capability_before_build() {
        let (lock, keys, _) = evidence_fixture();
        let (wallet, persistence, _, _dir) = wallet_fixture(&lock, keys).await;
        persistence
            .omit_activity_capability
            .store(true, Ordering::SeqCst);
        let operation = Arc::new(AssetLockOperationContext::new([0x98; 16]));
        let _registration = wallet.register_wallet_drain_operation(operation.clone());
        let error = wallet
            .asset_locks
            .begin_wallet_drain(&operation)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("shielded_activity_reconciliation"));
        assert!(operation.snapshot().out_point.is_none());
        assert_eq!(persistence.committed.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn should_atomically_replace_scan_aggregate_preserving_other_receipt_after_restart() {
        let (lock, keys, st) = evidence_fixture();
        let outputs = attempt_outputs(&lock, &st.serialize_to_bytes().unwrap(), &keys).unwrap();
        let views = keys[&0].clone();
        let (wallet, persistence, coordinator, dir) = wallet_fixture(&lock, keys.clone()).await;
        wallet
            .persist_drain_shielded_attempt(lock.out_point, &st)
            .await
            .unwrap();
        let id = SubwalletId::new(wallet.wallet_id(), 0);
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(
                &coordinator,
                wallet.wallet_id(),
                *account,
                *cmx,
                index as u64,
            )
            .await;
        }
        let unrelated_cmx = [0x91; 32];
        observe(&coordinator, wallet.wallet_id(), 0, unrelated_cmx, 2).await;
        let mut scan = crate::changeset::ShieldedChangeSet::default();
        coordinator
            .derive_activity_into_changeset(&[(id, views)], &mut scan)
            .await
            .unwrap();
        let aggregate = coordinator
            .store()
            .read()
            .await
            .get_activity(id, 0, 10)
            .unwrap();
        assert_eq!(aggregate.len(), 1);
        assert_eq!(aggregate[0].amount, 3_000);
        assert_eq!(aggregate[0].note_cmxs.len(), 3);
        // A distinct authored C discovered at the same anchor must survive,
        // and must not cause residual B to be suppressed by overlap deduping.
        let authored_cmx = [0x92; 32];
        observe(&coordinator, wallet.wallet_id(), 0, authored_cmx, 3).await;
        let mut authored = aggregate[0].clone();
        authored.id = compute_activity_id(&[authored_cmx]);
        authored.note_cmxs = vec![authored_cmx];
        authored.amount = 1_000;
        authored.kind = ShieldedActivityKind::Shield;
        authored.created_at_ms = 123;
        coordinator
            .store()
            .write()
            .await
            .save_activity(id, &authored)
            .unwrap();
        let before = coordinator
            .store()
            .read()
            .await
            .get_activity(id, 0, 10)
            .unwrap();
        let snapshot = crate::changeset::ShieldedSubwalletStartState {
            activity: before.clone(),
            notes: coordinator.store().read().await.get_all_notes(id).unwrap(),
            ..Default::default()
        };
        persistence.activity.lock().unwrap().insert(id, snapshot);
        // Only the activity callback fails; the lock write itself would succeed.
        persistence.fail_activity.store(true, Ordering::SeqCst);
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .is_err()
        );
        let restored = persistence.load().unwrap();
        assert_ne!(
            persistence.committed.lock().unwrap()[&lock.out_point].status,
            AssetLockStatus::Consumed
        );
        assert_eq!(restored.shielded.per_subwallet[&id].activity, before);
        // Reopen a fresh coordinator cache entirely from the host snapshot.
        drop(coordinator);
        let wallet = restart_wallet(wallet, persistence.clone()).await;
        let path = dir.0.join("reopened.sqlite");
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        let coordinator =
            NetworkShieldedCoordinator::new(wallet.sdk.clone(), Network::Testnet, path, store);
        coordinator
            .register_wallet(wallet.wallet_id(), keys, wallet.persister().clone())
            .await
            .unwrap();
        coordinator
            .restore_for_wallet(wallet.wallet_id(), &restored.shielded)
            .await
            .unwrap();
        persistence.fail_activity.store(false, Ordering::SeqCst);
        assert!(
            reconcile_fully_consumed(&wallet, &coordinator, lock.out_point)
                .await
                .unwrap()
        );
        let durable = persistence.load().unwrap();
        let rows = &durable.shielded.per_subwallet[&id].activity;
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|row| row.id != aggregate[0].id));
        assert_eq!(rows.iter().map(|row| row.amount).sum::<u64>(), 4_000);
        assert!(rows.iter().any(
            |row| row.kind == ShieldedActivityKind::ShieldFromAssetLock && row.amount == 2_000
        ));
        assert!(rows
            .iter()
            .any(|row| row.kind == ShieldedActivityKind::Received
                && row.note_cmxs == vec![unrelated_cmx]
                && row.amount == 1_000));
        assert!(rows.contains(&authored));
        assert_eq!(
            persistence.committed.lock().unwrap()[&lock.out_point].status,
            AssetLockStatus::Consumed
        );
        // A new scan cannot recreate the removed aggregate or double count A.
        let mut scan = crate::changeset::ShieldedChangeSet::default();
        coordinator
            .derive_activity_into_changeset(
                &[(
                    id,
                    wallet.shielded_keys.read().await.as_ref().unwrap()[&0].clone(),
                )],
                &mut scan,
            )
            .await
            .unwrap();
        // The existing live C may gain a confirmation height; no aggregate
        // or duplicate of A/B may be emitted.
        assert!(scan
            .activity_entries
            .values()
            .flatten()
            .all(|entry| entry.id == authored.id));
        assert_eq!(
            coordinator
                .store()
                .read()
                .await
                .get_activity(id, 0, 10)
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn should_reject_foreign_corrupted_unsigned_or_incomplete_drain_evidence() {
        let (lock, keys, st) = evidence_fixture();
        let bytes = st.serialize_to_bytes().unwrap();
        assert!(attempt_outputs(&lock, &bytes, &keys).is_ok());
        assert!(attempt_outputs(&lock, b"not a transition", &keys).is_err());
        let mut foreign = lock.clone();
        foreign.out_point.vout += 1;
        assert!(attempt_outputs(&foreign, &bytes, &keys).is_err());
        let mut unrelated = lock.clone();
        unrelated.operation_id = None;
        assert!(attempt_outputs(&unrelated, &bytes, &keys).is_err());
        let mut wrong_role = lock.clone();
        wrong_role.funding_type = AssetLockFundingType::IdentityRegistration;
        assert!(attempt_outputs(&wrong_role, &bytes, &keys).is_err());
        let other_keys = BTreeMap::from([(
            0,
            OrchardKeySet::from_seed(&[0x88; 32], Network::Testnet, 0)
                .unwrap()
                .viewing_keys(),
        )]);
        assert!(attempt_outputs(&lock, &bytes, &other_keys).is_err());
        let mut altered = st.clone();
        if let StateTransition::ShieldFromAssetLock(transition) = &mut altered {
            transition.set_value_balance(transition.value_balance() + 1);
        }
        assert!(attempt_outputs(&lock, &altered.serialize_to_bytes().unwrap(), &keys).is_err());
        let mut unsigned = st;
        if let StateTransition::ShieldFromAssetLock(transition) = &mut unsigned {
            transition.set_signature(Default::default());
        }
        assert!(attempt_outputs(&lock, &unsigned.serialize_to_bytes().unwrap(), &keys).is_err());
    }

    #[tokio::test]
    async fn should_leave_legacy_drain_without_evidence_pending_even_with_matching_balance() {
        let (lock, keys, st) = evidence_fixture();
        let outputs = attempt_outputs(&lock, &st.serialize_to_bytes().unwrap(), &keys).unwrap();
        let (wallet, _, coordinator, _dir) = wallet_fixture(&lock, keys).await;
        for (index, (account, cmx)) in outputs.iter().enumerate() {
            observe(
                &coordinator,
                wallet.wallet_id(),
                *account,
                *cmx,
                index as u64,
            )
            .await;
        }
        assert!(!wallet
            .reconcile_wallet_drain(&coordinator, lock.out_point)
            .await
            .unwrap());
        assert_eq!(
            wallet
                .shielded_wallet_drain_operation_status(&coordinator, [0x33; 16])
                .await
                .unwrap()
                .state,
            AssetLockOperationState::Pending
        );
    }
}
