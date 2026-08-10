//! Shielded transaction operations (6 transition types), multi-account.
//!
//! Each operation is a free function taking the
//! (sdk, store, persister, wallet_id, keys, account, …) tuple
//! it needs explicitly. Phase 4d.3 lifted these out of
//! `impl<S> ShieldedWallet<S>` so per-wallet shielded state on
//! `PlatformWallet` can be just the keys map without a wrapper
//! struct.
//!
//! Spends never cross account boundaries — note selection reads
//! only the given account's unspent notes.
//!
//! The six transition types are:
//! - **Shield** (Type 15): transparent platform addresses → shielded pool
//! - **ShieldFromAssetLock** (Type 18): Core L1 asset lock → shielded pool
//! - **Unshield** (Type 17): shielded pool → transparent platform address
//! - **Transfer** (Type 16): shielded pool → shielded pool (private)
//! - **Withdraw** (Type 19): shielded pool → Core L1 address
//! - **IdentityCreateFromShieldedPool** (Type 20): shielded pool → a brand-new Platform identity
//!   funded by a fixed denomination leaving the pool (any excess re-enters as a change note)

use super::activity::{ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection};
use super::activity_recorder::{
    build_pending_entry, changeset_for_entry, non_zero_memo, with_status, LiveEntryParams,
};
use super::keys::{AccountViewingKeys, OrchardKeySet};
use super::note_selection::{
    select_notes_for_denomination, select_notes_with_fee, ShieldedFeeKind,
};
use super::store::{PendingRedrive, ShieldedNote, ShieldedStore, SubwalletId};
use crate::changeset::{PlatformWalletChangeSet, ShieldedChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::identity_create_from_shielded_pool::IdentityCreateFromShieldedPool;
use dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, OrchardAddress, PlatformAddress,
};
use dpp::fee::Credits;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::prelude::Identifier;
use dpp::shielded::builder::{
    build_identity_create_from_shielded_pool_transition, build_shield_transition,
    build_shielded_transfer_transition, build_shielded_withdrawal_transition,
    build_unshield_transition, OrchardProver, SpendableNote,
};
use dpp::shielded::compute_minimum_shielded_fee;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::StateTransition;
use dpp::withdrawal::Pooling;
use grovedb_commitment_tree::{Anchor, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

/// Number of Orchard actions in a `Shield` (Type 15) bundle.
///
/// `build_shield_transition` builds an *output-only* bundle with a single
/// output (`build_output_only_bundle`), configured as Orchard's
/// `BundleType::Transactional { flags: SPENDS_DISABLED, bundle_required: false }`.
/// For one output and zero spends, Orchard's `num_actions` is
/// `max(max(0, 1), MIN_ACTIONS) == max(1, 2) == 2`, so the serialized bundle
/// always carries exactly two actions. Consensus prices the flat shielded fee
/// `F = compute_minimum_shielded_fee(actions.len())` from the on-wire action
/// count, so the wallet's fee reservation must use the same count.
const SHIELD_NUM_ACTIONS: usize = 2;

/// Try to extract a structured `AddressesNotEnoughFundsError` from
/// a broadcast error so the shield path can format a diagnostic
/// that includes Platform's actual per-input view (nonce + balance)
/// rather than just the stringified message.
fn addresses_not_enough_funds(
    e: &dash_sdk::Error,
) -> Option<&dpp::consensus::state::address_funds::AddressesNotEnoughFundsError> {
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::ProtocolError;

    let consensus: &ConsensusError = match e {
        dash_sdk::Error::Protocol(ProtocolError::ConsensusError(boxed)) => boxed.as_ref(),
        dash_sdk::Error::StateTransitionBroadcastError(s) => s.cause.as_ref()?,
        _ => return None,
    };
    match consensus {
        ConsensusError::StateError(StateError::AddressesNotEnoughFundsError(err)) => Some(err),
        _ => None,
    }
}

/// Try to extract the per-address `AddressNotEnoughFundsError` that the
/// pre-flight `fetch_inputs_with_nonce` hard balance check raises when a
/// single input is short. Mirrors [`addresses_not_enough_funds`] (the
/// plural broadcast-side variant) so the pre-broadcast failure can carry
/// the same structured `(address, balance, required)` info across the
/// FFI instead of collapsing to an opaque stringified error.
fn address_not_enough_funds(
    e: &dash_sdk::Error,
) -> Option<&dpp::consensus::state::address_funds::AddressNotEnoughFundsError> {
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::ProtocolError;

    let consensus: &ConsensusError = match e {
        dash_sdk::Error::Protocol(ProtocolError::ConsensusError(boxed)) => boxed.as_ref(),
        dash_sdk::Error::StateTransitionBroadcastError(s) => s.cause.as_ref()?,
        _ => return None,
    };
    match consensus {
        ConsensusError::StateError(StateError::AddressNotEnoughFundsError(err)) => Some(err),
        _ => None,
    }
}

/// Promote the shield pre-broadcast hard balance check into the typed capacity
/// error the FFI and Swift layers recognize.
///
/// The values are Platform's live per-input view, not the cached planner
/// snapshot. Their `Display` rendering therefore preserves the actionable
/// available/required diagnostic while the typed variant lets the host refresh
/// preflight instead of retrying the stale amount unchanged.
fn map_shield_input_fetch_error(e: &dash_sdk::Error) -> PlatformWalletError {
    match address_not_enough_funds(e) {
        Some(short) => PlatformWalletError::ShieldedInsufficientBalance {
            available: short.balance(),
            required: short.required_balance(),
        },
        None => PlatformWalletError::ShieldedBuildError(format!("fetch input nonces: {e}")),
    }
}

/// Format a one-line `addresses_with_info` summary for diagnostics —
/// each entry rendered as `<bech32m_addr>=(nonce <n>, <c> credits)`,
/// matching what the wallet UI shows.
fn format_addresses_with_info(
    map: &std::collections::BTreeMap<
        dpp::address_funds::PlatformAddress,
        (dpp::prelude::AddressNonce, dpp::fee::Credits),
    >,
    network: key_wallet::Network,
) -> String {
    map.iter()
        .map(|(addr, (nonce, credits))| {
            format!(
                "{}=(nonce {nonce}, {credits} credits)",
                addr.to_bech32m_string(network)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Queue a shielded changeset on the persister if one is
/// attached. No-op if the changeset is empty or no persister
/// was supplied.
fn queue_shielded_changeset(
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    cs: ShieldedChangeSet,
) {
    if cs.is_empty() {
        return;
    }
    let Some(persister) = persister else {
        return;
    };
    let full = PlatformWalletChangeSet {
        shielded: Some(cs),
        ..Default::default()
    };
    if let Err(e) = persister.store(full) {
        tracing::warn!(
            wallet_id = %hex::encode(wallet_id),
            error = %e,
            "Failed to queue shielded changeset"
        );
    }
}

/// Write an activity entry to the in-memory store, then queue it to the
/// host persister. For callers outside this module (the Type 18
/// orchestrator in `fund_from_asset_lock.rs`). Upserts by `entry.id`.
///
/// The in-memory `save_activity` MUST happen before the persister queue so
/// the same-session scan deriver's dedupe (which reads the in-memory store)
/// sees this live row and never re-derives a coarser entry over it. A store
/// write failure is logged but does not fail the op — the persister still
/// gets the entry, and the next sync reconciles the in-memory side.
pub(super) async fn queue_shielded_activity<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    entry: super::activity::ShieldedActivityEntry,
) {
    if let Err(e) = store.write().await.save_activity(id, &entry) {
        warn!(
            entry_id = %hex::encode(entry.id),
            error = %e,
            "live activity entry: in-memory save_activity failed; persister still queued"
        );
    }
    queue_shielded_changeset(persister, wallet_id, changeset_for_entry(id, entry));
}

/// Extract the serialized Orchard actions from a built shielded
/// `StateTransition`, for live activity-entry recording.
///
/// Returns `&[]` for any non-shielded variant (none reach the recorder).
/// Each shielded variant's `actions()` comes from its own accessor /
/// methods trait, so the relevant traits are imported locally.
fn shielded_actions(st: &StateTransition) -> &[dpp::shielded::SerializedAction] {
    // `actions()` is on the per-type accessor trait for the spend-based
    // transitions; Shield / ShieldFromAssetLock expose `actions` as a
    // public field on their V0 struct (no accessor trait), so match down
    // to the V0 variant for those two.
    use dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shielded_transfer_transition::accessors::ShieldedTransferTransitionAccessorsV0;
    use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;
    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
    use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;

    match st {
        StateTransition::Shield(ShieldTransition::V0(v0)) => &v0.actions,
        StateTransition::ShieldedTransfer(t) => t.actions(),
        StateTransition::Unshield(t) => t.actions(),
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(v0)) => &v0.actions,
        StateTransition::ShieldedWithdrawal(t) => t.actions(),
        StateTransition::IdentityCreateFromShieldedPool(t) => t.actions(),
        _ => &[],
    }
}

/// Record a live activity entry built from `params` for `id`, writing it
/// to the in-memory `store` and then shipping it to the host persister.
/// Returns the recorded entry so the caller can flip its status later (the
/// status flip re-records by the same `entry.id`, an upsert).
///
/// The in-memory `save_activity` MUST land before the persister queue so
/// the same-session scan deriver's dedupe (which reads the IN-MEMORY store
/// — see `coordinator::derive_activity_into_changeset`) sees this live row
/// and never re-derives a coarser entry that `save_activity` would upsert
/// over it. See [`queue_shielded_activity`].
///
/// Returns `None` (and queues nothing) when the bundle exposes no
/// wallet-visible output cmx — see [`build_pending_entry`].
async fn record_pending_activity<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    keys: &AccountViewingKeys,
    params: LiveEntryParams<'_>,
) -> Option<super::activity::ShieldedActivityEntry> {
    let kind = params.kind.clone();
    let Some(entry) = build_pending_entry(keys, params) else {
        // Should be unreachable for bundles our own builders produced
        // (they always carry at least one wallet-visible output) — but a
        // silent skip here means the operation leaves no history, so
        // make it loud.
        warn!(
            ?kind,
            "live activity entry skipped: no wallet-visible output cmx recovered from the bundle"
        );
        return None;
    };
    info!(
        ?kind,
        entry_id = %hex::encode(entry.id),
        cmxs = entry.note_cmxs.len(),
        "live activity entry recorded (pending)"
    );
    queue_shielded_activity(store, persister, wallet_id, id, entry.clone()).await;
    Some(entry)
}

/// Re-record `entry` with a flipped status (and optional confirmed
/// height) for `id`. Upserts by `entry.id` in the in-memory store and at
/// the persister, so a `Pending` row becomes `Confirmed` / `Failed` in
/// place. No-op when `entry` is `None` (nothing was recorded for this
/// operation).
///
/// The flip is applied to the CURRENT stored row (read under the same
/// write lock as the save), not the captured pre-broadcast entry: a
/// concurrent scan pass can upgrade the row to `Confirmed` at a real
/// height between broadcast and the result-wait, and rewriting the stale
/// capture would erase that scan-learned height. A row the scan already
/// confirmed WITH a height is chain truth and is left untouched
/// entirely — nothing the result-wait knows can improve on it.
async fn record_activity_status<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    entry: &Option<super::activity::ShieldedActivityEntry>,
    status: ShieldedActivityStatus,
    block_height: Option<u64>,
) {
    let Some(entry) = entry else {
        return;
    };
    let next = {
        let mut store = store.write().await;
        // A read FAILURE is not "no row": falling back to the captured
        // entry on Err would queue exactly the stale overwrite this
        // function exists to prevent, precisely when the invariant
        // couldn't be checked. Skip the flip instead — the scan
        // confirmation path reconciles the row's status later.
        let stored = match store.get_activity_by_entry_id(id, &entry.id) {
            Ok(stored) => stored,
            Err(e) => {
                warn!(
                    entry_id = %hex::encode(entry.id),
                    error = %e,
                    "live activity status flip: get_activity_by_entry_id failed; \
                     skipping flip to avoid clobbering a richer row"
                );
                return;
            }
        };
        let base = stored.as_ref().unwrap_or(entry);
        if base.status == ShieldedActivityStatus::Confirmed && base.block_height.is_some() {
            return;
        }
        // `with_status` keeps `base.block_height` when `block_height` is
        // `None`, so a height the scan populated survives the flip.
        let next = with_status(base, status, block_height);
        if let Err(e) = store.save_activity(id, &next) {
            warn!(
                entry_id = %hex::encode(next.id),
                error = %e,
                "live activity status flip: in-memory save_activity failed; persister still queued"
            );
        }
        next
    };
    queue_shielded_changeset(persister, wallet_id, changeset_for_entry(id, next));
}

/// Flip the activity row identified by `entry_id` (if present) to
/// `status`, reusing [`record_activity_status`]'s semantics — it re-reads
/// the CURRENT stored row and leaves a scan-`Confirmed`-with-height row
/// untouched. Used by the sync reconcile, which knows only the reservation's
/// stored `activity_id`, not the whole entry. No-op if no such row exists.
pub(super) async fn record_activity_status_by_id<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    entry_id: &[u8; 32],
    status: ShieldedActivityStatus,
) {
    let entry = match store.read().await.get_activity_by_entry_id(id, entry_id) {
        Ok(entry) => entry,
        Err(e) => {
            warn!(
                entry_id = %hex::encode(entry_id),
                error = %e,
                "activity status flip by id: lookup failed; skipping"
            );
            return;
        }
    };
    record_activity_status(store, persister, wallet_id, id, &entry, status, None).await;
}

// -------------------------------------------------------------------------
// Shield: platform addresses -> shielded pool (Type 15)
// -------------------------------------------------------------------------

/// Add the flat shielded fee `fee` to the smallest-key input's claim.
///
/// The Shield fee strategy is `DeductFromInput(0)`, where "input 0" is the
/// BTreeMap-smallest address. Consensus requires the input claims to sum to at
/// least `amount + fee`; the caller's selection claims exactly `amount` and
/// reserves the fee headroom as *unclaimed* balance on input 0, so loading the
/// fee onto that same input both satisfies the `Σ inputs >= amount + fee`
/// structure check and keeps the fee-payer aligned with the fee strategy.
///
/// Errors if `inputs` is empty (no input to carry the fee) or the addition
/// overflows.
fn reserve_shield_fee_on_input_0(
    mut inputs: BTreeMap<PlatformAddress, Credits>,
    fee: Credits,
) -> Result<BTreeMap<PlatformAddress, Credits>, PlatformWalletError> {
    let Some((&input_0_addr, _)) = inputs.iter().next() else {
        return Err(PlatformWalletError::ShieldedBuildError(
            "shield has no inputs to carry the shielded fee".to_string(),
        ));
    };
    let claim = inputs
        .get_mut(&input_0_addr)
        .expect("input_0_addr was just read from the map");
    *claim = claim.checked_add(fee).ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError(
            "input 0 claim + shielded fee overflows u64".to_string(),
        )
    })?;
    Ok(inputs)
}

/// Shield credits from transparent platform addresses into the
/// shielded pool, with the resulting note assigned to `account`'s
/// default Orchard payment address derived from `keys`.
#[allow(clippy::too_many_arguments)]
pub async fn shield<S: ShieldedStore, Sig: Signer<PlatformAddress>, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &AccountViewingKeys,
    account: u32,
    inputs: BTreeMap<PlatformAddress, Credits>,
    amount: u64,
    signer: &Sig,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let recipient_addr = default_orchard_address(keys)?;
    let id = SubwalletId::new(wallet_id, account);

    // Reserve the flat shielded fee `F` on top of `amount` in the input
    // claims. Consensus `validate_structure` (rs-dpp) now rejects a Shield
    // unless `Σ inputs >= amount + F`, where
    // `F = compute_minimum_shielded_fee(num_actions)` and `num_actions` is
    // the serialized output-only bundle's action count. That bundle has a
    // single output and spends disabled, so Orchard pads it to exactly
    // SHIELD_NUM_ACTIONS == 2 actions (see the constant doc below). We
    // mirror `note_selection.rs`'s spend-side fee math.
    //
    // The fee is loaded onto the smallest-key input — the `DeductFromInput(0)`
    // fee-strategy payer (input 0 == BTreeMap-smallest address). The caller
    // (`shielded_shield_from_account`) reserves ~1e9 credits of unclaimed
    // headroom on input 0 specifically for this, and `F` (~1.2e8 credits)
    // fits well within it. Inflating the claim BEFORE the fetch lets the
    // single hard balance check below validate the fee-inclusive claim
    // against the on-chain balance in one shot — no second round-trip and
    // no claim that outruns its balance check.
    let fee = compute_minimum_shielded_fee(SHIELD_NUM_ACTIONS, sdk.version())
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
    let inputs = reserve_shield_fee_on_input_0(inputs, fee)?;

    // Reuse rs-sdk's canonical fetch + hard balance check rather than
    // re-implementing the fetch-and-validate dance. Unlike the old
    // warn-and-proceed path, `fetch_inputs_with_nonce` errors with
    // `AddressNotEnoughFundsError` when any input is short, so we fail
    // before paying the ~30 s Halo 2 proof for a transition drive-abci
    // would reject. It returns the *current* on-chain nonces; we apply
    // a checked increment (the canonical `nonce_inc` uses an unchecked
    // `+ 1`, which would wrap an address at u32::MAX into a replay
    // nonce — bail loudly here instead).
    use dash_sdk::platform::transition::fetch_inputs_with_nonce;

    let fetched = fetch_inputs_with_nonce(sdk, &inputs)
        .await
        .map_err(|error| map_shield_input_fetch_error(&error))?;

    let mut inputs_with_nonce: BTreeMap<PlatformAddress, (u32, Credits)> = BTreeMap::new();
    for (addr, (nonce, credits)) in fetched {
        let next_nonce = nonce.checked_add(1).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(format!(
                "input address nonce exhausted on platform: {addr:?}"
            ))
        })?;
        inputs_with_nonce.insert(addr, (next_nonce, credits));
    }

    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

    info!(account, credits = amount, "Shield: building proof");

    let claimed_inputs = inputs_with_nonce.clone();

    let state_transition = build_shield_transition(
        &recipient_addr,
        amount,
        inputs_with_nonce,
        fee_strategy,
        signer,
        0, // user_fee_increase
        prover,
        [0u8; 36], // empty memo
        // Encrypt the output under the account's own OVK so the wallet's
        // shielded sync can recover this send (recipient, value, memo)
        // from chain data alone.
        Some(keys.outgoing_viewing_key.clone()),
        sdk.version(),
    )
    .await
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

    trace!("Shield credits: state transition built, broadcasting...");
    let network = sdk.network;

    // Live activity: Shield is `direction in`, amount = the note value
    // entering the pool, fee = the flat shielded fee reserved above. The
    // visible output cmx is the recipient note (own address, OVK-keyed),
    // which the scan later sees as an outgoing note recovered to self —
    // the ids line up.
    let pending_entry = record_pending_activity(
        store,
        persister,
        wallet_id,
        id,
        keys,
        LiveEntryParams {
            kind: ShieldedActivityKind::Shield,
            direction: ShieldedDirection::In,
            amount,
            fee: Some(fee),
            counterparty: None,
            memo: None,
            actions: shielded_actions(&state_transition),
            spent_notes: &[],
        },
    )
    .await;

    // Wait for proven execution (not just relay-ACK) so the host only
    // sees success once Platform has actually included the transition. A
    // DAPI-level ACK alone could otherwise mask a later Platform
    // rejection. The proven result is discarded; we only need the
    // confirmation. Staged like `broadcast_shielded_spend`: only a
    // DEFINITIVE verdict (CheckTx rejection / refused admission) marks
    // the activity row Failed — an ambiguous wait failure (timeout,
    // result-proof fetch error) leaves it Pending, because the shield
    // may still land and the scan's cmx-overlap confirmation will flip
    // the row when its note appears on-chain. Shield takes no note
    // reservation, so the ambiguous arm strands no local spend state;
    // its inputs are transparent address claims guarded by on-chain
    // nonces, and a host-level retry re-fetches those nonces.
    let enrich = |e: &dash_sdk::Error| -> PlatformWalletError {
        if let Some(rich) = addresses_not_enough_funds(e) {
            let claimed = claimed_inputs
                .iter()
                .map(|(addr, (nonce, credits))| {
                    format!(
                        "{}=(nonce {nonce}, {credits} credits)",
                        addr.to_bech32m_string(network)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            PlatformWalletError::ShieldedBroadcastFailed(format!(
                "addresses not enough funds: required {} credits; \
                 claimed inputs [{}]; platform sees [{}]",
                rich.required_balance(),
                claimed,
                format_addresses_with_info(rich.addresses_with_info(), network),
            ))
        } else {
            crate::error::promote_address_nonce_error(e)
                .unwrap_or_else(|| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))
        }
    };

    match state_transition.broadcast(sdk, None).await {
        Ok(()) => {}
        Err(e) if broadcast_definitely_failed(&e) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Failed,
                None,
            )
            .await;
            return Err(enrich(&e));
        }
        Err(e) => {
            warn!(
                account,
                error = %e,
                "Shield broadcast returned no verdict; the transition may have been \
                 admitted — falling through to the result wait"
            );
        }
    }

    // A shield proof authenticates the input addresses' post-state, not this
    // exact shield's execution — accept the affected-state snapshot; a
    // consensus rejection still surfaces as an error here.
    if let Err(wait_err) = state_transition
        .wait_for_affected_state::<StateTransitionProofResult>(sdk, None)
        .await
    {
        if carries_consensus_rejection(&wait_err) {
            // A consensus verdict in the result: the shield definitively
            // did not execute.
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Failed,
                None,
            )
            .await;
            return Err(enrich(&wait_err));
        }
        // Ambiguous: admitted but unconfirmed. Leave the activity row
        // Pending — the scan's confirmation pass flips it when the
        // shielded note lands on-chain.
        warn!(
            account,
            error = %wait_err,
            "Shield broadcast accepted but result confirmation failed; \
             leaving the activity row pending"
        );
        return Err(PlatformWalletError::ShieldedSpendUnconfirmed {
            operation: "shield",
            reason: wait_err.to_string(),
        });
    }

    record_activity_status(
        store,
        persister,
        wallet_id,
        id,
        &pending_entry,
        ShieldedActivityStatus::Confirmed,
        None,
    )
    .await;
    info!(account, credits = amount, "Shield broadcast succeeded");
    Ok(())
}

// -------------------------------------------------------------------------
// ShieldFromAssetLock: Core L1 asset lock -> shielded pool (Type 18)
// (orchestrated entry point lives in `wallet/shielded/fund_from_asset_lock.rs`)
// -------------------------------------------------------------------------

// -------------------------------------------------------------------------
// Unshield: shielded pool -> platform address (Type 17)
// -------------------------------------------------------------------------

/// Unshield funds from `account`'s shielded notes to a
/// transparent platform address.
#[allow(clippy::too_many_arguments)]
pub async fn unshield<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    to_address: &PlatformAddress,
    amount: u64,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let views = keys.viewing_keys();
    let change_addr = default_orchard_address(&views)?;
    let id = SubwalletId::new(wallet_id, account);

    // Reserve against the 2-action floor: Orchard's BundleType::DEFAULT pads single-spend
    // bundles to 2 actions, and the builder prices the fee at spends.len().max(2). Reserving
    // for 1 would under-fee a single-note transition and the builder would reject it locally.
    // Unshield is carved with `compute_shielded_unshield_fee` (the base fee PLUS the flat storage
    // cost of the single `AddBalanceToAddress` write crediting the net to the output address), so
    // reserve against `ShieldedFeeKind::Unshield` — reserving the base fee here would under-fund the
    // address-write cost and the builder would reject the spend (and the `fee_used == exact_fee`
    // debug assert below would fire).
    let (selected_notes, total_input, exact_fee) =
        reserve_unspent_notes(sdk, store, id, amount, 2, ShieldedFeeKind::Unshield).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        "Unshield"
    );

    // From here on every error path must release the reservation taken
    // by `reserve_unspent_notes` — except the ambiguous
    // `ShieldedSpendUnconfirmed` one, which intentionally leaves it in
    // place (see the outer match below).
    //
    // `pending_entry` is recorded once the transition is built (so we
    // have its output cmxs) and flipped to Confirmed / Failed in the
    // outer match. It lives outside the async block so the flip can see
    // it. A build failure leaves it `None` and records nothing.
    let mut pending_entry = None;
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
        // Capture the recorded anchor before the builder consumes it, so a
        // broadcast-accepted-but-unconfirmed spend can be auto-released once
        // this anchor is pruned from Platform's recorded set.
        let anchor_bytes = anchor.to_bytes();

        // The builder computes and returns the fee authoritatively; `exact_fee` (== the
        // minimum) was already used above for note reservation.
        let (state_transition, fee_used) = build_unshield_transition(
            spends,
            *to_address,
            amount,
            &change_addr,
            &keys.full_viewing_key,
            &keys.spend_auth_key,
            anchor,
            prover,
            [0u8; 36],
            sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
        // The builder's fee and the wallet's reserved `exact_fee` both come from
        // compute_shielded_unshield_fee with the same action count; lock that they agree.
        debug_assert_eq!(
            fee_used, exact_fee,
            "builder fee must match the reserved unshield fee"
        );

        // Live activity: a confirmed Unshield is `direction out`,
        // counterparty = the 21-byte serialized PlatformAddress, exact
        // fee = the builder's metered fee.
        pending_entry = record_pending_activity(
            store,
            persister,
            wallet_id,
            id,
            &views,
            LiveEntryParams {
                kind: ShieldedActivityKind::Unshield,
                direction: ShieldedDirection::Out,
                amount,
                fee: Some(fee_used),
                counterparty: Some(to_address.to_bytes()),
                memo: None,
                actions: shielded_actions(&state_transition),
                spent_notes: &selected_notes,
            },
        )
        .await;
        arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await;

        trace!("Unshield: state transition built, broadcasting...");
        broadcast_shielded_spend_with_redrive(
            sdk,
            store,
            id,
            &pending_entry,
            anchor_bytes,
            &selected_notes,
            &state_transition,
            "unshield",
        )
        .await
    }
    .await;

    match result {
        Ok(()) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Confirmed,
                None,
            )
            .await;
            // Broadcast already succeeded; spent-state bookkeeping is
            // best-effort. Surfacing a local write failure as a send
            // failure here would invite duplicate retries — the next
            // note scan reconciles any drift (scan-based spend
            // detection re-marks the note from its on-chain nullifier).
            //
            // No double-spend follows from this downgrade: the
            // authoritative no-reuse guarantee is the on-chain nullifier
            // set, not this local mark. Worst case, before the next note
            // scan runs the note is re-selected and a second spend is
            // built + proven, then rejected at broadcast with a
            // nullifier-already-used error — wasted ~30 s proof, never
            // fund loss. (`pending_nullifiers` is in-memory only, so it
            // does not protect across a process restart in this window;
            // the on-chain set does.)
            if let Err(e) = finalize_pending(store, persister, wallet_id, id, &selected_notes).await
            {
                warn!(
                    account,
                    error = %e,
                    "Unshield broadcast succeeded but local spent-state update failed; \
                     will heal on next sync"
                );
            }
            info!(account, credits = amount, "Unshield broadcast succeeded");
            Ok(())
        }
        // The broadcast was accepted but its execution result couldn't be
        // confirmed: the spend may well have executed, so do NOT release
        // the reservation. `pending_nullifiers` is in-memory only — if the
        // spend landed, the next nullifier sync's `mark_spent` promotes the
        // notes to spent (clearing the reservation); if it never landed, an
        // app restart drops the reservation and frees the notes. Releasing
        // them now would invite re-selecting notes whose nullifiers may
        // already be consumed on chain.
        //
        // Activity entry: leave it `Pending`. Like the note reservation,
        // the outcome is genuinely unknown here — a later scan that finds
        // the spend will flip the row to Confirmed (its id matches), and
        // until then surfacing it as Pending is honest.
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => Err(e),
        Err(e) => {
            // Definitive failure: the spend never executed. Mark the
            // activity row Failed (upsert by id) before releasing notes.
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Failed,
                None,
            )
            .await;
            cancel_pending(store, id, &selected_notes).await;
            Err(e)
        }
    }
}

// -------------------------------------------------------------------------
// Transfer: shielded pool -> shielded pool (Type 16)
// -------------------------------------------------------------------------

/// Transfer funds privately from `account`'s shielded notes to
/// another Orchard payment address.
#[allow(clippy::too_many_arguments)]
pub async fn transfer<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    to_address: &PaymentAddress,
    amount: u64,
    memo: [u8; 36],
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let recipient_addr = payment_address_to_orchard(to_address)?;
    let views = keys.viewing_keys();
    let change_addr = default_orchard_address(&views)?;
    let id = SubwalletId::new(wallet_id, account);

    // ShieldedTransfer is carved with the base `compute_minimum_shielded_fee`, so reserve
    // against `ShieldedFeeKind::Base`.
    let (selected_notes, total_input, exact_fee) =
        reserve_unspent_notes(sdk, store, id, amount, 2, ShieldedFeeKind::Base).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        "Shielded transfer"
    );

    let mut pending_entry = None;
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
        // Capture the recorded anchor before the builder consumes it, so a
        // broadcast-accepted-but-unconfirmed spend can be auto-released once
        // this anchor is pruned from Platform's recorded set.
        let anchor_bytes = anchor.to_bytes();

        // The builder computes and returns the fee authoritatively; `exact_fee` (== the
        // minimum) was already used above for note reservation.
        let (state_transition, fee_used) = build_shielded_transfer_transition(
            spends,
            &recipient_addr,
            amount,
            &change_addr,
            &keys.full_viewing_key,
            &keys.spend_auth_key,
            anchor,
            prover,
            memo,
            sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
        // The builder's fee and the wallet's reserved `exact_fee` both come from
        // compute_minimum_shielded_fee with the same action count; lock that they agree.
        debug_assert_eq!(
            fee_used, exact_fee,
            "builder fee must match the reserved minimum fee"
        );

        // Live activity: a confirmed transfer is a `Sent` (direction out),
        // counterparty = the recipient's 43-byte raw Orchard address, memo
        // attached when non-zero.
        pending_entry = record_pending_activity(
            store,
            persister,
            wallet_id,
            id,
            &views,
            LiveEntryParams {
                kind: ShieldedActivityKind::Sent,
                direction: ShieldedDirection::Out,
                amount,
                fee: Some(fee_used),
                counterparty: Some(to_address.to_raw_address_bytes().to_vec()),
                memo: non_zero_memo(&memo),
                actions: shielded_actions(&state_transition),
                spent_notes: &selected_notes,
            },
        )
        .await;
        arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await;

        trace!("Shielded transfer: state transition built, broadcasting...");
        broadcast_shielded_spend_with_redrive(
            sdk,
            store,
            id,
            &pending_entry,
            anchor_bytes,
            &selected_notes,
            &state_transition,
            "transfer",
        )
        .await
    }
    .await;

    match result {
        Ok(()) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Confirmed,
                None,
            )
            .await;
            // Best-effort post-broadcast bookkeeping (see unshield).
            if let Err(e) = finalize_pending(store, persister, wallet_id, id, &selected_notes).await
            {
                warn!(
                    account,
                    error = %e,
                    "Shielded transfer broadcast succeeded but local spent-state update \
                     failed; will heal on next sync"
                );
            }
            info!(
                account,
                credits = amount,
                "Shielded transfer broadcast succeeded"
            );
            Ok(())
        }
        // Ambiguous post-broadcast confirmation failure: leave the
        // reservation (and the Pending activity row) in place — a later
        // scan flips it to Confirmed (see `unshield`'s outer match).
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => Err(e),
        Err(e) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Failed,
                None,
            )
            .await;
            cancel_pending(store, id, &selected_notes).await;
            Err(e)
        }
    }
}

// -------------------------------------------------------------------------
// Withdraw: shielded pool -> Core L1 address (Type 19)
// -------------------------------------------------------------------------

/// Withdraw funds from `account`'s shielded notes to a Core L1 address.
#[allow(clippy::too_many_arguments)]
pub async fn withdraw<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    to_address: &dashcore::Address,
    amount: u64,
    core_fee_per_byte: u32,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let views = keys.viewing_keys();
    let change_addr = default_orchard_address(&views)?;
    let id = SubwalletId::new(wallet_id, account);
    let output_script = CoreScript::from_bytes(to_address.script_pubkey().to_bytes());

    // Reserve against the 2-action floor: Orchard's BundleType::DEFAULT pads single-spend
    // bundles to 2 actions, and the builder prices the fee at spends.len().max(2). Reserving
    // for 1 would under-fee a single-note transition and the builder would reject it locally.
    // ShieldedWithdrawal is carved with `compute_shielded_withdrawal_fee` (the base fee PLUS the
    // flat Core withdrawal-document storage cost), so reserve against
    // `ShieldedFeeKind::Withdrawal` — reserving the base fee here would under-fund the document
    // cost and the builder would reject the spend (and the `fee_used == exact_fee` debug assert
    // below would fire).
    let (selected_notes, total_input, exact_fee) =
        reserve_unspent_notes(sdk, store, id, amount, 2, ShieldedFeeKind::Withdrawal).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        "Shielded withdrawal"
    );

    // Capture the Core output script bytes for the activity counterparty
    // (the same bytes that fed `output_script` above) before the builder
    // consumes `output_script`.
    let counterparty_script = to_address.script_pubkey().to_bytes();

    let mut pending_entry = None;
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
        // Capture the recorded anchor before the builder consumes it, so a
        // broadcast-accepted-but-unconfirmed spend can be auto-released once
        // this anchor is pruned from Platform's recorded set.
        let anchor_bytes = anchor.to_bytes();

        // The builder computes and returns the fee authoritatively; `exact_fee` (== the
        // minimum) was already used above for note reservation.
        let (state_transition, fee_used) = build_shielded_withdrawal_transition(
            spends,
            amount,
            output_script,
            core_fee_per_byte,
            // Consensus pins shielded-withdrawal pooling to Never (validate_structure).
            Pooling::Never,
            &change_addr,
            &keys.full_viewing_key,
            &keys.spend_auth_key,
            anchor,
            prover,
            [0u8; 36],
            sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
        // The builder's fee and the wallet's reserved `exact_fee` both come from
        // compute_shielded_withdrawal_fee with the same action count; lock that they agree.
        debug_assert_eq!(
            fee_used, exact_fee,
            "builder fee must match the reserved withdrawal fee"
        );

        // Live activity: a Withdrawal (direction out), counterparty = the
        // Core output script bytes, exact metered fee.
        pending_entry = record_pending_activity(
            store,
            persister,
            wallet_id,
            id,
            &views,
            LiveEntryParams {
                kind: ShieldedActivityKind::Withdrawal,
                direction: ShieldedDirection::Out,
                amount,
                fee: Some(fee_used),
                counterparty: Some(counterparty_script.clone()),
                memo: None,
                actions: shielded_actions(&state_transition),
                spent_notes: &selected_notes,
            },
        )
        .await;
        arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await;

        trace!("Shielded withdrawal: state transition built, broadcasting...");
        broadcast_shielded_spend_with_redrive(
            sdk,
            store,
            id,
            &pending_entry,
            anchor_bytes,
            &selected_notes,
            &state_transition,
            "withdraw",
        )
        .await
    }
    .await;

    match result {
        Ok(()) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Confirmed,
                None,
            )
            .await;
            // Best-effort post-broadcast bookkeeping (see unshield).
            if let Err(e) = finalize_pending(store, persister, wallet_id, id, &selected_notes).await
            {
                warn!(
                    account,
                    error = %e,
                    "Shielded withdrawal broadcast succeeded but local spent-state update \
                     failed; will heal on next sync"
                );
            }
            info!(
                account,
                credits = amount,
                "Shielded withdrawal broadcast succeeded"
            );
            Ok(())
        }
        // Ambiguous post-broadcast confirmation failure: leave the
        // reservation (and the Pending activity row) in place — a later
        // scan flips it to Confirmed (see `unshield`'s outer match).
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => Err(e),
        Err(e) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Failed,
                None,
            )
            .await;
            cancel_pending(store, id, &selected_notes).await;
            Err(e)
        }
    }
}

// -------------------------------------------------------------------------
// IdentityCreateFromShieldedPool: shielded pool -> brand-new identity (Type 20)
// -------------------------------------------------------------------------

/// Create a brand-new Platform identity funded directly from `account`'s shielded notes.
///
/// Spends notes covering `denomination` (a member of the versioned exit-denomination set); the whole
/// denomination leaves the pool (`value_balance == denomination` EXACTLY, the ShieldedTransfer
/// exact-equality model) and the metered fee is taken FROM the denomination at execution, so the new
/// identity is created holding `denomination - total_fee`. Any spent value above the denomination
/// re-enters the pool as a single change note to `account`'s default Orchard address.
///
/// `public_keys` is the new identity's key set (each entry is the `IdentityPublicKey` and its
/// `IdentityPublicKeyInCreation` form); `identity_signer` produces each key's proof-of-possession
/// signature over the transition's signable bytes. Authorization is 100% the Orchard proof +
/// per-action spend-auth signatures + binding signature (which commits the derived id + denomination
/// + full key set) + the per-key PoP — there is NO platform identity signature.
///
/// Returns the new identity's id (`double_sha256(sorted nullifiers)`, derived deterministically
/// from the spent notes' nullifiers) together with the proof-verified [`Identity`] returned by the
/// SDK broadcast. The caller registers that `Identity` in its local `IdentityManager` so the host
/// persister emits the row, mirroring the address-funded registration path.
#[allow(clippy::too_many_arguments)]
pub async fn identity_create_from_shielded_pool<S, P, IS>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    public_keys: Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)>,
    denomination: u64,
    send_to_address_on_creation_failure: PlatformAddress,
    identity_signer: &IS,
    prover: &P,
) -> Result<(Identifier, Identity), PlatformWalletError>
where
    S: ShieldedStore,
    P: OrchardProver,
    IS: Signer<IdentityPublicKey>,
{
    if public_keys.is_empty() {
        return Err(PlatformWalletError::ShieldedBuildError(
            "identity-create-from-shielded-pool requires at least one public key".to_string(),
        ));
    }
    let views = keys.viewing_keys();
    let change_addr = default_orchard_address(&views)?;
    let id = SubwalletId::new(wallet_id, account);
    let num_keys = public_keys.len();

    // Exact-equality model: reserve notes covering the denomination itself (NOT denomination + fee
    // — the fee is metered FROM the denomination at execution). The reservation also gates on
    // `denomination > predicted_fee` so the new identity can't be created with a non-positive
    // balance. Orchard's BundleType::DEFAULT pads single-spend bundles to a 2-action floor.
    let (selected_notes, total_input, predicted_fee) =
        reserve_unspent_notes_for_denomination(sdk, store, id, denomination, 2, num_keys).await?;

    info!(
        account,
        denomination,
        predicted_fee,
        inputs = selected_notes.len(),
        total_input,
        keys = num_keys,
        "IdentityCreateFromShieldedPool"
    );

    // Snapshot the submitted `IdentityPublicKey` halves keyed by their `KeyID` BEFORE the build
    // consumes `public_keys`. This is the canonical record of the key set the transition commits to
    // (the binding signature covers it), so it's the defensive fallback if the proof-verified
    // identity comes back with an empty `public_keys()` map — same pattern register_from_addresses
    // uses for its address-funded `put_*` stub.
    let submitted_public_keys: BTreeMap<u32, IdentityPublicKey> = public_keys
        .iter()
        .map(|(key, _)| (key.id(), key.clone()))
        .collect();

    // From here on every error path must release the reservation taken above — except the
    // ambiguous `ShieldedBroadcastUnconfirmed` one, which intentionally leaves it in place
    // (see the outer match below).
    //
    // `pending_entry` is recorded once the bundle is built (so we know the
    // identity id + output cmxs) and flipped to Confirmed / Failed in the
    // outer match; it lives here so the flip can see it.
    let mut pending_entry = None;
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
        // Capture the recorded anchor before the builder consumes it, so a
        // broadcast-accepted-but-unconfirmed create can be auto-released once
        // this anchor is pruned from Platform's recorded set.
        let anchor_bytes = anchor.to_bytes();

        let build = build_identity_create_from_shielded_pool_transition(
            public_keys,
            denomination,
            send_to_address_on_creation_failure,
            spends,
            &change_addr,
            &keys.full_viewing_key,
            &keys.spend_auth_key,
            anchor,
            prover,
            identity_signer,
            [0u8; 36],
            sdk.version(),
        )
        .await
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        let identity_id = build.identity_id;

        // Live activity: IdentityCreate carries the created identity id.
        // The display amount is the denomination LESS the metered fee (the
        // credits the new identity is created holding); the exact fee is the
        // predicted fee reserved above (the builder meters the same value).
        // The output cmxs are taken from the bundle the transition is built
        // from (the change note re-entering the pool, if any).
        pending_entry = record_pending_activity(
            store,
            persister,
            wallet_id,
            id,
            &views,
            LiveEntryParams {
                kind: ShieldedActivityKind::IdentityCreate {
                    identity_id: identity_id.to_buffer(),
                },
                direction: ShieldedDirection::Out,
                amount: denomination.saturating_sub(predicted_fee),
                fee: Some(predicted_fee),
                counterparty: Some(identity_id.to_buffer().to_vec()),
                memo: None,
                actions: &build.bundle.actions,
                spent_notes: &selected_notes,
            },
        )
        .await;
        arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await;

        trace!("IdentityCreateFromShieldedPool: built, broadcasting via SDK...");
        // Stage the broadcast and the result-wait SEPARATELY (instead of one `broadcast_and_wait`)
        // so the two failure shapes can be told apart:
        //   - a broadcast-time rejection (relay/CheckTx refused the tx) means it never executed, so
        //     releasing the note reservations is correct;
        //   - a post-broadcast wait failure is AMBIGUOUS — the relay accepted the tx and it may well
        //     have executed; treating it as "unregistered" + releasing the notes is the
        //     orphaned-identity + double-spend hazard this split exists to avoid.
        // The transition is built once from the PoP-signed keys + bundle params (preserving the
        // per-key signatures); the binding signature already committed `identity_id`, the
        // denomination, and the full key set.
        let st = sdk
            .identity_create_from_shielded_pool_transition(
                build.public_keys,
                denomination,
                send_to_address_on_creation_failure,
                build.bundle,
            )
            .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        // Broadcast (relay-ACK only). Definitive failures — a consensus-verdict CheckTx
        // rejection, or a transport failure that proves nothing was delivered (see
        // `broadcast_definitely_failed`; connect-refused/offline is the common case) — map to
        // `ShieldedBroadcastFailed` and let the outer match release the reservation, exactly as
        // before. No-verdict failures (`AlreadyExists` proves the transition IS already in the
        // mempool or on chain after a lost-ACK retry; timeouts merely allow it) fall through to
        // the result wait below, whose ambiguous arm already owns the fetch-by-derived-id
        // fallback — so the in-flight tx gets confirmed (or held as unconfirmed) instead of
        // being reported as failed.
        match st.broadcast(sdk, None).await {
            Ok(()) => {}
            Err(e) if broadcast_definitely_failed(&e) => {
                return Err(PlatformWalletError::ShieldedBroadcastFailed(e.to_string()));
            }
            Err(e) => {
                warn!(
                    derived_id = %identity_id,
                    error = %e,
                    "IdentityCreateFromShieldedPool: broadcast returned no verdict; the \
                     transition may have been admitted — falling through to the result wait"
                );
            }
        }

        // Wait for proven execution. Classify the failure:
        //   - `StateTransitionBroadcastError` WITH a consensus `cause` = Platform DEFINITIVELY
        //     reported the transition's own execution error (it ran and was rejected on its
        //     merits; the serialized consensus error is the verdict). The identity does not
        //     exist; keep today's behavior (release the reservation via
        //     `ShieldedBroadcastFailed`).
        //   - any other error (DriveProofError / Proof / InvalidProvedResponse / TimeoutReached /
        //     DapiClientError / …) = AMBIGUOUS: the broadcast was accepted and the transition may
        //     have executed even though we couldn't fetch/verify its result proof (this is exactly
        //     the #3859 result-proof incident). That includes cause-less
        //     `StateTransitionBroadcastError`s — DAPI encodes its own wait-side failures
        //     (timeouts, internal errors) that way, with empty consensus data. Fall back to
        //     fetching the identity by its pre-derived id before deciding it doesn't exist.
        // An identity-create-from-shielded-pool proof authenticates the spent
        // nullifiers and resulting identity as a snapshot; it cannot bind the
        // complete Orchard request, so accept the affected-state outcome.
        let proof_result = match st
            .wait_for_affected_state::<StateTransitionProofResult>(sdk, None)
            .await
        {
            Ok(result) => result,
            Err(dash_sdk::Error::StateTransitionBroadcastError(e)) if e.cause.is_some() => {
                return Err(PlatformWalletError::ShieldedBroadcastFailed(e.to_string()));
            }
            Err(wait_err) => {
                warn!(
                    derived_id = %identity_id,
                    error = %wait_err,
                    "IdentityCreateFromShieldedPool: broadcast accepted but result confirmation \
                     failed; falling back to fetching the identity by its derived id"
                );
                // Fetch the identity directly. The id is committed in the sighash and derived
                // deterministically from the spent nullifiers, so if the transition landed the row
                // is queryable. A freshly-included identity can take a moment to index on the DAPI
                // node we hit, so retry a few times with a short, fixed backoff before concluding
                // it's truly absent — long enough to ride out routine indexing/replica lag, short
                // enough not to wedge the caller's UI for minutes.
                match fetch_identity_with_retries(sdk, identity_id).await {
                    Some(mut identity) => {
                        info!(
                            derived_id = %identity_id,
                            "IdentityCreateFromShieldedPool: result confirmation failed but the \
                             identity was found on chain by its derived id; treating as success"
                        );
                        // Same defensive empty-`public_keys` fill as the proven-result path below,
                        // so downstream auth-key checks see the committed key set immediately.
                        if identity.public_keys().is_empty() {
                            identity.set_public_keys(submitted_public_keys.clone());
                        }
                        return Ok::<(Identifier, Identity), PlatformWalletError>((
                            identity.id(),
                            identity,
                        ));
                    }
                    None => {
                        // Arm the persisted re-drive before surfacing the
                        // ambiguity: the sync-time pass re-checks the
                        // nullifiers, then re-broadcasts this
                        // byte-identical transition up to
                        // MAX_REDRIVE_ATTEMPTS times.
                        arm_redrive_record(
                            store,
                            id,
                            &pending_entry,
                            anchor_bytes,
                            &selected_notes,
                            &st,
                            "identity_create",
                        )
                        .await;
                        return Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
                            identity_id,
                            reason: wait_err.to_string(),
                        });
                    }
                }
            }
        };


        // Pull the verified `Identity` out of the proof result. The expected variant is
        // `VerifiedIdentityWithShieldedNullifiers`; if drive-abci ever returns a different one the
        // broadcast still SUCCEEDED, so we don't turn it into an error — we synthesize the identity
        // from the derived id + submitted keys (the binding signature committed both) and warn, so
        // the local row is still created.
        let identity = match proof_result {
            StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers(
                mut identity,
                _nullifiers,
            ) => {
                // The proof-verified id is authoritative: it's recomputed from the proven nullifier
                // set, while `identity_id` was derived pre-broadcast. They should match (the derived
                // id is committed in the sighash), but trust the verified one.
                if identity.id() != identity_id {
                    warn!(
                        derived_id = %identity_id,
                        verified_id = %identity.id(),
                        "IdentityCreateFromShieldedPool: derived id differs from proof-verified id; \
                         using the proof-verified id"
                    );
                }
                // Defensive: a proof result can hand back an identity whose `public_keys` map is
                // empty. Fill it from the submitted set so downstream auth-key checks see the keys
                // immediately without waiting for the next identity-fetch round (the transition
                // committed exactly these keys, so id reproducibility is preserved).
                if identity.public_keys().is_empty() {
                    identity.set_public_keys(submitted_public_keys);
                }
                identity
            }
            other => {
                warn!(
                    derived_id = %identity_id,
                    result = %other,
                    "IdentityCreateFromShieldedPool: unexpected proof-result variant; synthesizing \
                     the identity from the derived id + submitted keys so the local row still lands"
                );
                Identity::new_with_id_and_keys(
                    identity_id,
                    submitted_public_keys,
                    sdk.version(),
                )
                .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?
            }
        };

        Ok::<(Identifier, Identity), PlatformWalletError>((identity.id(), identity))
    }
    .await;

    match result {
        Ok((identity_id, identity)) => {
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                &pending_entry,
                ShieldedActivityStatus::Confirmed,
                None,
            )
            .await;
            // Best-effort post-broadcast bookkeeping (see `unshield`): mark the spent notes so the
            // local balance reflects the exit immediately; any drift heals on the next nullifier
            // sync. The on-chain nullifier set — not this local mark — is the authoritative
            // no-reuse guarantee.
            if let Err(e) = finalize_pending(store, persister, wallet_id, id, &selected_notes).await
            {
                warn!(
                    account,
                    error = %e,
                    "IdentityCreateFromShieldedPool broadcast succeeded but local spent-state \
                     update failed; will heal on next sync"
                );
            }
            info!(
                account,
                denomination,
                identity_id = %identity_id,
                "IdentityCreateFromShieldedPool broadcast succeeded"
            );
            Ok((identity_id, identity))
        }
        // The broadcast was accepted but its result couldn't be confirmed and the identity wasn't
        // found by a direct fetch. Do NOT `cancel_pending` here: `pending_nullifiers` is in-memory
        // only (see `SubwalletState`, "never persisted; the next sync after a crash reconciles"),
        // and `mark_spent` during nullifier sync clears matching reservations. So if the transition
        // actually executed, the next sync promotes these notes to spent; if it truly never landed,
        // an app restart drops the in-memory reservation and frees them. Releasing them now would
        // invite double-spend attempts against notes that may already be consumed on chain — the
        // very hazard this variant exists to prevent. The activity row likewise stays `Pending`
        // (a later scan that finds the identity's change note flips it to Confirmed).
        Err(e @ PlatformWalletError::ShieldedBroadcastUnconfirmed { .. }) => Err(e),
        Err(e) => {
            if error_releases_note_reservation(&e) {
                // Definitive failure: the identity was never created. Mark
                // the activity row Failed (upsert by id) before releasing.
                record_activity_status(
                    store,
                    persister,
                    wallet_id,
                    id,
                    &pending_entry,
                    ShieldedActivityStatus::Failed,
                    None,
                )
                .await;
                cancel_pending(store, id, &selected_notes).await;
            }
            Err(e)
        }
    }
}

/// Whether a failed identity-create should release the notes reserved for it.
///
/// `false` ONLY for [`PlatformWalletError::ShieldedBroadcastUnconfirmed`]: the broadcast was
/// accepted and the transition may have executed, so the reservation must be retained. Releasing it
/// now would invite double-spend attempts against notes that may already be consumed on chain — the
/// very hazard that variant exists to prevent. `pending_nullifiers` is in-memory only (see
/// `SubwalletState`, "never persisted; the next sync after a crash reconciles") and `mark_spent`
/// during nullifier sync clears matching reservations, so if the transition actually executed the
/// next sync promotes these notes to spent; if it truly never landed, an app restart drops the
/// in-memory reservation and frees them.
///
/// Everything else is a definitive pre-execution / build / rejection failure: the spend never
/// happened, so the reservation must be released.
fn error_releases_note_reservation(e: &PlatformWalletError) -> bool {
    !matches!(e, PlatformWalletError::ShieldedBroadcastUnconfirmed { .. })
}

/// Number of times [`identity_create_from_shielded_pool`] re-fetches the new identity by its
/// derived id after a post-broadcast result-confirmation failure, before declaring the broadcast
/// unconfirmed.
const IDENTITY_CREATE_FETCH_RETRIES: usize = 4;

/// Fixed backoff between identity fetch attempts. Four attempts ~3 s apart (~9 s of fetch window
/// total) is enough to ride out routine DAPI indexing / replica lag for a freshly-included identity
/// without wedging the caller's UI for minutes.
const IDENTITY_CREATE_FETCH_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// Fetch an identity by id with a few fixed-interval retries.
///
/// Used only on the ambiguous post-broadcast path: the result-proof fetch failed, so we don't know
/// whether the transition executed. The identity id is derived deterministically from the spent
/// notes' nullifiers and committed in the transition sighash, so a successful fetch is positive
/// proof the transition landed. Returns `Some(identity)` on the first hit, or `None` if every
/// attempt comes back empty or errors (transport hiccup, not-yet-indexed, …) — the caller then
/// surfaces `ShieldedBroadcastUnconfirmed` rather than a hard failure.
async fn fetch_identity_with_retries(
    sdk: &Arc<dash_sdk::Sdk>,
    identity_id: Identifier,
) -> Option<Identity> {
    use dash_sdk::platform::Fetch;

    for attempt in 0..IDENTITY_CREATE_FETCH_RETRIES {
        match Identity::fetch(sdk, identity_id).await {
            Ok(Some(identity)) => return Some(identity),
            Ok(None) => {
                trace!(
                    %identity_id,
                    attempt,
                    "IdentityCreateFromShieldedPool confirmation fetch: not found yet"
                );
            }
            Err(e) => {
                trace!(
                    %identity_id,
                    attempt,
                    error = %e,
                    "IdentityCreateFromShieldedPool confirmation fetch errored; will retry"
                );
            }
        }
        // Skip the trailing sleep after the final attempt — nothing follows it.
        if attempt + 1 < IDENTITY_CREATE_FETCH_RETRIES {
            tokio::time::sleep(IDENTITY_CREATE_FETCH_RETRY_DELAY).await;
        }
    }
    None
}

// -------------------------------------------------------------------------
// Internal helpers (free fns)
// -------------------------------------------------------------------------

/// Convert `keys`'s default `PaymentAddress` to an `OrchardAddress`.
fn default_orchard_address(
    keys: &AccountViewingKeys,
) -> Result<OrchardAddress, PlatformWalletError> {
    payment_address_to_orchard(&keys.default_address)
}

/// Checkpoint depths probed for a Platform-recorded anchor. Kept equal to the
/// commitment tree's `max_checkpoints` retention (the store is opened with
/// `100` — see `PlatformWalletManager`) so the probe reaches every checkpoint
/// the tree still holds and no further: deeper checkpoints are pruned, so
/// probing past this bound is wasted work. Coupled by convention — if that
/// retention changes, update this in lockstep.
const MAX_ANCHOR_PROBE_DEPTH: usize = 100;

/// Extract `SpendableNote` structs with Merkle witnesses and an anchor
/// Platform has recorded.
///
/// A shielded spend's proof is accepted only if its anchor is a
/// commitment-tree root Platform recorded (`validate_anchor_exists`).
/// Platform records one anchor per block, but an index-chunk sync routinely
/// leaves the wallet's tree mid-block, so the depth-0 (current) root is
/// frequently a value Platform never recorded — building against it
/// unconditionally is what made such spends fail and never land.
///
/// This fetches Platform's recorded anchor set (outside the store lock), then
/// selects the shallowest checkpoint depth whose root is in that set — depth 0
/// being the fully-synced fast path — witnessing every note at that same depth
/// so the anchor and the authentication paths agree (the builder derives the
/// anchor from the witnesses via `MerklePath::root`, so a per-note disagreement
/// would surface downstream as `AnchorMismatch`). When no probed depth has a
/// recorded root it returns the retryable
/// [`PlatformWalletError::ShieldedNoRecordedAnchor`] rather than broadcasting a
/// spend Platform is guaranteed to reject.
async fn extract_spends_and_anchor<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    notes: &[ShieldedNote],
) -> Result<(Vec<SpendableNote>, Anchor), PlatformWalletError> {
    // Nothing selected — fail before the network round-trip.
    if notes.is_empty() {
        return Err(PlatformWalletError::ShieldedBuildError(
            "no spendable notes selected — anchor undefined".to_string(),
        ));
    }

    // Fetch the recorded anchor set OUTSIDE the store lock so the network
    // round-trip doesn't serialize with other store users, and so the lock is
    // held only for the mutually-consistent depth/witness probe below.
    let dash_sdk::query_types::ShieldedAnchors(recorded_anchors) =
        dash_sdk::query_types::ShieldedAnchors::fetch_current(sdk).await?;
    let recorded: HashSet<[u8; 32]> = recorded_anchors.into_iter().collect();

    // Hold a single read lock across the whole probe so the checkpoint depths
    // and the per-note witnesses stay mutually consistent: a concurrent sync
    // checkpointing mid-probe would otherwise shift the depth indices out from
    // under us.
    let store = store.read().await;
    select_recorded_spends(&*store, notes, &recorded)
}

/// Pick the shallowest checkpoint depth whose tree root is in `recorded`,
/// witnessing every note at that depth. Pure (no SDK, no async), so the depth
/// walk can be unit-tested against a real commitment tree.
///
/// Depth 0 is the current tree state (the fully-synced fast path). Deeper
/// checkpoints are older and hold strictly fewer positions, so the probe stops
/// as soon as a selected note is no longer witnessable at a depth — no deeper
/// checkpoint could contain it. Returns
/// [`PlatformWalletError::ShieldedNoRecordedAnchor`] when no probed depth has a
/// recorded root (a clean, retryable outcome — nothing is broadcast).
fn select_recorded_spends<S: ShieldedStore>(
    store: &S,
    notes: &[ShieldedNote],
    recorded: &HashSet<[u8; 32]>,
) -> Result<(Vec<SpendableNote>, Anchor), PlatformWalletError> {
    use grovedb_commitment_tree::ExtractedNoteCommitment;

    // Deserialize each note and decode its commitment ONCE — both are
    // independent of the checkpoint depth, so hoisting them out of the probe
    // keeps the depth walk cheap (each probed depth only re-witnesses).
    let prepared: Vec<(u64, grovedb_commitment_tree::Note, ExtractedNoteCommitment)> = notes
        .iter()
        .map(|note| {
            let orchard_note = deserialize_note(&note.note_data).ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "Failed to deserialize note at position {}",
                    note.position
                ))
            })?;
            let cmx = ExtractedNoteCommitment::from_bytes(&note.cmx)
                .into_option()
                .ok_or_else(|| {
                    PlatformWalletError::ShieldedBuildError(format!(
                        "invalid stored cmx for note at position {}",
                        note.position
                    ))
                })?;
            Ok((note.position, orchard_note, cmx))
        })
        .collect::<Result<_, PlatformWalletError>>()?;

    // Build every selected note's `SpendableNote` plus the shared anchor at a
    // single checkpoint `depth`.
    //
    // `strict` (depth 0 only): a missing/failed witness is a hard
    // `ShieldedMerkleWitnessUnavailable` (the note is expected to be witnessable
    // at the current tip). At depth > 0, `Ok(None)` means the note post-dates
    // this older checkpoint, so the depth is unusable — return `Ok(None)` and
    // let the caller stop probing deeper; a genuine store `Err` (poisoned mutex,
    // IO, tree corruption) is logged — the probe would otherwise discard the
    // message — and likewise treated as an unusable depth rather than aborting,
    // so a transient read can't strand a spend a shallower depth already
    // covered. An anchor disagreement across notes is always a hard error (the
    // spend builder would reject it downstream).
    let build_at_depth = |depth: usize,
                          strict: bool|
     -> Result<Option<(Vec<SpendableNote>, Anchor)>, PlatformWalletError> {
        let mut spends = Vec::with_capacity(prepared.len());
        let mut anchor: Option<Anchor> = None;
        for (position, note, cmx) in &prepared {
            let merkle_path = match store.witness_at_depth(*position, depth) {
                Ok(Some(path)) => path,
                Ok(None) if strict => {
                    return Err(PlatformWalletError::ShieldedMerkleWitnessUnavailable(format!(
                        "no witness available for note at position {position} (not marked, or pruned past this position)"
                    )));
                }
                Err(e) if strict => {
                    return Err(PlatformWalletError::ShieldedMerkleWitnessUnavailable(
                        e.to_string(),
                    ));
                }
                // depth > 0: the note isn't witnessable at this older checkpoint
                // (appended after it, or the depth doesn't exist).
                Ok(None) => return Ok(None),
                // depth > 0: a genuine store failure. Log it so the operator sees
                // it (the anchor probe otherwise swallows the message), then treat
                // the depth as unusable — never a mid-probe abort.
                Err(e) => {
                    tracing::warn!(
                        position = *position,
                        depth,
                        error = %e,
                        "shielded anchor probe: witness_at_depth failed at depth > 0; skipping depth"
                    );
                    return Ok(None);
                }
            };

            // The anchor is derived from the witness path itself
            // (`MerklePath::root(cmx)`); all selected notes must agree on it, or
            // the store handed back witnesses from different checkpoints and the
            // spend builder would reject the mismatch downstream.
            let witness_anchor = merkle_path.root(*cmx);
            match &anchor {
                None => anchor = Some(witness_anchor),
                Some(prev) if prev.to_bytes() != witness_anchor.to_bytes() => {
                    return Err(PlatformWalletError::ShieldedBuildError(format!(
                        "witness anchor mismatch across selected notes (position {position})"
                    )));
                }
                _ => {}
            }

            spends.push(SpendableNote {
                note: *note,
                merkle_path,
            });
        }

        // `notes` is non-empty (the caller checked), so `anchor` is set.
        let anchor = anchor.ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(
                "no spendable notes selected — anchor undefined".to_string(),
            )
        })?;
        Ok(Some((spends, anchor)))
    };

    // Fast path: a fully-synced wallet's depth-0 root is a recorded anchor.
    let (spends, anchor) = match build_at_depth(0, true)? {
        Some(pair) => pair,
        // Unreachable — a strict build returns `Some` or errors — but stay
        // fund-safe (a clean error, never a panic) if that invariant breaks.
        None => {
            return Err(PlatformWalletError::ShieldedMerkleWitnessUnavailable(
                "depth-0 witness probe returned no witness for a selected note".to_string(),
            ));
        }
    };
    if recorded.contains(&anchor.to_bytes()) {
        return Ok((spends, anchor));
    }

    // Otherwise walk older checkpoints newest→oldest for the shallowest
    // recorded root.
    for depth in 1..MAX_ANCHOR_PROBE_DEPTH {
        match build_at_depth(depth, false)? {
            Some((spends, anchor)) if recorded.contains(&anchor.to_bytes()) => {
                return Ok((spends, anchor));
            }
            // A root exists at this depth but Platform didn't record it — try an
            // older checkpoint.
            Some(_) => continue,
            // A selected note isn't witnessable this deep; every deeper
            // checkpoint is older still, so none can cover it either.
            None => break,
        }
    }

    Err(PlatformWalletError::ShieldedNoRecordedAnchor(
        "no recorded anchor covers the selected notes; wait for the next shielded sync".to_string(),
    ))
}

/// Mark the selected notes as spent for `id`. Also queues a
/// shielded changeset on the persister so the spent flag reaches
/// durable storage immediately rather than waiting for the next
/// note scan to rediscover the spend (scan-based spend detection).
/// Also drops any matching pending reservation so the
/// confirmed-spent state and the in-flight-spend state can't
/// disagree.
async fn mark_notes_spent<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    notes: &[ShieldedNote],
) -> Result<(), PlatformWalletError> {
    let mut changeset = ShieldedChangeSet::default();
    {
        let mut store = store.write().await;
        for note in notes {
            if store
                .mark_spent(id, &note.nullifier)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
            {
                changeset.record_nullifier_spent(id, note.nullifier);
            }
        }
    }
    queue_shielded_changeset(persister, wallet_id, changeset);
    Ok(())
}

/// Select unspent notes and reserve them against an in-flight
/// spend in one write-locked critical section.
///
/// Combining selection and reservation under a single write lock
/// is the only thing that prevents two overlapping spend calls
/// from picking the same notes: with separate read-then-write
/// phases, the second caller would observe the same
/// `unspent_notes()` between the first caller's read and write
/// and proceed to build a duplicate proof that's only rejected
/// ~30 s later at broadcast time.
///
/// The reservation is in-memory only — see
/// [`ShieldedStore::mark_pending`] for the crash-recovery note.
/// Callers must pair this with [`finalize_pending`] (on
/// broadcast success) or [`cancel_pending`] (on failure) so the
/// reservation is always released.
async fn reserve_unspent_notes<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    amount: u64,
    outputs: usize,
    fee_kind: ShieldedFeeKind,
) -> Result<(Vec<ShieldedNote>, u64, u64), PlatformWalletError> {
    let mut store = store.write().await;
    let unspent = store
        .get_unspent_notes(id)
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    let (selected, total_input, exact_fee) =
        select_notes_with_fee(&unspent, amount, outputs, fee_kind, sdk.version())?.into_owned();
    for note in &selected {
        store
            .mark_pending(id, &note.nullifier)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    }
    Ok((selected, total_input, exact_fee))
}

/// Exact-equality sibling of [`reserve_unspent_notes`] for
/// `IdentityCreateFromShieldedPool`: select + reserve notes covering exactly `denomination`
/// (the fee is metered FROM the denomination, not added to the target) in one write-locked
/// critical section, gating on `denomination > predicted_fee` via
/// [`select_notes_for_denomination`]. Returns the selected notes, total input value, and the
/// predicted fee. Callers must pair this with [`finalize_pending`] / [`cancel_pending`].
async fn reserve_unspent_notes_for_denomination<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    denomination: u64,
    min_actions: usize,
    num_keys: usize,
) -> Result<(Vec<ShieldedNote>, u64, u64), PlatformWalletError> {
    let mut store = store.write().await;
    let unspent = store
        .get_unspent_notes(id)
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    let (selected, total_input, predicted_fee) = select_notes_for_denomination(
        &unspent,
        denomination,
        min_actions,
        num_keys,
        sdk.version(),
    )?
    .into_owned();
    for note in &selected {
        store
            .mark_pending(id, &note.nullifier)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    }
    Ok((selected, total_input, predicted_fee))
}

/// Promote a successful broadcast: mark the notes spent (which
/// also clears any matching pending reservation, see
/// [`SubwalletState::mark_spent`]) and queue the changeset for
/// the host persister.
async fn finalize_pending<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    notes: &[ShieldedNote],
) -> Result<(), PlatformWalletError> {
    mark_notes_spent(store, persister, wallet_id, id, notes).await
}

/// Roll back a reservation when the broadcast / wait fails.
/// Best-effort and doesn't surface its own errors — the caller
/// is already returning the broadcast error.
async fn cancel_pending<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    notes: &[ShieldedNote],
) {
    let mut store = store.write().await;
    for note in notes {
        if let Err(e) = store.clear_pending(id, &note.nullifier) {
            tracing::warn!(
                error = %e,
                "cancel_pending: clear_pending failed; the next note scan will reconcile"
            );
        }
    }
}

/// Record the recorded `anchor` the spend was built against and the
/// linked activity entry on every selected note's reservation, so a
/// spend that ends broadcast-accepted-but-unconfirmed can be released
/// on a later sync once that anchor is pruned from Platform's recorded
/// set (see `NetworkShieldedCoordinator::release_stranded_spends`).
///
/// No-op when no activity entry was recorded (an output-less bundle —
/// unreachable for our own builders, which always carry a visible
/// output). Best-effort: a store write failure only means the
/// reservation won't self-release on a pruned anchor (it still frees
/// on the next restart), so it must never abort a spend about to
/// broadcast. A success (`finalize_pending`) or a definite failure
/// (`cancel_pending`) removes the entry, so only an ambiguous
/// unconfirmed outcome leaves it carrying the anchor.
async fn arm_pending_release<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    anchor: [u8; 32],
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    notes: &[ShieldedNote],
) {
    let Some(entry) = pending_entry else {
        return;
    };
    let mut store = store.write().await;
    for note in notes {
        if let Err(e) = store.set_pending_spend(id, &note.nullifier, anchor, entry.id) {
            warn!(
                error = %e,
                "set_pending_spend failed; this reservation won't self-release on a pruned \
                 anchor (it still frees on the next restart)"
            );
        }
    }
}

/// Maximum sync-time re-broadcast attempts for a
/// broadcast-accepted-but-unconfirmed spend before the re-drive stops
/// and the anchor-prune release backstop owns the reservation.
pub(super) const MAX_REDRIVE_ATTEMPTS: u32 = 3;

/// Broadcast a built shielded spend and, on the AMBIGUOUS outcome only
/// (`ShieldedSpendUnconfirmed` — accepted broadcast, failed result
/// wait), persist a [`PendingRedrive`] so the sync-time re-drive can
/// resolve the ambiguity actively: the next scan detects a landing via
/// the nullifiers; otherwise the byte-identical transition is
/// re-broadcast up to [`MAX_REDRIVE_ATTEMPTS`] times (fund-safe —
/// identical nullifiers cannot double-spend); only if every attempt
/// stays silent does the anchor-prune release backstop take over.
#[allow(clippy::too_many_arguments)]
async fn broadcast_shielded_spend_with_redrive<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    anchor: [u8; 32],
    notes: &[ShieldedNote],
    state_transition: &StateTransition,
    operation: &'static str,
) -> Result<(), PlatformWalletError> {
    let result = broadcast_shielded_spend(sdk, state_transition, operation).await;
    if matches!(
        &result,
        Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
    ) {
        arm_redrive_record(
            store,
            id,
            pending_entry,
            anchor,
            notes,
            state_transition,
            operation,
        )
        .await;
    }
    result
}

/// Persist the re-drivable record for an ambiguous spend. Best-effort:
/// a failure here only demotes the resolution path to the anchor-prune
/// backstop (plus restart-loss of the reservation), never fails the
/// spend call itself — the ambiguity already happened.
async fn arm_redrive_record<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    anchor: [u8; 32],
    notes: &[ShieldedNote],
    state_transition: &StateTransition,
    operation: &'static str,
) {
    use dpp::serialization::PlatformSerializable;

    let Some(entry) = pending_entry else {
        return;
    };
    let st_bytes = match state_transition.serialize_to_bytes() {
        Ok(b) => b,
        Err(e) => {
            warn!(
                operation,
                error = %e,
                "failed to serialize the unconfirmed transition; re-drive disabled for this \
                 spend (prune backstop still applies)"
            );
            return;
        }
    };
    let redrive = PendingRedrive {
        activity_id: entry.id,
        anchor,
        nullifiers: notes.iter().map(|n| n.nullifier).collect(),
        st_bytes,
        attempts: 0,
    };
    if let Err(e) = store.write().await.arm_redrive(id, redrive) {
        warn!(
            operation,
            error = %e,
            "failed to persist the redrive record; re-drive disabled for this spend (prune \
             backstop still applies)"
        );
    }
}

/// Whether an SDK error is Platform's `NullifierAlreadySpentError` —
/// on a RE-broadcast of our own byte-identical transition this means
/// the ORIGINAL broadcast executed: the nullifiers are consumed by the
/// very spend being re-driven, so it is a success signal (the next scan
/// confirms the notes spent), never a failure.
fn is_nullifier_already_spent(e: &dash_sdk::Error) -> bool {
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;

    let consensus: Option<&ConsensusError> = match e {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(c)) => Some(c),
        dash_sdk::Error::StateTransitionBroadcastError(b) => b.cause.as_ref(),
        _ => None,
    };
    matches!(
        consensus,
        Some(ConsensusError::StateError(
            StateError::NullifierAlreadySpentError(_)
        ))
    )
}

/// Pure outcome classification for one re-broadcast attempt. Extracted
/// from the redrive loop so the arm ORDER — `AlreadyExecuted` must win
/// over the generic consensus-rejection check, since
/// `NullifierAlreadySpent` is itself a consensus error — is pinned by
/// unit tests without needing a broadcast-mockable network seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedriveBroadcastOutcome {
    /// Relay accepted the re-broadcast; the next scan detects a landing.
    Accepted,
    /// `NullifierAlreadySpent`: the ORIGINAL broadcast executed — a
    /// success signal, never a failure.
    AlreadyExecuted,
    /// Any other consensus verdict: the transition can never execute.
    DefinitiveRejection,
    /// Transport noise / `AlreadyExists` / anything non-definitive.
    Inconclusive,
}

fn classify_redrive_broadcast(result: &Result<(), dash_sdk::Error>) -> RedriveBroadcastOutcome {
    match result {
        Ok(()) => RedriveBroadcastOutcome::Accepted,
        Err(e) if is_nullifier_already_spent(e) => RedriveBroadcastOutcome::AlreadyExecuted,
        Err(e) if carries_consensus_rejection(e) => RedriveBroadcastOutcome::DefinitiveRejection,
        Err(_) => RedriveBroadcastOutcome::Inconclusive,
    }
}

/// Bump a redrive attempt counter, logging (rather than discarding) a
/// persistence failure. On `Err` the durable counter did not advance and
/// the file store's persist-first ordering leaves memory untouched, so
/// the same attempt slot is retried on the next pass — the log line is
/// what makes that visible.
async fn bump_redrive_attempts_logged<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    activity_id: &[u8; 32],
) -> u32 {
    match store.write().await.bump_redrive_attempts(id, activity_id) {
        Ok(attempts) => attempts,
        Err(e) => {
            warn!(
                error = %e,
                "redrive: failed to persist the attempt counter; the attempt will be \
                 retried on the next pass"
            );
            0
        }
    }
}

/// Sync-time re-drive for `id`'s armed unconfirmed spends: for each
/// [`PendingRedrive`] whose anchor is still in Platform's `recorded`
/// set and whose attempt budget remains, re-broadcast the stored
/// byte-identical transition (relay-ACK only — the landing itself is
/// detected by the NEXT scan's nullifier reconcile) and classify:
///
/// - accepted / inconclusive → count the attempt; wait for the next scan;
/// - `NullifierAlreadySpent` → the original executed; the next scan
///   confirms — touch nothing;
/// - any other consensus verdict → provably dead NOW: release the
///   reservation and flip the activity row to Failed, hours before the
///   prune backstop would;
/// - pruned anchor / exhausted attempts → leave it to the
///   prune-backstop release pass.
///
/// Runs after the scan's spent-note reconcile (a landed spend's record
/// was already dropped by the `mark_spent` hook, so anything still
/// armed here is genuinely unresolved).
pub(super) async fn redrive_pending_spends<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    recorded: &std::collections::HashSet<[u8; 32]>,
) {
    use dpp::serialization::PlatformDeserializable;

    let redrives = match store.read().await.pending_redrives(id) {
        Ok(r) => r,
        Err(e) => {
            warn!(
                error = %e,
                "redrive: pending_redrives failed; skipping subwallet"
            );
            return;
        }
    };
    for redrive in redrives {
        // A pruned anchor is the release pass's call, not ours; an
        // exhausted budget means we've said our three pieces.
        if !recorded.contains(&redrive.anchor) || redrive.attempts >= MAX_REDRIVE_ATTEMPTS {
            continue;
        }
        let st = match StateTransition::deserialize_from_bytes(&redrive.st_bytes) {
            Ok(st) => st,
            Err(e) => {
                warn!(
                    error = %e,
                    "redrive: stored transition failed to deserialize; dropping the record \
                     (the prune backstop still frees the notes)"
                );
                if let Err(e) = store.write().await.clear_redrive(id, &redrive.activity_id) {
                    warn!(error = %e, "redrive: clear_redrive failed");
                }
                continue;
            }
        };
        let broadcast_result = st.broadcast(sdk, None).await;
        let err_display = broadcast_result
            .as_ref()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        match classify_redrive_broadcast(&broadcast_result) {
            RedriveBroadcastOutcome::Accepted => {
                let attempts = bump_redrive_attempts_logged(store, id, &redrive.activity_id).await;
                info!(
                    attempts,
                    max = MAX_REDRIVE_ATTEMPTS,
                    "redrive: re-broadcast accepted; the next scan detects the landing"
                );
            }
            RedriveBroadcastOutcome::AlreadyExecuted => {
                // Success signal — but still consume an attempt: the scan
                // normally confirms the landing and drops the record, and
                // if it lags, this arm must not re-broadcast unboundedly
                // on every pass. The cap parks the record for the scan /
                // prune passes to settle.
                let attempts = bump_redrive_attempts_logged(store, id, &redrive.activity_id).await;
                info!(
                    attempts,
                    max = MAX_REDRIVE_ATTEMPTS,
                    "redrive: transition already executed on-chain; the next scan confirms \
                     the notes spent"
                );
            }
            RedriveBroadcastOutcome::DefinitiveRejection => {
                warn!(
                    error = %err_display,
                    "redrive: definitive consensus rejection; the spend can never execute — \
                     releasing the reservation"
                );
                {
                    let mut guard = store.write().await;
                    for n in &redrive.nullifiers {
                        // Also drops the redrive record via the
                        // clear_pending hook.
                        if let Err(e) = guard.clear_pending(id, n) {
                            warn!(error = %e, "redrive: clear_pending failed");
                        }
                    }
                }
                record_activity_status_by_id(
                    store,
                    persister,
                    wallet_id,
                    id,
                    &redrive.activity_id,
                    ShieldedActivityStatus::Failed,
                )
                .await;
            }
            RedriveBroadcastOutcome::Inconclusive => {
                // `AlreadyExists` (still in a mempool after a lost-ACK
                // retry) or transport noise: inconclusive; counts toward
                // the cap.
                let attempts = bump_redrive_attempts_logged(store, id, &redrive.activity_id).await;
                debug!(
                    attempts,
                    max = MAX_REDRIVE_ATTEMPTS,
                    error = %err_display,
                    "redrive: re-broadcast inconclusive"
                );
            }
        }
    }
}

/// Whether an SDK error carries Platform's own consensus verdict on the
/// transition. Two shapes qualify:
///
/// - `Error::Protocol(ProtocolError::ConsensusError(_))` — DAPI attached the
///   serialized consensus error as gRPC metadata
///   (`dash-serialized-consensus-error-bin`), which the dapi-client decodes
///   on any failed request. This is how a CheckTx rejection of the
///   transition surfaces from `broadcast()` (rs-dapi's
///   `map_broadcast_error` decodes the consensus error from Tenderdash's
///   `info` field and `TenderdashStatus` re-attaches it as metadata);
/// - a `StateTransitionBroadcastError` whose `cause` deserialized from
///   non-empty consensus `data` — the wait-stream error envelope for a
///   transition Platform executed and rejected on its merits.
///
/// Recurses through a `NoAvailableAddressesToRetry` envelope, mirroring
/// [`crate::error::as_address_invalid_nonce`].
///
/// Only these prove the transition was evaluated and REJECTED. Everything
/// else — transport errors, timeouts, `AlreadyExists` (which proves the
/// opposite: the transition is already in the mempool or on chain),
/// DAPI-internal failures, cause-less broadcast envelopes (the shape DAPI
/// uses for its own wait-side timeouts) — leaves the outcome unknown.
fn carries_consensus_rejection(err: &dash_sdk::Error) -> bool {
    match err {
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(_)) => true,
        dash_sdk::Error::StateTransitionBroadcastError(e) => e.cause.is_some(),
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => carries_consensus_rejection(inner),
        _ => false,
    }
}

/// Broadcast a built shielded spend transition (unshield / transfer /
/// withdraw) and wait for proven execution, staging the two SDK calls
/// separately so the caller's reservation rollback only runs when the
/// spend DEFINITIVELY did not happen:
///
/// - a definitive `broadcast()` failure ([`broadcast_definitely_failed`]:
///   a consensus-verdict CheckTx rejection, or a transport failure that
///   proves nothing was delivered), or a wait-stage consensus rejection
///   (Platform ran the transition and rejected it on its merits — see
///   [`carries_consensus_rejection`]), means the spend never executed →
///   [`PlatformWalletError::ShieldedBroadcastFailed`], and the caller
///   releases the note reservations via [`cancel_pending`];
/// - a no-verdict `broadcast()` failure (`AlreadyExists` proves the tx IS
///   in the mempool after a lost-ACK retry; timeouts merely allow it)
///   falls through to the result wait, which either proves execution or
///   classifies the residual ambiguity;
/// - any other wait failure (transport error, timeout, result-proof
///   fetch/verify error, …) is AMBIGUOUS — the relay accepted (or may
///   have accepted) the tx and it may well have executed →
///   [`PlatformWalletError::ShieldedSpendUnconfirmed`], and the caller
///   must leave the reservations in place (each spend flow's outer
///   match has a dedicated arm for this).
///
/// Mirrors the staging in [`identity_create_from_shielded_pool`], minus
/// its fetch-by-derived-id fallback: a spend leaves no artifact as
/// cheaply queryable as an identity row, so ambiguity is surfaced
/// directly and reconciled by the next nullifier sync. The proven
/// result is discarded; only the confirmation matters.
async fn broadcast_shielded_spend(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition: &StateTransition,
    operation: &'static str,
) -> Result<(), PlatformWalletError> {
    match state_transition.broadcast(sdk, None).await {
        Ok(()) => {}
        Err(e) if broadcast_definitely_failed(&e) => {
            return Err(crate::error::promote_address_nonce_error(&e)
                .unwrap_or_else(|| PlatformWalletError::ShieldedBroadcastFailed(e.to_string())));
        }
        Err(e) => {
            warn!(
                operation,
                error = %e,
                "Shielded spend broadcast returned no verdict; the transition may have been \
                 admitted — falling through to the result wait"
            );
        }
    }

    state_transition
        .wait_for_response::<StateTransitionProofResult>(sdk, None)
        .await
        .map(|_| ())
        .map_err(|wait_err| classify_spend_wait_failure(operation, &wait_err))
}

/// Whether a failed `broadcast()` call DEFINITIVELY left the transition
/// out of every mempool, so any note reservations may be released and the
/// caller may rebuild and retry:
///
/// - a consensus verdict ([`carries_consensus_rejection`]): CheckTx
///   evaluated the transition and refused it;
/// - a gRPC response whose status code is a server-side rejection or a
///   connection-establishment failure. `Unavailable` is the common shape
///   of a connect-refused/offline attempt — classifying it as definitive
///   keeps the no-network failure's notes immediately re-spendable
///   instead of stranding them until the next restart — and rejection
///   codes (`InvalidArgument`, `ResourceExhausted` = mempool full, …) are
///   verdicts that the tx was refused admission;
/// - no usable DAPI addresses at all (nothing was ever sent).
///
/// `Unavailable` is NOT an absolute never-delivered guarantee: HTTP/2
/// stream resets after the request bytes left can surface the same code,
/// and the dapi-client's cross-address retry only retains the LAST
/// transport error, so an earlier-attempt delivery can hide behind a
/// later attempt's `Unavailable`. Releasing the notes in that residual
/// window is still fund-safe — the authoritative no-reuse guarantee is
/// the on-chain nullifier set, so a re-selected note at worst wastes a
/// ~30 s proof on a nullifier-already-used rejection (see the
/// `finalize_pending` downgrade rationale in `unshield`); never fund
/// loss. The trade is deliberate: UX for the dominant offline case over
/// strict conservatism in a rare race.
///
/// Everything else leaves the outcome unknown and the caller must fall
/// through to the result wait instead of failing: `AlreadyExists` proves
/// the tx IS in the mempool or on chain (a lost-ACK attempt was re-sent
/// by the dapi-client retry and hit tenderdash's dedupe), and
/// timeout/cancellation/no-response shapes (`TimeoutReached`,
/// `Cancelled`, gRPC `DeadlineExceeded`/`Cancelled`, plus
/// `Internal`/`Unknown`/`Aborted`/`DataLoss`, which DAPI also uses for
/// its own tenderdash-side failures that can postdate delivery) allow
/// the request to have outlived its lost ACK.
fn broadcast_definitely_failed(e: &dash_sdk::Error) -> bool {
    use dash_sdk::dapi_client::transport::TransportError;
    use dash_sdk::dapi_client::DapiClientError;
    use dash_sdk::dapi_grpc::tonic::Code;

    fn status_is_verdict(t: &TransportError) -> bool {
        let TransportError::Grpc(status) = t;
        !matches!(
            status.code(),
            Code::DeadlineExceeded
                | Code::Cancelled
                | Code::Unknown
                | Code::Internal
                | Code::Aborted
                | Code::DataLoss
        )
    }

    if carries_consensus_rejection(e) {
        return true;
    }
    match e {
        dash_sdk::Error::AlreadyExists(_) => false,
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(t)) => status_is_verdict(t),
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddresses) => true,
        dash_sdk::Error::DapiClientError(DapiClientError::NoAvailableAddressesToRetry(t)) => {
            status_is_verdict(t)
        }
        dash_sdk::Error::NoAvailableAddressesToRetry(inner) => broadcast_definitely_failed(inner),
        _ => false,
    }
}

/// Classify a `wait_for_response` failure for an already-broadcast
/// shielded spend (see [`broadcast_shielded_spend`]).
///
/// Only a consensus verdict ([`carries_consensus_rejection`]) proves
/// Platform executed the transition and rejected it on its merits — the
/// serialized consensus error is the verdict. DAPI encodes its own
/// wait-side failures (timeouts, internal errors; see
/// `build_wait_for_state_transition_error_response` in rs-dapi) as
/// `StateTransitionBroadcastError`s with EMPTY consensus data, which
/// the SDK surfaces as `cause: None` — those are ambiguous, not
/// rejections, and must keep the note reservations. Erring this way is
/// the safe direction: misreading a rejection as ambiguous only delays
/// note release until the next sync or restart, while misreading a
/// timeout as a rejection re-frees notes whose nullifiers may already
/// be consumed on chain.
fn classify_spend_wait_failure(
    operation: &'static str,
    wait_err: &dash_sdk::Error,
) -> PlatformWalletError {
    if carries_consensus_rejection(wait_err) {
        crate::error::promote_address_nonce_error(wait_err)
            .unwrap_or_else(|| PlatformWalletError::ShieldedBroadcastFailed(wait_err.to_string()))
    } else {
        warn!(
            operation,
            error = %wait_err,
            "Shielded spend broadcast accepted but result confirmation failed; \
             leaving the note reservations in place"
        );
        PlatformWalletError::ShieldedSpendUnconfirmed {
            operation,
            reason: wait_err.to_string(),
        }
    }
}

/// Helper to clone selection results out from under the store lock.
trait SelectionResultOwned {
    fn into_owned(self) -> (Vec<ShieldedNote>, u64, u64);
}

impl SelectionResultOwned for (Vec<&ShieldedNote>, u64, u64) {
    fn into_owned(self) -> (Vec<ShieldedNote>, u64, u64) {
        let (refs, total, fee) = self;
        let owned: Vec<ShieldedNote> = refs.into_iter().cloned().collect();
        (owned, total, fee)
    }
}

/// Convert a `PaymentAddress` to an `OrchardAddress` for the DPP builder.
fn payment_address_to_orchard(
    addr: &PaymentAddress,
) -> Result<OrchardAddress, PlatformWalletError> {
    let raw = addr.to_raw_address_bytes();
    OrchardAddress::from_raw_bytes(&raw).map_err(|_| {
        PlatformWalletError::ShieldedBuildError(
            "Failed to convert PaymentAddress to OrchardAddress".to_string(),
        )
    })
}

/// Deserialize an Orchard Note from 115 bytes.
///
/// Format: `recipient(43) || value(8 LE) || rho(32) || rseed(32)`.
/// Must be kept in sync with `serialize_note()` in sync.rs.
fn deserialize_note(data: &[u8]) -> Option<grovedb_commitment_tree::Note> {
    use grovedb_commitment_tree::{Note, NoteValue, RandomSeed, Rho};

    const SERIALIZED_NOTE_LEN: usize = 43 + 8 + 32 + 32;

    if data.len() != SERIALIZED_NOTE_LEN {
        return None;
    }

    let recipient_bytes: [u8; 43] = data[0..43].try_into().ok()?;
    let recipient = PaymentAddress::from_raw_address_bytes(&recipient_bytes).into_option()?;

    let value_bytes: [u8; 8] = data[43..51].try_into().ok()?;
    let value = NoteValue::from_raw(u64::from_le_bytes(value_bytes));

    let rho_bytes: [u8; 32] = data[51..83].try_into().ok()?;
    let rho = Rho::from_bytes(&rho_bytes).into_option()?;

    let rseed_bytes: [u8; 32] = data[83..115].try_into().ok()?;
    let rseed = RandomSeed::from_bytes(rseed_bytes, &rho).into_option()?;

    Note::from_parts(recipient, value, rho, rseed).into_option()
}

#[cfg(test)]
mod redrive_tests {
    use super::*;
    use crate::wallet::shielded::store::InMemoryShieldedStore;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;

    /// On a re-broadcast of our own byte-identical transition,
    /// `NullifierAlreadySpent` means the ORIGINAL executed — the
    /// classification must treat it as a success signal, distinct from
    /// every other consensus verdict.
    #[test]
    fn nullifier_already_spent_is_a_success_signal() {
        let cause = ConsensusError::StateError(StateError::NullifierAlreadySpentError(
            NullifierAlreadySpentError::new([1u8; 32]),
        ));
        let err = dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
            code: 1,
            message: "state error".to_string(),
            cause: Some(cause),
        });
        assert!(is_nullifier_already_spent(&err));
        // ...and it still counts as a consensus-rejection shape, so arm
        // ordering matters: the already-spent check must run first.
        assert!(carries_consensus_rejection(&err));

        let other = dash_sdk::Error::TimeoutReached(
            std::time::Duration::from_secs(1),
            "waiting".to_string(),
        );
        assert!(!is_nullifier_already_spent(&other));
    }

    /// The re-broadcast outcome classifier, arm order included:
    /// `NullifierAlreadySpent` is itself a consensus error, so the
    /// `AlreadyExecuted` arm must win over `DefinitiveRejection`.
    #[test]
    fn redrive_broadcast_classification_matrix() {
        assert_eq!(
            classify_redrive_broadcast(&Ok(())),
            RedriveBroadcastOutcome::Accepted
        );

        let already_spent = ConsensusError::StateError(StateError::NullifierAlreadySpentError(
            NullifierAlreadySpentError::new([1u8; 32]),
        ));
        let already_spent_err =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 1,
                message: "state error".to_string(),
                cause: Some(already_spent),
            });
        assert_eq!(
            classify_redrive_broadcast(&Err(already_spent_err)),
            RedriveBroadcastOutcome::AlreadyExecuted,
            "already-spent must classify as success BEFORE the generic rejection arm"
        );

        let other_rejection = ConsensusError::BasicError(
            dpp::consensus::basic::BasicError::ProtocolVersionParsingError(
                dpp::consensus::basic::decode::ProtocolVersionParsingError::new(
                    "bad version".to_string(),
                ),
            ),
        );
        let rejection_err =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 1,
                message: "state error".to_string(),
                cause: Some(other_rejection),
            });
        assert_eq!(
            classify_redrive_broadcast(&Err(rejection_err)),
            RedriveBroadcastOutcome::DefinitiveRejection
        );

        let timeout = dash_sdk::Error::TimeoutReached(
            std::time::Duration::from_secs(1),
            "waiting".to_string(),
        );
        assert_eq!(
            classify_redrive_broadcast(&Err(timeout)),
            RedriveBroadcastOutcome::Inconclusive
        );
    }

    /// Decision paths that must NOT touch the network (the mock SDK has
    /// no broadcast expectation, so any attempt would error into the
    /// inconclusive arm and bump the counter): a pruned anchor belongs
    /// to the release backstop, an exhausted budget stays parked, and a
    /// corrupt stored transition is dropped so it can't wedge the pass.
    #[tokio::test]
    async fn redrive_skips_pruned_and_exhausted_and_drops_garbage() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id = [5u8; 32];
        let id = SubwalletId::new(wallet_id, 0);

        let rec = |activity: u8, anchor: u8, attempts: u32| PendingRedrive {
            activity_id: [activity; 32],
            anchor: [anchor; 32],
            nullifiers: vec![[activity ^ 0xFF; 32]],
            st_bytes: vec![0xDE, 0xAD], // never deserializes
            attempts,
        };
        {
            let mut guard = store.write().await;
            // Pruned anchor (10 not in recorded set) → untouched.
            guard.arm_redrive(id, rec(1, 10, 0)).unwrap();
            // Recorded anchor but attempts exhausted → untouched.
            guard
                .arm_redrive(id, rec(2, 11, MAX_REDRIVE_ATTEMPTS))
                .unwrap();
            // Recorded anchor, budget left, garbage bytes → dropped
            // before any broadcast.
            guard.arm_redrive(id, rec(3, 12, 0)).unwrap();
        }
        let recorded: std::collections::HashSet<[u8; 32]> =
            [[11u8; 32], [12u8; 32]].into_iter().collect();

        redrive_pending_spends(&sdk, &store, None, wallet_id, id, &recorded).await;

        let left = store.read().await.pending_redrives(id).unwrap();
        let ids: Vec<[u8; 32]> = left.iter().map(|r| r.activity_id).collect();
        assert!(ids.contains(&[1u8; 32]), "pruned-anchor record left alone");
        assert!(ids.contains(&[2u8; 32]), "exhausted record left alone");
        assert!(
            !ids.contains(&[3u8; 32]),
            "corrupt record dropped without a broadcast attempt"
        );
        assert_eq!(
            left.iter().map(|r| r.attempts).max(),
            Some(MAX_REDRIVE_ATTEMPTS),
            "no attempt counters were bumped — nothing touched the network"
        );
    }
}

#[cfg(test)]
mod classify_spend_wait_failure_tests {
    use super::*;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dpp::consensus::basic::decode::ProtocolVersionParsingError;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;

    fn broadcast_err(cause: Option<ConsensusError>) -> dash_sdk::Error {
        dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
            code: 13,
            message: "context deadline exceeded".to_string(),
            cause,
        })
    }

    #[test]
    fn consensus_cause_is_a_definitive_rejection() {
        let cause = ConsensusError::BasicError(BasicError::ProtocolVersionParsingError(
            ProtocolVersionParsingError::new("bad version".to_string()),
        ));
        let err = classify_spend_wait_failure("unshield", &broadcast_err(Some(cause)));
        assert!(
            matches!(err, PlatformWalletError::ShieldedBroadcastFailed(_)),
            "a broadcast error carrying a consensus cause means Platform ran and \
             rejected the transition; the caller may release the reservation"
        );
    }

    #[test]
    fn causeless_broadcast_error_is_ambiguous() {
        // DAPI maps its own wait failures (e.g. `DapiError::Timeout`) to a
        // `StateTransitionBroadcastError` with EMPTY consensus data, which the
        // SDK decodes as `cause: None`. The spend may still execute after the
        // wait gave up, so this must NOT release the note reservations.
        let err = classify_spend_wait_failure("unshield", &broadcast_err(None));
        assert!(
            matches!(
                err,
                PlatformWalletError::ShieldedSpendUnconfirmed {
                    operation: "unshield",
                    ..
                }
            ),
            "got {err:?}"
        );
    }

    #[test]
    fn transport_errors_are_ambiguous() {
        let err = classify_spend_wait_failure(
            "withdraw",
            &dash_sdk::Error::TimeoutReached(
                std::time::Duration::from_secs(80),
                "waiting for response".to_string(),
            ),
        );
        assert!(matches!(
            err,
            PlatformWalletError::ShieldedSpendUnconfirmed {
                operation: "withdraw",
                ..
            }
        ));
    }

    /// The shape a CheckTx rejection takes when it reaches the SDK from
    /// `broadcast()`: DAPI re-attaches the serialized consensus error as
    /// gRPC metadata and the dapi-client decodes it into
    /// `Error::Protocol(ConsensusError)`.
    fn consensus_metadata_rejection() -> dash_sdk::Error {
        let cause = ConsensusError::BasicError(BasicError::ProtocolVersionParsingError(
            ProtocolVersionParsingError::new("bad version".to_string()),
        ));
        dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(cause)))
    }

    /// A CheckTx consensus rejection surfacing from `broadcast()` IS
    /// definitive: the transition was evaluated and refused, it sits in no
    /// mempool, and the caller may release the reservations and retry.
    #[test]
    fn broadcast_consensus_rejection_is_definitive() {
        assert!(broadcast_definitely_failed(&consensus_metadata_rejection()));
        // The same verdict shape is definitive on the wait stage too.
        let err = classify_spend_wait_failure("transfer", &consensus_metadata_rejection());
        assert!(matches!(
            err,
            PlatformWalletError::ShieldedBroadcastFailed(_)
        ));
    }

    fn grpc_err(code: dash_sdk::dapi_grpc::tonic::Code) -> dash_sdk::Error {
        use dash_sdk::dapi_client::transport::TransportError;
        use dash_sdk::dapi_client::DapiClientError;
        dash_sdk::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
            dash_sdk::dapi_grpc::tonic::Status::new(code, "boom"),
        )))
    }

    /// Connection-establishment failures and non-consensus server
    /// rejections of `broadcast()` ARE definitive: nothing was admitted to
    /// a mempool. The offline case (connect refused → `Unavailable`) in
    /// particular must release the notes immediately rather than strand
    /// them until restart.
    #[test]
    fn broadcast_connection_failures_and_rejections_are_definitive() {
        use dash_sdk::dapi_grpc::tonic::Code;
        assert!(broadcast_definitely_failed(&grpc_err(Code::Unavailable)));
        // Mempool full / malformed request: server verdicts refusing admission.
        assert!(broadcast_definitely_failed(&grpc_err(
            Code::ResourceExhausted
        )));
        assert!(broadcast_definitely_failed(&grpc_err(
            Code::InvalidArgument
        )));
    }

    /// No-response shapes of the `broadcast()` call itself are NOT
    /// definitive: the dapi-client retries across addresses, so the request
    /// may have been delivered to a node whose ACK was lost. The caller
    /// falls through to the result wait instead of releasing the
    /// reservations (which would let a retry select other unreserved notes
    /// and double-send if the original broadcast landed).
    #[test]
    fn broadcast_no_response_shapes_are_inconclusive() {
        use dash_sdk::dapi_grpc::tonic::Code;
        assert!(!broadcast_definitely_failed(&grpc_err(
            Code::DeadlineExceeded
        )));
        // DAPI maps its own tenderdash-side failures (which can postdate
        // delivery) to Internal.
        assert!(!broadcast_definitely_failed(&grpc_err(Code::Internal)));
        assert!(!broadcast_definitely_failed(
            &dash_sdk::Error::TimeoutReached(
                std::time::Duration::from_secs(30),
                "broadcast".to_string(),
            )
        ));
        assert!(!broadcast_definitely_failed(&dash_sdk::Error::Generic(
            "transport error: connection reset".to_string()
        )));
        // …including as the terminal error of an exhausted retry loop.
        assert!(!broadcast_definitely_failed(
            &dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(grpc_err(
                Code::DeadlineExceeded
            )))
        ));
    }

    /// `AlreadyExists` from `broadcast()` proves the transition IS already
    /// in the mempool or on chain (e.g. an internal dapi-client retry after
    /// an ambiguous first delivery), so it must NOT be treated as a failure
    /// — the spend is in flight and the result wait will confirm it.
    #[test]
    fn broadcast_already_exists_is_in_flight() {
        assert!(!broadcast_definitely_failed(
            &dash_sdk::Error::AlreadyExists("state transition already in mempool".to_string())
        ));
    }

    /// A consensus verdict wrapped in the dapi-client's `NoAvailableAddressesToRetry`
    /// retry envelope must still count as a rejection — `carries_consensus_rejection`
    /// recurses through the envelope, in lockstep with `as_address_invalid_nonce`.
    #[test]
    fn wrapped_consensus_rejection_is_a_rejection() {
        let wrapped =
            dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(consensus_metadata_rejection()));
        assert!(carries_consensus_rejection(&wrapped));
    }

    /// A transport error wrapped in the retry envelope carries no consensus
    /// verdict, so it remains ambiguous — the recursion must not misread it.
    #[test]
    fn wrapped_transport_error_is_not_a_rejection() {
        use dash_sdk::dapi_grpc::tonic::Code;
        let wrapped = dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(grpc_err(
            Code::DeadlineExceeded,
        )));
        assert!(!carries_consensus_rejection(&wrapped));
    }

    /// A nonce rejection wrapped in the retry envelope must reach
    /// `promote_address_nonce_error` and surface as the typed
    /// `AddressNonceMismatch`, not fall through to `ShieldedSpendUnconfirmed`.
    #[test]
    fn wrapped_nonce_rejection_promotes_to_typed_mismatch() {
        use dpp::address_funds::PlatformAddress;
        use dpp::consensus::state::address_funds::AddressInvalidNonceError;
        use dpp::consensus::state::state_error::StateError;

        let address = PlatformAddress::P2pkh([9u8; 20]);
        let cause = ConsensusError::StateError(StateError::AddressInvalidNonceError(
            AddressInvalidNonceError::new(address, 7, 8),
        ));
        let inner = dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(cause)));
        let wrapped = dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(inner));

        match classify_spend_wait_failure("withdraw", &wrapped) {
            PlatformWalletError::AddressNonceMismatch {
                address: got,
                provided_nonce,
                expected_nonce,
            } => {
                assert_eq!(got, address);
                assert_eq!(provided_nonce, 7);
                assert_eq!(expected_nonce, 8);
            }
            other => panic!("expected AddressNonceMismatch, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod shield_input_fetch_error_tests {
    use super::*;
    use dpp::consensus::state::address_funds::AddressNotEnoughFundsError;

    #[test]
    fn live_address_shortfall_maps_to_typed_shield_capacity_error() {
        let sdk_error = dash_sdk::Error::from(AddressNotEnoughFundsError::new(
            PlatformAddress::P2pkh([7; 20]),
            3_623_849_220,
            3_623_849_221,
        ));

        let mapped = map_shield_input_fetch_error(&sdk_error);
        assert!(matches!(
            &mapped,
            PlatformWalletError::ShieldedInsufficientBalance { available, required }
                if *available == 3_623_849_220 && *required == 3_623_849_221
        ));
        assert_eq!(
            mapped.to_string(),
            "Insufficient shielded balance: available 3623849220, required 3623849221"
        );
    }
}

#[cfg(test)]
mod reserve_shield_fee_tests {
    use super::*;

    fn addr(b: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([b; 20])
    }

    #[test]
    fn loads_fee_onto_smallest_key_input() {
        // Input 0 is the BTreeMap-smallest address (addr(1)); the fee must
        // land there, matching the `DeductFromInput(0)` fee strategy.
        let mut inputs = BTreeMap::new();
        inputs.insert(addr(2), 5_000_000u64);
        inputs.insert(addr(1), 1_000_000u64);

        let fee = 123_097_600u64;
        let out = reserve_shield_fee_on_input_0(inputs, fee).expect("non-empty inputs");

        assert_eq!(out.get(&addr(1)), Some(&(1_000_000 + fee)));
        assert_eq!(
            out.get(&addr(2)),
            Some(&5_000_000),
            "other inputs untouched"
        );
        // Σ claims grew by exactly `fee`, satisfying `Σ inputs >= amount + F`.
        assert_eq!(out.values().sum::<u64>(), 6_000_000 + fee);
    }

    #[test]
    fn errors_on_empty_inputs() {
        let inputs: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
        let err = reserve_shield_fee_on_input_0(inputs, 1).expect_err("empty must reject");
        assert!(matches!(err, PlatformWalletError::ShieldedBuildError(_)));
    }

    #[test]
    fn errors_on_claim_plus_fee_overflow() {
        let mut inputs = BTreeMap::new();
        inputs.insert(addr(1), u64::MAX);
        let err = reserve_shield_fee_on_input_0(inputs, 1).expect_err("overflow must reject");
        assert!(matches!(err, PlatformWalletError::ShieldedBuildError(_)));
    }
}

#[cfg(test)]
mod note_reservation_release_tests {
    use super::*;

    /// `ShieldedBroadcastUnconfirmed` is the one failure that must NOT release the reservation: the
    /// broadcast was accepted and the transition may have executed, so freeing the notes invites a
    /// double-spend against notes that may already be consumed on chain. The next nullifier sync
    /// reconciles them.
    #[test]
    fn unconfirmed_broadcast_retains_reservation() {
        let e = PlatformWalletError::ShieldedBroadcastUnconfirmed {
            identity_id: Identifier::from([7u8; 32]),
            reason: "result proof unavailable".to_string(),
        };
        assert!(
            !error_releases_note_reservation(&e),
            "ShieldedBroadcastUnconfirmed must retain the note reservation"
        );
    }

    /// Every other failure is a definitive pre-execution / build / rejection failure — the spend
    /// never happened, so the reservation must be released.
    #[test]
    fn definitive_failures_release_reservation() {
        let releasing: Vec<PlatformWalletError> = vec![
            PlatformWalletError::ShieldedBroadcastFailed("rejected on merits".to_string()),
            PlatformWalletError::ShieldedBuildError("note selection failed".to_string()),
            PlatformWalletError::ShieldedStoreError("store write failed".to_string()),
        ];
        for e in &releasing {
            assert!(
                error_releases_note_reservation(e),
                "{e:?} must release the note reservation"
            );
        }
    }
}

#[cfg(test)]
mod record_activity_status_tests {
    use super::*;
    use crate::wallet::shielded::activity::{
        ShieldedActivityEntry, ShieldedActivityKind, ShieldedDirection,
    };
    use crate::wallet::shielded::store::InMemoryShieldedStore;

    fn sub() -> SubwalletId {
        SubwalletId::new([0xCC; 32], 0)
    }

    /// The Pending entry a live recorder captures before broadcast.
    fn captured_pending() -> ShieldedActivityEntry {
        ShieldedActivityEntry {
            id: [0xAA; 32],
            kind: ShieldedActivityKind::Shield,
            direction: ShieldedDirection::In,
            amount: 1_000,
            fee: Some(10),
            counterparty: None,
            memo: None,
            block_height: None,
            status: ShieldedActivityStatus::Pending,
            created_at_ms: 1,
            min_note_position: None,
            note_cmxs: vec![[0x01; 32]],
            spent_nullifiers: vec![],
        }
    }

    /// A scan pass that confirmed the row at a real height between the
    /// broadcast and the result-wait must win over the post-wait flip:
    /// the stale captured entry must not overwrite the stored
    /// `Confirmed`-with-height row (neither downgrading it to `Failed`
    /// nor erasing the scan-learned height).
    #[tokio::test]
    async fn flip_does_not_clobber_scan_confirmed_row() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = sub();
        let pending = captured_pending();
        let scan_confirmed = with_status(&pending, ShieldedActivityStatus::Confirmed, Some(777));
        store
            .write()
            .await
            .save_activity(id, &scan_confirmed)
            .unwrap();

        record_activity_status(
            &store,
            None,
            id.wallet_id,
            id,
            &Some(pending),
            ShieldedActivityStatus::Failed,
            None,
        )
        .await;

        let stored = store
            .read()
            .await
            .get_activity_by_entry_id(id, &[0xAA; 32])
            .unwrap()
            .expect("row must still exist");
        assert_eq!(stored.status, ShieldedActivityStatus::Confirmed);
        assert_eq!(stored.block_height, Some(777));
    }

    /// No concurrent scan: the flip applies to the stored Pending row
    /// (and falls back to the captured entry when the store has none),
    /// writing the new status into the in-memory store.
    #[tokio::test]
    async fn flip_applies_when_row_is_still_pending() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = sub();
        let pending = captured_pending();
        store.write().await.save_activity(id, &pending).unwrap();

        record_activity_status(
            &store,
            None,
            id.wallet_id,
            id,
            &Some(pending),
            ShieldedActivityStatus::Confirmed,
            Some(900),
        )
        .await;

        let stored = store
            .read()
            .await
            .get_activity_by_entry_id(id, &[0xAA; 32])
            .unwrap()
            .expect("row must exist");
        assert_eq!(stored.status, ShieldedActivityStatus::Confirmed);
        assert_eq!(stored.block_height, Some(900));
    }

    /// The by-id flip the sync reconcile uses: it knows only the
    /// reservation's stored `activity_id`, so it looks the row up and flips
    /// it — a released stranded spend moves Pending → Failed.
    #[tokio::test]
    async fn status_flip_by_id_flips_pending_to_failed() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = sub();
        let pending = captured_pending();
        store.write().await.save_activity(id, &pending).unwrap();

        record_activity_status_by_id(
            &store,
            None,
            id.wallet_id,
            id,
            &pending.id,
            ShieldedActivityStatus::Failed,
        )
        .await;

        let stored = store
            .read()
            .await
            .get_activity_by_entry_id(id, &pending.id)
            .unwrap()
            .expect("row must exist");
        assert_eq!(stored.status, ShieldedActivityStatus::Failed);
    }

    /// A by-id flip for an entry that doesn't exist is a silent no-op
    /// (nothing to flip), never a panic.
    #[tokio::test]
    async fn status_flip_by_id_missing_entry_is_noop() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = sub();
        record_activity_status_by_id(
            &store,
            None,
            id.wallet_id,
            id,
            &[0xDE; 32],
            ShieldedActivityStatus::Failed,
        )
        .await;
        assert!(store
            .read()
            .await
            .get_activity_by_entry_id(id, &[0xDE; 32])
            .unwrap()
            .is_none());
    }
}

/// Unit tests for the pure anchor-selection probe ([`select_recorded_spends`])
/// against a real SQLite-backed commitment tree — no SDK, no network.
///
/// These pin the fix for the shielded-withdrawal "never lands" root cause:
/// the wallet must build a spend against a Platform-recorded anchor, not the
/// bleeding-edge depth-0 root a mid-block index-chunk sync leaves behind. They
/// reuse the block-boundary tree shape from the `file_store` reproduction test.
#[cfg(test)]
mod select_recorded_spends_tests {
    use super::*;
    use crate::wallet::shielded::file_store::FileBackedShieldedStore;
    use dashcore::Network;
    use grovedb_commitment_tree::{ExtractedNoteCommitment, Note, NoteValue, RandomSeed, Rho};

    /// Unique temp path for a test tree (no `tempfile` dev-dep).
    fn temp_tree_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("select_recorded_spends_{tag}_{nanos}.sqlite"))
    }

    /// A filler leaf commitment for non-owned positions. Any canonical 32-byte
    /// field element works — the probe only needs the tree to grow between
    /// blocks so successive checkpoint roots differ.
    fn filler_cmx(b: u8) -> [u8; 32] {
        let mut c = [0u8; 32];
        c[0] = b;
        c
    }

    /// Build one real, spendable Orchard note owned by a fixed test seed and
    /// return the wallet's `ShieldedNote` view of it.
    ///
    /// `note_data` is the real serialized note so `deserialize_note` accepts
    /// it, and `cmx` is the note's real extracted commitment so that appending
    /// `cmx` as the leaf at `position` makes `witness(position, d).root(cmx)`
    /// reproduce the tree's anchor at depth `d`.
    fn real_note(position: u64) -> ShieldedNote {
        let keys = OrchardKeySet::from_seed(&[0x42; 32], Network::Testnet, 0)
            .expect("ZIP-32 derivation from a fixed seed");
        let recipient = keys.default_address;

        // rho and rseed must be canonical Pallas base-field elements; not every
        // 32-byte pattern is, so scan deterministically for a valid pair drawn
        // from disjoint byte regions (mirroring the sync tests' note builders).
        let rho = (1u16..=u16::MAX)
            .find_map(|n| {
                let mut b = [0u8; 32];
                b[0..2].copy_from_slice(&n.to_le_bytes());
                Rho::from_bytes(&b).into_option()
            })
            .expect("a canonical rho exists");
        let rseed = (1u16..=u16::MAX)
            .find_map(|m| {
                let mut b = [0u8; 32];
                b[2..4].copy_from_slice(&m.to_le_bytes());
                RandomSeed::from_bytes(b, &rho).into_option()
            })
            .expect("a canonical rseed exists");

        let value = NoteValue::from_raw(100_000);
        let note = Note::from_parts(recipient, value, rho, rseed)
            .into_option()
            .expect("valid note parts");
        let cmx = ExtractedNoteCommitment::from(note.commitment()).to_bytes();

        // `recipient(43) || value(8 LE) || rho(32) || rseed(32)` — the exact
        // format `deserialize_note` expects.
        let mut note_data = Vec::with_capacity(115);
        note_data.extend_from_slice(&note.recipient().to_raw_address_bytes());
        note_data.extend_from_slice(&note.value().inner().to_le_bytes());
        note_data.extend_from_slice(&note.rho().to_bytes());
        note_data.extend_from_slice(note.rseed().as_bytes());

        ShieldedNote {
            position,
            cmx,
            nullifier: [0x07; 32],
            block_height: 1,
            is_spent: false,
            value: 100_000,
            note_data,
        }
    }

    /// Mid-block: the wallet's depth-0 root is not recorded, but a prior
    /// block-boundary checkpoint is — the probe must select that older recorded
    /// anchor (the shallowest one), never the mid-block depth-0 root Platform
    /// never recorded.
    #[test]
    fn mid_block_selects_prior_recorded_checkpoint_not_depth0() {
        let path = temp_tree_path("midblock");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // The owned note lives at position 0, present since block 1.
        let note = real_note(0);

        // Block 1 = positions 0,1,2 (leaf 0 is the owned note's cmx). drive
        // records ONE anchor per block, at block-processing-end.
        store.append_commitment(&note.cmx, true).unwrap();
        store.append_commitment(&filler_cmx(0xA1), true).unwrap();
        store.append_commitment(&filler_cmx(0xA2), true).unwrap();
        store.checkpoint_tree(3).unwrap();
        let root_block1 = store.tree_anchor().unwrap();

        // Block 2 = positions 3,4,5. Its block-end root is the second recorded
        // anchor.
        store.append_commitment(&filler_cmx(0xB1), true).unwrap();
        store.append_commitment(&filler_cmx(0xB2), true).unwrap();
        store.append_commitment(&filler_cmx(0xB3), true).unwrap();
        store.checkpoint_tree(6).unwrap();
        let root_block2 = store.tree_anchor().unwrap();

        // Mid-block: the index-chunk sync appends one more commitment (position
        // 6) and checkpoints there. The depth-0 root is now a state drive never
        // recorded.
        store.append_commitment(&filler_cmx(0xC1), true).unwrap();
        store.checkpoint_tree(7).unwrap();
        let root_depth0 = store.tree_anchor().unwrap();

        // drive's recorded set is exactly the two block-boundary roots.
        let recorded: HashSet<[u8; 32]> = [root_block1, root_block2].into_iter().collect();

        let (spends, anchor) =
            select_recorded_spends(&store, std::slice::from_ref(&note), &recorded)
                .expect("a prior recorded checkpoint covers the owned note");

        let _ = std::fs::remove_file(&path);

        assert_eq!(spends.len(), 1, "the single owned note is spendable");
        assert!(
            recorded.contains(&anchor.to_bytes()),
            "the selected anchor must be a Platform-recorded root"
        );
        assert_eq!(
            anchor.to_bytes(),
            root_block2,
            "must pick the shallowest recorded checkpoint (block 2 / depth 1), not a deeper one"
        );
        assert_ne!(
            anchor.to_bytes(),
            root_depth0,
            "must NOT use the mid-block depth-0 root drive never recorded"
        );
    }

    /// Fully synced: the wallet's depth-0 root IS recorded, so the probe takes
    /// the fast path and returns the depth-0 anchor without walking deeper.
    #[test]
    fn fully_synced_returns_depth0_anchor() {
        let path = temp_tree_path("fastpath");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let note = real_note(0);

        // One block, checkpointed exactly on its boundary: depth 0 == a recorded
        // anchor.
        store.append_commitment(&note.cmx, true).unwrap();
        store.append_commitment(&filler_cmx(0xA1), true).unwrap();
        store.append_commitment(&filler_cmx(0xA2), true).unwrap();
        store.checkpoint_tree(3).unwrap();
        let root_depth0 = store.tree_anchor().unwrap();

        let recorded: HashSet<[u8; 32]> = [root_depth0].into_iter().collect();

        let (spends, anchor) =
            select_recorded_spends(&store, std::slice::from_ref(&note), &recorded)
                .expect("depth-0 root is recorded");

        let _ = std::fs::remove_file(&path);

        assert_eq!(spends.len(), 1);
        assert_eq!(
            anchor.to_bytes(),
            root_depth0,
            "the fully-synced fast path returns the depth-0 anchor"
        );
    }

    /// No checkpoint root is recorded: the probe exhausts every depth and
    /// returns the retryable `ShieldedNoRecordedAnchor` — nothing is broadcast.
    #[test]
    fn no_recorded_checkpoint_returns_retryable_error() {
        let path = temp_tree_path("none");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let note = real_note(0);

        store.append_commitment(&note.cmx, true).unwrap();
        store.append_commitment(&filler_cmx(0xA1), true).unwrap();
        store.append_commitment(&filler_cmx(0xA2), true).unwrap();
        store.checkpoint_tree(3).unwrap();

        // Platform recorded none of this wallet's checkpoint roots.
        let recorded: HashSet<[u8; 32]> = HashSet::new();

        // `SpendableNote` isn't `Debug`, so match rather than `expect_err`.
        let result = select_recorded_spends(&store, std::slice::from_ref(&note), &recorded);

        let _ = std::fs::remove_file(&path);

        match result {
            Err(PlatformWalletError::ShieldedNoRecordedAnchor(_)) => {}
            Err(other) => {
                panic!("expected ShieldedNoRecordedAnchor, got error: {other:?}")
            }
            Ok(_) => panic!("expected ShieldedNoRecordedAnchor, got Ok"),
        }
    }

    /// A selected note that post-dates the only recorded checkpoint. Depth 0
    /// (which contains the note) isn't recorded; at depth 1 the note is not yet
    /// in the tree, so the probe's early-termination fires (a deeper checkpoint
    /// is older still and can't contain it) and the retryable error is returned
    /// — even though a recorded anchor exists, it doesn't cover this note. This
    /// pins the walk's break arm and the value-selection trade-off (a mid-block
    /// wallet whose selected note is newer than every recorded checkpoint waits
    /// for the next sync rather than spending).
    #[test]
    fn note_newer_than_recorded_checkpoint_breaks_and_returns_retryable_error() {
        let path = temp_tree_path("toonew");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Block 1 = positions 0,1,2 (fillers), checkpointed on its boundary —
        // the only root Platform recorded.
        store.append_commitment(&filler_cmx(0xA0), true).unwrap();
        store.append_commitment(&filler_cmx(0xA1), true).unwrap();
        store.append_commitment(&filler_cmx(0xA2), true).unwrap();
        store.checkpoint_tree(3).unwrap();
        let root_block1 = store.tree_anchor().unwrap();

        // The owned note is appended AFTER block 1 (position 3) and checkpointed:
        // present at depth 0, absent from the block-1 checkpoint (depth 1).
        let note = real_note(3);
        store.append_commitment(&note.cmx, true).unwrap();
        store.checkpoint_tree(4).unwrap();

        let recorded: HashSet<[u8; 32]> = [root_block1].into_iter().collect();

        let result = select_recorded_spends(&store, std::slice::from_ref(&note), &recorded);

        let _ = std::fs::remove_file(&path);

        match result {
            Err(PlatformWalletError::ShieldedNoRecordedAnchor(_)) => {}
            Err(other) => panic!("expected ShieldedNoRecordedAnchor, got error: {other:?}"),
            Ok(_) => panic!("expected ShieldedNoRecordedAnchor, got Ok"),
        }
    }
}
