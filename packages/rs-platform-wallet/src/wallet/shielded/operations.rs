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
use crate::broadcast_outcome::{broadcast_definitely_failed, carries_consensus_rejection};
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
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::identity_id_from_nullifiers;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use grovedb_commitment_tree::{Anchor, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn};

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

/// Multiplier applied to the versioned minimum shield fee when sizing the
/// planner's input-0 reserve.
///
/// Execution deducts the ACTUAL fee — the GroveDB-metered storage/processing
/// of the note/nullifier writes plus `compute_shielded_verification_fee` —
/// from input 0's post-reallocation residue, and rejects the shield when the
/// residue can't cover it. `compute_minimum_shielded_fee` estimates that
/// actual fee with a flat per-action storage term the client cannot meter
/// itself, so the reserve keeps one extra fee of headroom for metering
/// variance. The reserve is NOT what satisfies the structure gate
/// (`Σ claims ≥ amount + fee`) — `reserve_shield_fee_on_input_0` loads the
/// claimed fee for that — so it needs no allowance beyond metering variance.
const SHIELD_FEE_RESERVE_MULTIPLIER: u64 = 2;

/// Versioned balance the shield planner keeps unclaimed on the
/// lexicographically first (fee-paying) input.
///
/// The preflight and the execution path both derive capacity from this one
/// value, so it directly sets three host-visible numbers: the viability
/// threshold an address must exceed to serve as input 0, the account's
/// `max_shieldable_credits`, and the residue a Max shield leaves transparent
/// (`reserve − actual fee`). Deriving it from the versioned fee formula keeps
/// all three tracking fee-constant bumps instead of freezing a magic number
/// that overstates the fee and understates capacity.
pub fn shield_fee_reserve_credits(
    platform_version: &PlatformVersion,
) -> Result<Credits, PlatformWalletError> {
    let fee = compute_minimum_shielded_fee(SHIELD_NUM_ACTIONS, platform_version)
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
    fee.checked_mul(SHIELD_FEE_RESERVE_MULTIPLIER)
        .ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError("shield fee reserve overflows u64".to_string())
        })
}

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
        Some(short) => PlatformWalletError::PlatformShieldCapacityExceeded {
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

/// The resolved Orchard output and live-activity classification for a
/// shield — see [`resolve_shield_recipient`].
#[derive(Debug)]
struct ShieldRecipient {
    /// The address the note is built for.
    address: OrchardAddress,
    /// Raw 43-byte recipient for the activity row (`Some` only for a
    /// third-party recipient).
    counterparty: Option<Vec<u8>>,
    kind: ShieldedActivityKind,
    direction: ShieldedDirection,
}

/// Resolve a shield's Orchard output address and live-activity
/// classification from the optional third-party `recipient`.
///
/// `None` is the internal shield-to-self: the note goes to the
/// account's default address and the live entry is `Shield`/`In` with
/// no counterparty. `Some` pays a THIRD-PARTY address: `Sent`/`Out`
/// with the raw 43-byte address as counterparty — the exact
/// classification the scan deriver produces for an OVK-recovered send
/// to a non-own address, so a restore derives the same row.
///
/// A `Some` address the account's own IVK recognizes (default or any
/// diversified index — the same `diversifier_index` test the scan's
/// `is_own_orchard_recipient` uses) is rejected instead of classified:
/// its note WOULD be spendable here, so it is not a send, and
/// recording it live as `Sent`/`Out` while a restore scan-derives a
/// self-pay row would fork the two histories. Self-shields take the
/// `None` path.
fn resolve_shield_recipient(
    keys: &AccountViewingKeys,
    recipient: Option<&PaymentAddress>,
) -> Result<ShieldRecipient, PlatformWalletError> {
    match recipient {
        Some(payment_address) => {
            if keys
                .incoming_viewing_key
                .diversifier_index(payment_address)
                .is_some()
            {
                return Err(PlatformWalletError::ShieldedBuildError(
                    "recipient belongs to this shielded account; use the self-shield \
                     entry point (no recipient) instead"
                        .to_string(),
                ));
            }
            Ok(ShieldRecipient {
                address: payment_address_to_orchard(payment_address)?,
                counterparty: Some(payment_address.to_raw_address_bytes().to_vec()),
                kind: ShieldedActivityKind::Sent,
                direction: ShieldedDirection::Out,
            })
        }
        None => Ok(ShieldRecipient {
            address: default_orchard_address(keys)?,
            counterparty: None,
            kind: ShieldedActivityKind::Shield,
            direction: ShieldedDirection::In,
        }),
    }
}

/// Shield credits from transparent platform addresses into the
/// shielded pool, with the resulting note assigned to `account`'s
/// default Orchard payment address derived from `keys`.
///
/// Self-shield front for [`shield_to`], preserving the pre-recipient
/// signature for existing callers.
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
    shield_to(
        sdk, store, persister, wallet_id, keys, account, None, inputs, amount,
        [0u8; 36], // empty memo
        signer, prover,
    )
    .await
}

/// Shield credits from transparent platform addresses into the
/// shielded pool. `recipient` selects the note's Orchard payment
/// address: `None` assigns it to `account`'s default address derived
/// from `keys` (the internal shield-to-self); `Some` pays a
/// third-party address — the note funds THAT wallet's pool and never
/// becomes spendable here (a `Some` address this account's own IVK
/// recognizes is rejected — see [`resolve_shield_recipient`]). Either
/// way the output is encrypted under our own OVK, so the scan recovers
/// the send from chain data and the live and scan-derived activity ids
/// line up.
#[allow(clippy::too_many_arguments)]
pub async fn shield_to<S: ShieldedStore, Sig: Signer<PlatformAddress>, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &AccountViewingKeys,
    account: u32,
    recipient: Option<&PaymentAddress>,
    inputs: BTreeMap<PlatformAddress, Credits>,
    amount: u64,
    memo: [u8; 36],
    signer: &Sig,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let ShieldRecipient {
        address: recipient_addr,
        counterparty: external_counterparty,
        kind,
        direction,
    } = resolve_shield_recipient(keys, recipient)?;
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
    // (`shielded_shield_from_account`) reserves `shield_fee_reserve_credits`
    // (a small multiple of this same versioned fee) of unclaimed headroom on
    // input 0 specifically for this, so `F` always fits within the reserve.
    // Inflating the claim BEFORE the fetch lets the
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

    info!(
        account,
        credits = amount,
        external = external_counterparty.is_some(),
        "Shield: building proof"
    );

    let claimed_inputs = inputs_with_nonce.clone();

    let state_transition = build_shield_transition(
        &recipient_addr,
        amount,
        inputs_with_nonce,
        fee_strategy,
        signer,
        0, // user_fee_increase
        prover,
        memo,
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

    // Live activity. Kind / direction / counterparty were resolved
    // alongside the recipient above (see `resolve_shield_recipient` for
    // why the rows match what a restore's scan derives). Fee = the flat
    // shielded fee reserved above. The visible output cmx is the
    // recipient note (OVK-keyed either way), so the live and scan ids
    // line up.
    let pending_entry = record_pending_activity(
        store,
        persister,
        wallet_id,
        id,
        keys,
        LiveEntryParams {
            kind,
            direction,
            amount,
            fee: Some(fee),
            counterparty: external_counterparty,
            memo: non_zero_memo(&memo),
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

// -------------------------------------------------------------------------
// IdentityCreateFromShieldedPool from a ONE-TIME Orchard key (Type 20, L2
// invitations — the claim side)
// -------------------------------------------------------------------------

/// Create a brand-new Platform identity funded from a ONE-TIME Orchard spending
/// key (the L2-invitation *claim* side).
///
/// Unlike [`identity_create_from_shielded_pool`], the spend authority is NOT the
/// wallet's own [`OrchardKeySet`]; it is a foreign `one_time_sk` — the single-use
/// Orchard spending key an invitation was funded to. The op:
/// 1. derives the full-viewing / incoming-viewing / spend-authorizing keys from
///    `one_time_sk`,
/// 2. transiently scans the network for the note(s) that key owns (they are not
///    tracked in any subwallet store — see [`super::sync::scan_notes_for_foreign_key`]),
/// 3. selects notes covering exactly `denomination` (the exact-equality model —
///    the fee is metered FROM the denomination) and gates on
///    `denomination > predicted_fee`,
/// 4. witnesses the selected notes against a Platform-recorded anchor from the
///    shared (fully-marked) commitment tree — the SAME anchor probe the
///    pool-funded op uses (so a wallet that hasn't synced past the funding
///    position gets the retryable [`PlatformWalletError::ShieldedMerkleWitnessUnavailable`]),
/// 5. feeds the key-agnostic Type-20 builder with the one-time key's fvk/ask, and
/// 6. broadcasts + waits with the same fetch-by-derived-id fallback.
///
/// The whole denomination leaves the pool; any spent value above it re-enters as
/// a single change note to `change_address` (the claimer's OWN default Orchard
/// address — over-funding is expected to be zero for a one-time invitation key,
/// but is handled). There is NO wallet-side note reservation to take or release:
/// the spent notes belong to the foreign key, not to any subwallet, so an
/// unconfirmed broadcast simply leaves the on-chain nullifiers as the
/// authoritative no-reuse guarantee.
///
/// `funding_birth_height` is an advisory hint only (see
/// [`super::sync::scan_notes_for_foreign_key`] — the tree has no height→position
/// oracle, so it cannot seed the scan start today; repeated attempts are
/// bounded by the scan's coordinator-owned resume checkpoint instead).
///
/// Returns the new identity's id, the proof-verified [`Identity`], and the
/// claim's retained recovery record. The caller registers that identity in its
/// local `IdentityManager` and, once that registration is DURABLE, passes the
/// record to [`acknowledge_one_time_claim_registration`] — which is what
/// finally drops it. Until then the record survives the native return, so a
/// caller that dies, cannot find its wallet entry, or fails to persist the
/// identity leaves a retry able to recover the exact identity id rather than
/// hitting the terminal [`PlatformWalletError::ShieldedInviteAlreadyClaimed`]
/// (#4313 review finding 325ce9fa8f84).
///
/// `detached` is the claimer wallet's irreversible removal flag. It is re-read
/// once this claim holds store admission, so a claim that begins after a wallet
/// removal has committed is refused with
/// [`PlatformWalletError::WalletNotFound`] before anything is scanned, built or
/// broadcast (#4313 review finding 4a2c679745bb).
#[allow(clippy::too_many_arguments)]
pub async fn identity_create_from_one_time_key<S, P, IS>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    // Coordinator-owned per-FVK single-flight guards — see
    // [`ForeignClaimGuards`]. Acquired for the WHOLE body, so concurrent
    // same-key claims serialize instead of racing the durable record.
    claim_guards: &ForeignClaimGuards,
    // Coordinator-owned transient-scan resume checkpoints — see
    // [`super::sync::ForeignScanCheckpointCache`] for the chain-isolation
    // contract.
    scan_checkpoints: &super::sync::ForeignScanCheckpointCache,
    // The claimer wallet's irreversible `shielded_detached` flag, re-read once
    // this claim holds store admission (#4313 review finding 4a2c679745bb).
    // See the re-check below for why the caller's own pre-flight check cannot
    // stand alone.
    detached: &std::sync::atomic::AtomicBool,
    // Claimer's wallet id — keys the durable pending-claim record (under the
    // reserved `ONE_TIME_CLAIM_RECORDS_ACCOUNT` subwallet of this wallet).
    wallet_id: WalletId,
    // Bearer spend authority: carried in a `Zeroizing` buffer so every wallet-layer
    // copy of the one-time spending key is scrubbed on drop (#4204 key-hygiene).
    one_time_sk: zeroize::Zeroizing<[u8; 32]>,
    funding_birth_height: Option<u32>,
    change_address: &OrchardAddress,
    identity_index: u32,
    public_keys: Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)>,
    denomination: u64,
    send_to_address_on_creation_failure: PlatformAddress,
    identity_signer: &IS,
    prover: &P,
) -> Result<OneTimeClaimOutcome, PlatformWalletError>
where
    S: ShieldedStore,
    P: OrchardProver,
    IS: Signer<IdentityPublicKey>,
{
    use grovedb_commitment_tree::{FullViewingKey, Scope, SpendAuthorizingKey, SpendingKey};

    if public_keys.is_empty() {
        return Err(PlatformWalletError::ShieldedBuildError(
            "identity-create-from-one-time-key requires at least one public key".to_string(),
        ));
    }

    // Derive the Orchard key material from the one-time spending key. `from_bytes`
    // returns a `CtOption`; an invalid scalar means the caller handed us a
    // non-key, which is a hard input error.
    //
    // KEY HYGIENE (#4204 finding 1ee08ba70627): orchard 0.14's `SpendingKey`
    // and `SpendAuthorizingKey` are `Copy` types with no `Zeroize` support, so
    // holding them as plain locals would leave complete spend-authority
    // representations in this LONG-LIVED async frame across every network
    // await below. Both are contained in [`super::keys::ScrubOnDrop`] guards
    // (volatile-scrubbed on every exit path) and explicitly dropped at their
    // final use: `sk` right after the derivations here, `ask` right after the
    // bundle build. The `*one_time_sk` deref feeding `from_bytes` is the one
    // unavoidable transient copy (orchard's API takes the array by value); the
    // `Zeroizing` parameter itself scrubs the wallet-layer buffer on drop.
    let sk = super::keys::ScrubOnDrop(
        Option::<SpendingKey>::from(SpendingKey::from_bytes(*one_time_sk)).ok_or_else(|| {
            PlatformWalletError::ShieldedKeyDerivation(
                "one-time spending key is not a valid Orchard SpendingKey".to_string(),
            )
        })?,
    );
    let fvk = FullViewingKey::from(&*sk);
    let ask = super::keys::ScrubOnDrop(SpendAuthorizingKey::from(&*sk));
    let ivk = fvk.to_ivk(Scope::External);
    // The spending key's final use is behind us — scrub it before any network
    // work; only the spend-auth key must survive to the bundle build.
    drop(sk);

    let num_keys = public_keys.len();

    // The invitee's re-derivable MASTER auth key hash: the unique, Platform-indexed
    // handle we recover the created identity by if a claim turns out to have
    // already executed (idempotent-retry recovery — see the spent-nullifier
    // preflight and the broadcast handling below). Captured before `public_keys` is
    // moved into the builder. `None` only if the caller submitted no master auth
    // key (identity creation requires one, so this is defensive).
    let master_key_hash = master_auth_public_key_hash(&public_keys);

    // Snapshot the submitted keys for the defensive empty-`public_keys` fill (the
    // binding signature committed exactly these; same pattern as the pool op).
    let submitted_public_keys: BTreeMap<u32, IdentityPublicKey> = public_keys
        .iter()
        .map(|(key, _)| (key.id(), key.clone()))
        .collect();

    // ---- Per-FVK single-flight (#4313 review finding 979bbc2fcb3c) ----
    //
    // Serialize the COMPLETE claim lifecycle for this invitation key —
    // pending-record lookup, transient scan, transition construction, atomic
    // arming, broadcast, and finalization — before touching any shared state.
    // Without it, two concurrent claims for the same key both see no pending
    // record, build transitions with DIFFERENT padded identity ids, and the
    // second `arm_one_time_claim_record` (INSERT-OR-REPLACE) overwrites the
    // first's byte-exact recovery row while its broadcast may already be on
    // the wire — stranding that identity forever. A parked second caller
    // instead resumes the settled record when the guard lifts. The guard is
    // an async mutex (held across every await below; released on drop, so a
    // cancelled claim cannot wedge the key) owned by the SAME coordinator
    // that owns the record store it protects.
    let claim_record_key = one_time_claim_record_key(&fvk);
    let lifecycle_entry = claim_guards.entry_for(claim_record_key);
    let _lifecycle_guard = lifecycle_entry.lock().await;

    // ---- Store-level lifecycle admission (#4313 review finding cr-7e6c98b9) ----
    //
    // The guard above is per-COORDINATOR: it serializes same-key claims that
    // share this `NetworkShieldedCoordinator`, and nothing else. It cannot
    // order this claim against `clear` / `unregister_wallet` / `remove_wallet`,
    // which take the coordinator's `lifecycle` mutex (a claim takes neither),
    // and it cannot reach a SECOND coordinator or process at all — those get
    // their own `FileBackedShieldedStore` with its own SQLite connections to
    // the same file. Without admission, such a purge deletes this claim's
    // pending record while its transition is broadcasting, and the identity it
    // creates is unrecoverable.
    //
    // So admission is taken where the contention actually is — the store. A
    // destructive operation that already holds admission refuses this claim
    // outright (nothing scanned, built or broadcast); one that starts later
    // sees this lease and waits for it. Both directions are decided by a
    // single atomic store step on each side, so there is no interleaving in
    // which both proceed — see `store::LifecycleAdmission`.
    //
    // Released deterministically after the claim body below. A claim CANCELLED
    // mid-flight (a dropped JNI call) cannot run an async release from `Drop`,
    // so its lease is reclaimed by expiry instead — which errs toward "the
    // purge waits", never toward "the record is deleted".
    let admission = super::store::AdmissionToken::generate()?;
    let admitted = store
        .write()
        .await
        .begin_claim_admission(
            wallet_id,
            admission,
            super::store::admission_now_ms(),
            super::store::CLAIM_LEASE_MS,
        )
        .map_err(|e| {
            PlatformWalletError::Persistence(format!(
                "one-time claim: could not take store lifecycle admission: {e}"
            ))
        })?;
    if !admitted {
        return Err(PlatformWalletError::ShieldedLifecycleBusy {
            reason: "this wallet's shielded state is being cleared or removed; the invitation \
                     claim was not started"
                .to_string(),
        });
    }

    // ---- Removal fence, re-checked UNDER admission (#4313 review finding
    // 4a2c679745bb) ----
    //
    // The caller checks `shielded_detached` before it gets here, and that check
    // cannot stand alone: an FFI caller can resolve and retain the wallet and
    // coordinator, pass the pre-flight check, and only then have
    // `unregister_wallet_with` commit the detach, purge the store, release its
    // destructive barrier, and drop the wallet from the manager. The stale
    // handle would then take a FRESH admission (the barrier is gone), arm a new
    // pending row, and broadcast for a wallet the host has already removed —
    // and its registration tail, finding no manager entry, would leave that row
    // behind.
    //
    // Re-reading the flag HERE is what closes it, because the two admissions
    // are mutually exclusive at the store and totally ordered by it:
    //
    // * If the removal got there first, it has already set `detached` (the flag
    //   is set by `on_admitted`, which runs after IT holds admission), so
    //   either our `begin_claim_admission` above was refused, or the removal
    //   has fully finished and this read sees `true`.
    // * If we got here first, the removal cannot set the flag until it is
    //   admitted, and it cannot be admitted until we release. So a `false` read
    //   under our own lease stays true for the whole claim.
    //
    // Typed as `WalletNotFound`, deliberately NOT `ShieldedLifecycleBusy`: the
    // wallet is gone for good, so this is terminal for this wallet and a retry
    // can only fail again. It is equally not the invitation's terminal
    // `ShieldedInviteAlreadyClaimed` — nothing was spent, and the invitation
    // remains claimable by some other wallet.
    if detached.load(std::sync::atomic::Ordering::Acquire) {
        release_claim_admission(store, admission).await;
        return Err(PlatformWalletError::WalletNotFound(format!(
            "{} was removed from the manager while the invitation claim was starting; nothing \
             was scanned, built or broadcast",
            hex::encode(wallet_id)
        )));
    }

    // ---- Durable per-INVITATION reservation (#4313 review finding cr-9d0e1a44) ----
    //
    // The lease above is per-WALLET: it orders this claim against a purge and
    // nothing else, so it admits BOTH claims of the same invitation. The
    // per-FVK mutex above serializes same-key claims that share THIS
    // coordinator; a second coordinator, or a second process, opens its own
    // SQLite connections to the same file and shares no lock at all. Both then
    // found no pending record, built transitions with DIFFERENT padded identity
    // ids, and the loser's `arm_redrive_under_claim` — an INSERT OR REPLACE —
    // silently overwrote the winner's byte-exact recovery row while the
    // winner's transition was already on the wire.
    //
    // So the invitation's record key is reserved durably, by an insert that
    // cannot overwrite a live row, and the DURABLE row is read back to decide
    // who owns it. The in-process guard stays as the fast path — it parks a
    // same-coordinator second caller before it ever reaches the store — and
    // this is the backstop for the two cases that guard cannot see.
    //
    // The SAME atomic step also returns the durable pending-claim record, if
    // one already exists (#4313 review finding r3767229122). It has to. The
    // record lookup used to run separately, through `pending_redrives`, which
    // in the file-backed store reads an in-memory mirror hydrated once at
    // store OPEN — so a record a PEER store armed after that open was
    // invisible. This claim would then see "no record", build a SECOND
    // transition with a different padded identity id, and replace the peer's
    // only recovery row while the peer's transition was already on the wire,
    // stranding the identity that transition creates. Reading the row inside
    // the reservation's transaction closes the gap by construction: whoever
    // settles the reservation settles what already exists under it, together.
    let claim_records_id = SubwalletId::new(wallet_id, ONE_TIME_CLAIM_RECORDS_ACCOUNT);
    let reservation = store
        .write()
        .await
        .reserve_one_time_claim_key(
            claim_records_id,
            claim_record_key,
            admission,
            super::store::admission_now_ms(),
            super::store::CLAIM_LEASE_MS,
        )
        .map_err(|e| {
            // Fail closed. Proceeding without knowing who owns the key is
            // exactly the clobber this reservation exists to prevent.
            PlatformWalletError::Persistence(format!(
                "one-time claim: could not reserve the invitation's claim-record key: {e}"
            ))
        });
    let reservation = match reservation {
        Ok(r) => r,
        Err(e) => {
            release_claim_admission(store, admission).await;
            return Err(e);
        }
    };
    if let super::store::ClaimKeyReservation::Held { holder, expires_at } = reservation.reservation
    {
        debug!(
            holder = %hex::encode(holder.0),
            expires_at,
            resumable_record = reservation.pending.is_some(),
            "one-time claim: another claimant holds this invitation's claim-record key"
        );
    }

    // The heartbeat wraps the COMPLETE admitted claim body — pending-record
    // lookup, resume, transient scan, build, arm, broadcast and confirmation
    // wait alike (#4313 review finding 8de8d05a). See
    // `under_renewed_claim_lease` for why it cannot sit any deeper.
    let claim_result = under_renewed_claim_lease(
        store,
        admission,
        one_time_claim_admitted(
            sdk,
            store,
            scan_checkpoints,
            claim_records_id,
            claim_record_key,
            admission,
            reservation.is_acquired(),
            reservation.pending,
            fvk,
            ivk,
            ask,
            funding_birth_height,
            change_address,
            identity_index,
            public_keys,
            num_keys,
            master_key_hash,
            submitted_public_keys,
            denomination,
            send_to_address_on_creation_failure,
            identity_signer,
            prover,
        ),
    )
    .await;

    release_claim_admission(store, admission).await;
    let (identity_id, identity, recovery_record) = claim_result?;
    Ok(OneTimeClaimOutcome {
        identity_id,
        identity,
        recovery_record,
    })
}

/// Drive `body` to completion while re-stamping the claim lease `admission` on
/// a [`CLAIM_LEASE_RENEW_INTERVAL`](super::store::CLAIM_LEASE_RENEW_INTERVAL)
/// timer, so the protected window is tied to how long the claim actually runs
/// rather than to a fixed guess from whenever the lease was last stamped.
///
/// # Why it wraps the whole admitted body
///
/// The heartbeat originally wrapped only the fresh-build path's broadcast. That
/// left the RESUME path — pending-record lookup, nullifier queries, repeated
/// identity recovery, re-broadcast of the stored transition and an unbounded
/// confirmation wait — running under the INITIAL lease alone, because it
/// returns before the fresh-build path is ever reached (#4313 review finding
/// 8de8d05a). Resume is if anything the slower of the two: it is the path a
/// claim takes precisely because the previous attempt could not resolve
/// quickly.
///
/// A lease that lapses mid-claim is reaped, at which point a concurrent purge
/// counts zero live claims and deletes the very record the in-flight claim
/// needs to recover — so the window has to cover every phase between taking the
/// lease and releasing it. Hoisting it to the single call site around
/// [`one_time_claim_admitted`] covers both paths by construction, and there is
/// no deeper place that could: the two paths only converge here.
///
/// The claim-key reservation taken under the same token rides along, because
/// [`ShieldedStore::renew_claim_admission`] re-stamps it in the same step.
///
/// Cancellation-safe: dropping the returned future drops `body` with it, and
/// the lease is then reclaimed by expiry.
async fn under_renewed_claim_lease<S, F, T>(
    store: &Arc<RwLock<S>>,
    admission: super::store::AdmissionToken,
    body: F,
) -> T
where
    S: ShieldedStore,
    F: std::future::Future<Output = T>,
{
    tokio::pin!(body);
    loop {
        tokio::select! {
            // Bias the body so a renewal tick can never starve the outcome we
            // are actually waiting for.
            biased;
            outcome = &mut body => break outcome,
            _ = tokio::time::sleep(super::store::CLAIM_LEASE_RENEW_INTERVAL) => {
                let renewed = store
                    .write()
                    .await
                    .renew_claim_admission(
                        admission,
                        super::store::admission_now_ms(),
                        super::store::CLAIM_LEASE_MS,
                    );
                match renewed {
                    Ok(true) => {}
                    // A transition may already be on the wire; aborting cannot
                    // un-send it and would only lose the outcome
                    // classification. Carry on, loudly — and note that this is
                    // NOT what protects the claim: the body re-proves ownership
                    // synchronously immediately before every chargeable step
                    // (`assert_claim_lease_before_chargeable_step`), so a claim
                    // that reaches this arm has either not broadcast yet — and
                    // will refuse to — or is already past the point where
                    // stopping helps (#4313 review finding f58ed9d910d8).
                    Ok(false) => error!(
                        "one-time claim lease lapsed or was displaced mid-claim; a \
                         concurrent wallet removal may purge this claim's recovery record"
                    ),
                    Err(e) => warn!(
                        error = %e,
                        "could not renew the one-time claim lease mid-claim"
                    ),
                }
            }
        }
    }
}

/// Re-prove, synchronously and immediately before a CHARGEABLE or otherwise
/// irreversible step, that this claim still owns its purge-protection lease.
///
/// # Why the heartbeat is not enough
///
/// [`under_renewed_claim_lease`] logs a failed renewal and lets the claim run
/// on. Once a transition has been armed that no longer preserves the recovery
/// invariant: [`ShieldedStore::renew_claim_admission`] deliberately refuses to
/// resurrect an expired lease, and destructive admission reaps expired leases
/// before counting live claims. A forward wall-clock adjustment, a run of
/// SQLite errors, or a long executor suspension can therefore leave a live
/// claim non-renewable — and a concurrent `clear` or `unregister_wallet` then
/// counts zero claims and deletes the pending row while the transition is on
/// the wire. For a padded single-note bundle that row is the ONLY handle to
/// the randomized identity id, so losing it strands the identity permanently
/// (#4313 review finding f58ed9d910d8).
///
/// # The gate
///
/// So ownership is proven positively at the last instant before it matters,
/// rather than assumed from a heartbeat that is allowed to fail. A renewal
/// that returns `Ok(true)` both proves the lease is live and re-stamps it (and
/// the claim-key reservation riding the same token), so the broadcast that
/// follows runs at the START of a full lease window rather than at whatever
/// remained of one a long proof build had already spent.
///
/// Everything else fails CLOSED and retryably:
///
/// - `Ok(false)` — the lease is definitively gone (expired or displaced);
/// - `Err(_)` — renewal can no longer PROVE ownership, which for this purpose
///   is the same thing. Treating a store error as "probably still ours" is
///   exactly the repeated-SQLite-error case the finding calls out.
///
/// Refusing here is clean: the caller has not broadcast, so nothing is
/// consumed, nothing is charged, and no proof is burned. The armed record
/// stays on disk with its notes unspent, so the retry resumes it and
/// re-broadcasts the byte-identical transition. If instead the record IS
/// purged in the meantime, that too is harmless — nothing was ever on the
/// wire, so a fresh claim simply rebuilds.
async fn assert_claim_lease_before_chargeable_step<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    admission: super::store::AdmissionToken,
    step: &str,
) -> Result<(), PlatformWalletError> {
    let renewed = store.write().await.renew_claim_admission(
        admission,
        super::store::admission_now_ms(),
        super::store::CLAIM_LEASE_MS,
    );
    match renewed {
        Ok(true) => Ok(()),
        Ok(false) => {
            error!(
                step,
                "one-time claim: the purge-protection lease is gone; refusing to proceed to a \
                 chargeable step without it"
            );
            Err(PlatformWalletError::ShieldedLifecycleBusy {
                reason: format!(
                    "this claim's purge-protection lease lapsed or was displaced before {step}; \
                     without it a concurrent wallet clear or removal could delete the recovery \
                     record while the transition was in flight, so nothing was broadcast — retry \
                     the claim"
                ),
            })
        }
        Err(e) => {
            error!(
                step,
                error = %e,
                "one-time claim: could not prove the purge-protection lease is still held; \
                 refusing to proceed to a chargeable step"
            );
            Err(PlatformWalletError::ShieldedLifecycleBusy {
                reason: format!(
                    "this claim's purge-protection lease could not be verified before {step} \
                     ({e}); proceeding without proof of ownership risks losing the recovery \
                     record mid-flight, so nothing was broadcast — retry the claim"
                ),
            })
        }
    }
}

/// Release a claim's store admission — its lifecycle lease AND the claim-key
/// reservation taken under the same token. Best-effort: both carry an expiry,
/// so a failure here only delays the next claimant of this invitation.
async fn release_claim_admission<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    admission: super::store::AdmissionToken,
) {
    if let Err(e) = store.write().await.end_claim_admission(admission) {
        warn!(
            error = %e,
            "one-time claim: failed to release the store lifecycle admission; it expires on its own"
        );
    }
}

/// The one-time-key claim body, running under a held store admission lease.
///
/// Split out of [`identity_create_from_one_time_key`] purely so the lease is
/// released on EVERY exit — including the many `?` paths — without threading a
/// release through each of them (#4313). The caller owns acquire/release; this
/// function owns the claim.
///
/// `owns_claim_key` is whether this caller won the durable per-invitation
/// reservation. A caller that did NOT is forbidden to build, broadcast or arm
/// anything: it may only RESUME the record the owner left, or refuse. See the
/// branch below.
///
/// `pending_record` is that record, read from DURABLE state in the same atomic
/// step that settled the reservation. It is passed in rather than looked up
/// here on purpose: the file-backed store's `pending_redrives` reads a mirror
/// hydrated at store open, which cannot see a row a peer store armed later, and
/// a claim that trusted it would build a second transition over a live one
/// (#4313 review finding r3767229122).
#[allow(clippy::too_many_arguments)]
async fn one_time_claim_admitted<S, P, IS>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    scan_checkpoints: &super::sync::ForeignScanCheckpointCache,
    claim_records_id: SubwalletId,
    claim_record_key: [u8; 32],
    admission: super::store::AdmissionToken,
    owns_claim_key: bool,
    pending_record: Option<PendingRedrive>,
    fvk: grovedb_commitment_tree::FullViewingKey,
    ivk: grovedb_commitment_tree::IncomingViewingKey,
    ask: super::keys::ScrubOnDrop<grovedb_commitment_tree::SpendAuthorizingKey>,
    funding_birth_height: Option<u32>,
    change_address: &OrchardAddress,
    identity_index: u32,
    public_keys: Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)>,
    num_keys: usize,
    master_key_hash: Option<[u8; 20]>,
    submitted_public_keys: BTreeMap<u32, IdentityPublicKey>,
    denomination: u64,
    send_to_address_on_creation_failure: PlatformAddress,
    identity_signer: &IS,
    prover: &P,
) -> Result<(Identifier, Identity, Option<OneTimeClaimRecoveryRecord>), PlatformWalletError>
where
    S: ShieldedStore,
    P: OrchardProver,
    IS: Signer<IdentityPublicKey>,
{
    // Advisory only: the shielded tree has no height→note-index oracle (a chunk's
    // block_height is the proof-tip height, not per-note inclusion height), so the
    // transient scan cannot seed its start from a height; it bounds itself by
    // value coverage plus a coordinator-owned resume checkpoint (one
    // full-history scan per key per coordinator — see
    // `scan_notes_for_foreign_key`). Logged so
    // the hint is observable and not silently dropped.
    if let Some(h) = funding_birth_height {
        debug!(
            funding_birth_height = h,
            "identity_create_from_one_time_key: birth-height hint (advisory; scan is value-bounded)"
        );
    }

    // ---- Durable pending-claim resume (#4204 review finding c0781f9d387f) ----
    //
    // A claim that broadcast but never confirmed (process death, JNI
    // cancellation, lost result wait) left a persisted record carrying the
    // byte-exact transition and its declared identity id. Consult it BEFORE
    // the transient scan: the record's id survives even for a padded
    // single-note bundle (whose id embeds a random dummy nullifier and is
    // otherwise unrecoverable), so a retry can reconcile or re-drive the
    // byte-identical transition instead of rebuilding one whose preflight
    // would misread the spent notes as a foreign claim
    // (`ShieldedInviteAlreadyClaimed`).
    //
    // The record arrives from the caller, read out of DURABLE state in the same
    // transaction that settled the claim-key reservation — never from the
    // store's startup-hydrated mirror (#4313 review finding r3767229122).
    if let Some(record) = pending_record {
        match resume_one_time_claim(
            sdk,
            store,
            claim_records_id,
            &record,
            admission,
            master_key_hash,
            submitted_public_keys.clone(),
            denomination,
            identity_index,
        )
        .await
        {
            OneTimeClaimResume::Resolved(result) => {
                // Same retention contract as the fresh-build tail below: a
                // successful resume keeps its record until the caller
                // acknowledges durable local registration
                // (#4313 review finding 325ce9fa8f84). This branch had the
                // identical premature-clear ordering.
                let retained = finalize_one_time_claim_record(
                    store,
                    claim_records_id,
                    claim_record_key,
                    &result,
                )
                .await;
                let (identity_id, identity) = result?;
                return Ok((identity_id, identity, retained));
            }
            // The stored transition is unusable (corrupt, or definitively
            // rejected while its notes are provably unspent) — the record has
            // been cleared; build a fresh claim below.
            OneTimeClaimResume::RecordUnusable => {}
        }
    }

    // ---- Losing claimant: RESUME above, or REFUSE here ----
    //
    // Everything past this point builds a fresh transition and arms a fresh
    // record under `claim_record_key`. Only the holder of the durable
    // per-invitation reservation may do that (#4313 review finding
    // cr-9d0e1a44). A claimant that lost the reservation reaches here in
    // exactly two states, and neither may proceed:
    //
    // * the owner has already armed its record — then the resume above ran and
    //   returned, so we are not here at all (or the record was unusable and the
    //   owner will re-arm it, which is still not ours to do);
    // * the owner is admitted but has not armed yet — building here is
    //   precisely the race: two transitions with different padded identity ids,
    //   and whichever arms second overwrites the other's only recovery handle.
    //
    // So refuse, retryably. Nothing was scanned, built or broadcast; the note
    // is untouched; a retry a moment later either finds the owner's record and
    // resumes it, or finds the key free and proceeds as the owner. The storage
    // layer refuses the same case independently — `arm_redrive_under_claim`
    // will not write under a foreign reservation — so this branch is the clean
    // error, not the safety property.
    if !owns_claim_key {
        return Err(PlatformWalletError::ShieldedLifecycleBusy {
            reason: "another claimant currently holds this invitation's claim record; nothing \
                     was scanned, built or broadcast — retry shortly, and the retry will either \
                     resume that claim's outcome or take the invitation over if it lapsed"
                .to_string(),
        });
    }

    // Transient scan: re-derive the one-time key's note(s) from the network.
    let discovered =
        super::sync::scan_notes_for_foreign_key(sdk, scan_checkpoints, &fvk, &ivk, denomination)
            .await?;
    if discovered.is_empty() {
        // No note decrypts under this key — nothing was funded to it (or the
        // wallet hasn't synced far enough to see it yet).
        return Err(PlatformWalletError::ShieldedNoUnspentNotes);
    }

    // Exact-equality selection over the transiently-scanned set: cover exactly
    // `denomination`, gate on `denomination > predicted_fee`. Surfaces
    // `ShieldedInsufficientBalance { available, required }` when the key's notes
    // don't cover the denomination, mirroring the pool-funded neighbor.
    let (selected_refs, total_input, predicted_fee) =
        select_notes_for_denomination(&discovered, denomination, 2, num_keys, sdk.version())?;
    let selected_notes: Vec<ShieldedNote> = selected_refs.into_iter().cloned().collect();

    info!(
        denomination,
        predicted_fee,
        inputs = selected_notes.len(),
        total_input,
        keys = num_keys,
        "IdentityCreateFromOneTimeKey"
    );

    // Idempotent-retry preflight (no persisted record for this key). If this one-time key's
    // selected note(s) are ALREADY spent on chain, a byte-identical claim already
    // executed — so we must NOT rebuild+rebroadcast (that would only earn a
    // `NullifierAlreadySpent` rejection). Everything checked here is re-derived
    // from the invite the invitee holds: the one-time key → its note(s) via the
    // transient scan above, and each note's real nullifier (`ShieldedNote.nullifier`,
    // stamped `note.nullifier(fvk)` during the scan). If spent, recover the
    // previously-created identity by the invitee's own re-derivable MASTER auth key
    // hash (`discover_inner`'s unique-hash probe) and return it as success.
    let selected_nullifiers: Vec<[u8; 32]> = selected_notes.iter().map(|n| n.nullifier).collect();

    // The id that an identity created by THIS claim must carry — the single
    // handle that ties a recovered identity back to this claim's spend, and the
    // reason a MASTER-key-hash hit alone is not evidence of a successful claim
    // (see `recovered_identity_matches_claim`).
    //
    // Consensus derives the new identity id as `double_sha256` over the SORTED
    // set of PUBLISHED action nullifiers (`derive_identity_id_from_actions`) and
    // rejects a transition whose declared id differs, so this is a binding, not a
    // guess.
    //
    // `None` for a single-spend claim: the builder pads to Orchard's 2-action
    // minimum (`num_actions = spends.len().max(2)`) and the padding action's
    // dummy nullifier is randomly generated per build, so it participates in the
    // derivation but cannot be reproduced on a retry. With two or more real
    // spends no padding is added and the published set is exactly
    // `selected_nullifiers`.
    let expected_identity_id =
        (selected_notes.len() >= 2).then(|| identity_id_from_nullifiers(&selected_nullifiers));

    // Idempotent-retry preflight. If this one-time key's selected note(s) are
    // ALREADY spent on chain, this claim can never execute — rebuilding and
    // rebroadcasting would only earn a `NullifierAlreadySpent` rejection and burn
    // a Halo 2 proof. Hand off to the reconciler, which decides between "this
    // claim created that identity" (both bindings verified), "the invitation is
    // gone" (terminal), and "executed but not yet indexed" (retryable).
    // `Unknown` proceeds here — that is safe pre-broadcast: the idempotent
    // broadcast path reconciles via the `NullifierAlreadySpent` verdict, so a
    // transient query failure only costs a harmless rebuild.
    if nullifier_spent_status(sdk, &selected_nullifiers).await == NullifierSpentStatus::Spent {
        // No record was armed on this path — nothing was built or broadcast —
        // so there is none to retain or acknowledge.
        let (identity_id, identity) = recover_executed_one_time_claim(
            sdk,
            master_key_hash,
            expected_identity_id,
            false,
            "the selected note's nullifier is already spent on chain (pre-broadcast preflight)",
        )
        .await?;
        return Ok((identity_id, identity, None));
    }

    // Witness the selected notes against a Platform-recorded anchor from the
    // shared, fully-marked commitment tree (identical probe to the pool op).
    let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
    let anchor_bytes = anchor.to_bytes();

    let build = build_identity_create_from_shielded_pool_transition(
        public_keys,
        denomination,
        send_to_address_on_creation_failure,
        spends,
        change_address,
        &fvk,
        &ask,
        anchor,
        prover,
        identity_signer,
        [0u8; 36],
        sdk.version(),
    )
    .await
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
    // The spend-auth key's final use (the bundle build + spend-auth
    // signatures above) is behind us — scrub it before the broadcast and
    // result wait keep this frame alive across the network.
    drop(ask);

    let identity_id = build.identity_id;

    // Re-assemble the transition from the PoP-signed keys + bundle params
    // (preserving the per-key signatures) and broadcast. The broadcast/wait
    // classification mirrors `identity_create_from_shielded_pool` verbatim, minus
    // the note-reservation bookkeeping (there is no subwallet reservation to
    // release — the spent notes belong to the foreign one-time key).
    let st = sdk
        .identity_create_from_shielded_pool_transition(
            build.public_keys,
            denomination,
            send_to_address_on_creation_failure,
            build.bundle,
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

    // Persist the pending-claim record BEFORE the broadcast (#4204 review
    // finding c0781f9d387f): once the transition leaves this process, the
    // declared id — the only handle that recovers a padded single-note claim —
    // must already be durable. Fail-closed: nothing has been consumed yet, so
    // refusing to broadcast on a persistence failure is a clean, retryable
    // stop; broadcasting without the record risks an unrecoverable
    // `ShieldedInviteAlreadyClaimed` on the next attempt.
    arm_one_time_claim_record(
        store,
        claim_records_id,
        claim_record_key,
        anchor_bytes,
        &selected_nullifiers,
        &st,
        admission,
        identity_index,
    )
    .await?;

    // The renewal heartbeat that holds the lease open across this broadcast and
    // its confirmation wait runs in the CALLER, around the whole admitted claim
    // body — see `under_renewed_claim_lease`. It used to wrap only this
    // broadcast, which left the resume path (returning above) covered by the
    // initial lease alone (#4313 review finding 8de8d05a).
    //
    // But the heartbeat only LOGS a failed renewal, so it cannot be what
    // authorizes the broadcast. Prove ownership synchronously here instead: the
    // record above is armed and the transition is about to become chargeable
    // and irreversible, and from this point on the record is the only handle
    // that can recover a padded single-note claim's identity. If the lease
    // cannot be re-proven, abort — nothing has been broadcast, the notes are
    // unspent, and the retry resumes this very record
    // (#4313 review finding f58ed9d910d8).
    assert_claim_lease_before_chargeable_step(store, admission, "the claim broadcast").await?;

    let result = broadcast_and_confirm_one_time_claim(
        sdk,
        st,
        identity_id,
        expected_identity_id,
        master_key_hash,
        &selected_nullifiers,
        submitted_public_keys,
        denomination,
    )
    .await;
    // On success the record is RETAINED and handed to the caller, which drops
    // it only once the identity is durably registered locally. Clearing it here
    // — as this used to — lost the padded identity id whenever the process died,
    // the wallet-manager entry was missing, or `persister.store` failed, between
    // this line and the JNI return (#4313 review finding 325ce9fa8f84).
    let retained =
        finalize_one_time_claim_record(store, claim_records_id, claim_record_key, &result).await;
    let (identity_id, identity) = result?;
    Ok((identity_id, identity, retained))
}

/// Broadcast an assembled one-time-key claim transition and drive it to a
/// classified outcome: proven success, idempotent recovery of an
/// already-executed claim, terminal `ShieldedInviteAlreadyClaimed`, definitive
/// `ShieldedBroadcastFailed`, or retryable `ShieldedBroadcastUnconfirmed`.
///
/// Shared by the fresh-build path and the pending-claim resume path
/// (`resume_one_time_claim`), which re-broadcasts the persisted byte-identical
/// transition. `identity_id` is the id the transition DECLARES;
/// `expected_identity_id` is the pre-build re-derivable id (`None` for a
/// padded single-note bundle on the fresh path; always `Some` on the resume
/// path, where the declared id was recovered from the record).
#[allow(clippy::too_many_arguments)]
async fn broadcast_and_confirm_one_time_claim(
    sdk: &Arc<dash_sdk::Sdk>,
    st: StateTransition,
    identity_id: Identifier,
    expected_identity_id: Option<Identifier>,
    master_key_hash: Option<[u8; 20]>,
    claim_nullifiers: &[[u8; 32]],
    submitted_public_keys: BTreeMap<u32, IdentityPublicKey>,
    denomination: u64,
) -> Result<(Identifier, Identity), PlatformWalletError> {
    match st.broadcast(sdk, None).await {
        Ok(()) => {}
        // A `NullifierAlreadySpent` verdict is NOT a failure on this path: it is
        // positive proof a byte-identical claim already executed (the note is
        // consumed on chain). Recover the created identity instead of stranding
        // the retry. Checked before the generic `broadcast_definitely_failed` arm,
        // which would otherwise classify this consensus rejection as a hard failure.
        //
        // POST-BUILD, the reconciler gets `Some(identity_id)` — the id THIS
        // transition committed — never the pre-build `expected_identity_id`
        // (which is deliberately `None` for a padded single-note bundle). The
        // SDK's broadcast internally retries requests, so an accepted first
        // request whose acknowledgement was lost legitimately produces
        // `NullifierAlreadySpent` on the retry; with `None` the reconciler
        // would declare our own successfully created identity permanently
        // lost (`ShieldedInviteAlreadyClaimed`) instead of recovering it by
        // its exact id (#4204 review finding a00cee018e73).
        Err(e) if is_nullifier_already_spent(&e) => {
            return recover_executed_one_time_claim(
                sdk,
                master_key_hash,
                Some(identity_id),
                false,
                &format!("broadcast returned NullifierAlreadySpent: {e}"),
            )
            .await;
        }
        Err(e) if broadcast_definitely_failed(&e) => {
            return Err(PlatformWalletError::ShieldedBroadcastFailed(e.to_string()));
        }
        Err(e) => {
            warn!(
                derived_id = %identity_id,
                error = %e,
                "IdentityCreateFromOneTimeKey: broadcast returned no verdict; the transition may \
                 have been admitted — falling through to the result wait"
            );
        }
    }

    // Wait for proven execution, mirroring the pool-funded sibling verbatim. A
    // Type-20 IdentityCreateFromShieldedPool proof authenticates the spent
    // nullifiers and resulting identity as an affected-state snapshot; it cannot
    // bind the complete Orchard request, so the current proof contract marks it as
    // affected-state. Use `wait_for_affected_state` — the strict `wait_for_response`
    // would classify every valid claim proof as `ExecutionNotProved`, drop into the
    // ambiguous fallback, and risk reporting a successful claim as unconfirmed.
    let proof_result = match st
        .wait_for_affected_state::<StateTransitionProofResult>(sdk, None)
        .await
    {
        Ok(result) => result,
        // Same idempotent recovery as the broadcast arm: a `NullifierAlreadySpent`
        // verdict surfacing at wait time proves the claim executed, so recover the
        // identity rather than reporting a broadcast failure. Ordered before the
        // generic consensus-rejection arm below (which would classify it as a
        // failure). Same post-build rule as the broadcast arm: pass the id THIS
        // transition committed, never the padding-lossy pre-build one (#4204
        // review finding a00cee018e73).
        Err(wait_err) if is_nullifier_already_spent(&wait_err) => {
            return recover_executed_one_time_claim(
                sdk,
                master_key_hash,
                Some(identity_id),
                false,
                &format!("result wait returned NullifierAlreadySpent: {wait_err}"),
            )
            .await;
        }
        Err(dash_sdk::Error::StateTransitionBroadcastError(e)) if e.cause.is_some() => {
            // A populated cause is a consensus verdict — but for Type 20 a
            // verdict is NOT proof of non-execution: a duplicate unique-key
            // hash makes Drive APPLY the chargeable `UnshieldAction` fallback
            // (the invitation nullifiers are consumed, the fallback address is
            // credited minus the penalty) and record a `PaidConsensusError`,
            // which reaches this arm exactly like a plain rejection. Declaring
            // `ShieldedBroadcastFailed` then would hand the host code 16 —
            // documented as definitive non-execution and safe to retry — for
            // an invitation that is already consumed, and every retry would
            // burn a ~30s proof to earn `NullifierAlreadySpent`. Check the
            // selected nullifiers first: consumed notes prove the transition
            // (or its fallback) APPLIED, so hand off to the reconciler for
            // the terminal claimed/fallback verdict — it distinguishes "this
            // claim created the identity" (recovered as success) from the
            // chargeable fallback / competing claim (terminal
            // `ShieldedInviteAlreadyClaimed`) (#4204 review finding
            // 8d020115b274).
            //
            // The three spent-status outcomes diverge here and only `Unspent`
            // may produce `ShieldedBroadcastFailed`: the host documents that
            // code as definitive non-execution and safe to retry, so it
            // requires PROOF the notes are unconsumed. `Unknown` (query
            // failure / partial response) yields `ShieldedBroadcastUnconfirmed`
            // instead — the armed pending-claim record lets a later retry
            // reconcile with the exact id once the status is queryable.
            match nullifier_spent_status(sdk, claim_nullifiers).await {
                NullifierSpentStatus::Spent => {
                    // `spend_finalized = true`: this claim's own wait returned a
                    // definitive verdict AND the notes are proven consumed, so
                    // "no identity carries our bindings" is the terminal
                    // chargeable-fallback / competing-claim outcome — even when
                    // the colliding unique key was not MASTER and no identity is
                    // findable under either probe.
                    return recover_executed_one_time_claim(
                        sdk,
                        master_key_hash,
                        Some(identity_id),
                        true,
                        &format!(
                            "result wait returned an executed consensus verdict (the invitation \
                             notes are spent — applied claim or chargeable fallback): {e}"
                        ),
                    )
                    .await;
                }
                NullifierSpentStatus::Unspent => {
                    return Err(PlatformWalletError::ShieldedBroadcastFailed(e.to_string()));
                }
                NullifierSpentStatus::Unknown => {
                    return Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
                        identity_id,
                        reason: format!(
                            "consensus verdict received but the invitation notes' spent status \
                             could not be established; not classifying as a definitive failure \
                             (an applied chargeable fallback would be indistinguishable): {e}"
                        ),
                    });
                }
            }
        }
        Err(wait_err) => {
            warn!(
                derived_id = %identity_id,
                error = %wait_err,
                "IdentityCreateFromOneTimeKey: broadcast accepted but result confirmation failed; \
                 falling back to fetching the identity by its derived id"
            );
            match fetch_identity_with_retries(sdk, identity_id).await {
                Some(mut identity) => {
                    // `identity_id` is the id THIS build derived. Whether finding
                    // an identity under it proves this transition created it
                    // depends on whether the bundle was padded:
                    //
                    // - **Padded (single spend)** — the id embeds a locally
                    //   generated random dummy nullifier that no other party can
                    //   reproduce, so an identity at this id can only have come
                    //   from this transition. The id alone is proof.
                    // - **Not padded (>= 2 spends)** — the id is derived from the
                    //   invitation's real nullifiers alone, so any other holder of
                    //   the same bearer one-time key derives the SAME id under
                    //   their own keys. The on-chain MASTER auth key must be
                    //   checked before this can be called ours.
                    if expected_identity_id.is_some()
                        && !recovered_identity_matches_claim(
                            &identity,
                            expected_identity_id,
                            master_key_hash,
                        )
                    {
                        warn!(
                            derived_id = %identity_id,
                            "IdentityCreateFromOneTimeKey: an identity exists at this claim's \
                             derived id but does not carry the submitted master auth key; another \
                             holder of the same one-time key claimed the invitation first"
                        );
                        return Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
                            reason: format!(
                                "identity {identity_id} was created from this invitation's notes \
                                 but does not carry the submitted master authentication key, so it \
                                 belongs to another holder of the one-time key: {wait_err}"
                            ),
                        });
                    }
                    info!(
                        derived_id = %identity_id,
                        "IdentityCreateFromOneTimeKey: result confirmation failed but the identity \
                         was found on chain by its derived id; treating as success"
                    );
                    // Only reached once the identity is proven to be this claim's,
                    // so back-filling the keys this transition itself submitted is
                    // a local-row convenience, not an unproven ownership claim.
                    if identity.public_keys().is_empty() {
                        identity.set_public_keys(submitted_public_keys.clone());
                    }
                    return Ok((identity.id(), identity));
                }
                None => {
                    return Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
                        identity_id,
                        reason: wait_err.to_string(),
                    });
                }
            }
        }
    };

    let identity = match proof_result {
        StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers(mut identity, _n) => {
            if identity.id() != identity_id {
                warn!(
                    derived_id = %identity_id,
                    verified_id = %identity.id(),
                    "IdentityCreateFromOneTimeKey: derived id differs from proof-verified id; using \
                     the proof-verified id"
                );
            }
            if identity.public_keys().is_empty() {
                identity.set_public_keys(submitted_public_keys);
            }
            identity
        }
        other => {
            warn!(
                derived_id = %identity_id,
                result = %other,
                "IdentityCreateFromOneTimeKey: unexpected proof-result variant; synthesizing the \
                 identity from the derived id + submitted keys so the local row still lands"
            );
            Identity::new_with_id_and_keys(identity_id, submitted_public_keys, sdk.version())
                .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?
        }
    };

    info!(
        denomination,
        identity_id = %identity.id(),
        "IdentityCreateFromOneTimeKey broadcast succeeded"
    );
    Ok((identity.id(), identity))
}

/// The synthetic ZIP-32 account index that keys durable one-time-claim records
/// in the [`ShieldedStore`].
///
/// Claim records reuse the store's persisted [`PendingRedrive`] rows (byte-exact
/// transition + nullifiers + anchor), but live under this reserved subwallet so
/// the spend-redrive sync pass — which iterates REAL Orchard accounts — never
/// re-broadcasts or prunes them; their lifecycle is owned entirely by
/// [`identity_create_from_one_time_key`]. ZIP-32 account indices are hardened
/// (`< 2^31`), so `u32::MAX` cannot collide with a real subwallet.
pub(super) const ONE_TIME_CLAIM_RECORDS_ACCOUNT: u32 = u32::MAX;

/// Deterministic record key for a one-time claim: every retry of the same
/// invitation re-derives the same key from the one-time FVK, which is exactly
/// what lets a retry find the record a crashed attempt left behind. Domain-
/// separated so it can never collide with an activity-entry id (sha256 of
/// visible output cmxs) sharing the `PendingRedrive.activity_id` keyspace.
fn one_time_claim_record_key(fvk: &grovedb_commitment_tree::FullViewingKey) -> [u8; 32] {
    use dashcore::hashes::{sha256, Hash};

    let mut preimage = Vec::with_capacity(96 + 33);
    preimage.extend_from_slice(b"platform-wallet:one-time-claim:v1");
    preimage.extend_from_slice(&fvk.to_bytes());
    sha256::Hash::hash(&preimage).to_byte_array()
}

/// The shared per-FVK lifecycle mutex handed to every same-key claimer.
type ClaimGuard = Arc<tokio::sync::Mutex<()>>;

/// One registry row: the guard key (see `one_time_claim_record_key`) paired
/// with a non-owning handle to its guard, so abandoned keys are pruned on the
/// next acquisition rather than pinning the mutex alive.
type ClaimGuardEntry = ([u8; 32], std::sync::Weak<tokio::sync::Mutex<()>>);

/// Per-FVK single-flight guards for the one-time-key claim lifecycle.
///
/// Owned by `NetworkShieldedCoordinator` (the same owner as the durable
/// pending-claim record store the guard protects). Two concurrent
/// [`identity_create_from_one_time_key`] calls for the SAME foreign key would
/// otherwise both observe no pending record, build two transitions whose
/// padded single-note identity ids differ (random padding nullifier), and
/// race `arm_one_time_claim_record` — whose store implementation is an
/// INSERT-OR-REPLACE — so the loser's byte-exact recovery row is silently
/// overwritten and its identity becomes unrecoverable; either caller could
/// also finalize (clear) the shared row while the other is mid-broadcast
/// (#4313 review finding 979bbc2fcb3c / cr-4808dde4). The guard therefore
/// spans the COMPLETE lifecycle — pending-record lookup, transient scan,
/// transition construction, atomic arming, broadcast, and finalization — not
/// just the scan-checkpoint window: the second caller parks until the first
/// settles, then resumes that outcome through the persisted record instead of
/// double-spending the invitation.
///
/// Mechanics: `entry_for` hands every same-key caller the SAME
/// `Arc<tokio::sync::Mutex<()>>` (a live entry is always upgraded, never
/// replaced), whose async lock is cancellation-safe — dropping a parked or
/// mid-claim future releases it. The map holds only `Weak` handles, pruned on
/// every acquisition, so abandoned keys cost nothing and hostile key churn
/// cannot grow the map beyond the keys currently in flight.
#[derive(Default)]
pub struct ForeignClaimGuards {
    entries: std::sync::Mutex<Vec<ClaimGuardEntry>>,
}

impl ForeignClaimGuards {
    /// The shared lifecycle mutex for `key`. Callers `.lock().await` the
    /// returned handle and hold the guard across the whole claim; the
    /// internal registry lock is sync-only and released before any await.
    fn entry_for(&self, key: [u8; 32]) -> ClaimGuard {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entries.retain(|(_, weak)| weak.strong_count() > 0);
        if let Some((_, weak)) = entries.iter().find(|(k, _)| *k == key) {
            if let Some(existing) = weak.upgrade() {
                return existing;
            }
        }
        let fresh = Arc::new(tokio::sync::Mutex::new(()));
        entries.retain(|(k, _)| *k != key);
        entries.push((key, Arc::downgrade(&fresh)));
        fresh
    }
}

/// Look up the pending-claim record for `key` through
/// [`ShieldedStore::pending_redrives`].
///
/// **Test-only.** The production claim path no longer reads the record this
/// way: `pending_redrives` is served from the file-backed store's
/// startup-hydrated mirror, which cannot see a row a peer store armed after
/// our open, and a claim that trusted it would build a second transition over
/// a live one (#4313 review finding r3767229122). The real lookup now comes
/// back from [`ShieldedStore::reserve_one_time_claim_key`], read out of
/// durable state in the same transaction that settles the reservation.
///
/// It survives here because the arm/clear/finalize round-trip tests drive the
/// [`InMemoryShieldedStore`], whose map IS its durable state — so for THEM the
/// two reads are the same read.
///
/// [`InMemoryShieldedStore`]: super::store::InMemoryShieldedStore
#[cfg(test)]
async fn find_one_time_claim_record<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    key: [u8; 32],
) -> Result<Option<PendingRedrive>, PlatformWalletError> {
    store
        .read()
        .await
        .pending_redrives(id)
        .map(|records| records.into_iter().find(|r| r.activity_id == key))
        .map_err(|e| {
            PlatformWalletError::Persistence(format!(
                "pending one-time-claim record lookup failed; refusing to build a fresh claim \
                 while an earlier attempt's record may exist: {e}"
            ))
        })
}

/// Persist the pending-claim record UNDER this claim's store-level admission
/// lease. Called BEFORE the broadcast; a failure aborts the claim (fail-closed
/// — see the call site).
///
/// The lease re-check and the record write are one atomic store step
/// ([`ShieldedStore::arm_redrive_under_claim`]), which is what leaves no gap
/// between "still admitted" and "record written" for a concurrent
/// `clear`/`unregister_wallet` to slot into (#4313). The same step re-stamps
/// the lease (and the claim-key reservation riding the same token), so the
/// record is written into a freshly extended window rather than into whatever
/// remained of one a long scan had already spent. Keeping that window open past
/// this point is the heartbeat's job — see [`under_renewed_claim_lease`].
///
/// A lost lease is a hard stop, not a warning: nothing has been broadcast yet,
/// so refusing is clean and retryable, whereas broadcasting without the record
/// is how a padded single-note claim's identity becomes unrecoverable.
///
/// `identity_index` is persisted with the record because it is the ONE part of
/// the claim's binding the transition cannot witness — a purely local DIP-9
/// slot that appears nowhere in `st_bytes` (#4313 review finding 5d4d6efa).
/// Everything else a resume checks is re-derived from the stored bytes; see
/// [`resume_one_time_claim`].
#[allow(clippy::too_many_arguments)]
async fn arm_one_time_claim_record<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    key: [u8; 32],
    anchor: [u8; 32],
    nullifiers: &[[u8; 32]],
    st: &StateTransition,
    admission: super::store::AdmissionToken,
    identity_index: u32,
) -> Result<(), PlatformWalletError> {
    use dpp::serialization::PlatformSerializable;

    let st_bytes = st
        .serialize_to_bytes()
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
    let admitted = store
        .write()
        .await
        .arm_redrive_under_claim(
            id,
            PendingRedrive {
                activity_id: key,
                anchor,
                nullifiers: nullifiers.to_vec(),
                st_bytes,
                attempts: 0,
                identity_index: Some(identity_index),
            },
            admission,
            super::store::admission_now_ms(),
            super::store::CLAIM_LEASE_MS,
        )
        .map_err(|e| {
            PlatformWalletError::Persistence(format!(
                "failed to persist the pending one-time-claim record before broadcast: {e}"
            ))
        })?;
    if !admitted {
        return Err(PlatformWalletError::ShieldedLifecycleBusy {
            reason: "this claim's store admission lapsed before its recovery record could be \
                     written (the wallet was cleared or removed, or the claim outran its lease); \
                     nothing was broadcast — retry the claim"
                .to_string(),
        });
    }
    Ok(())
}

/// Drop the pending-claim record. Best-effort: a failure only means the next
/// attempt resumes a settled record, which re-resolves to the same outcome.
async fn clear_one_time_claim_record<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    key: [u8; 32],
) {
    if let Err(e) = store.write().await.clear_redrive(id, &key) {
        warn!(
            error = %e,
            "one-time claim: failed to clear the pending-claim record"
        );
    }
}

/// Settle the pending-claim record for a claim that has reached an outcome.
///
/// # A success does NOT clear the record
///
/// It used to. That dropped the row the moment the transition resolved, which
/// is BEFORE control returns through `PlatformWallet::identity_create_from_one_time_key`,
/// where the identity is added to the local manager and handed to the host
/// persister. A process death in that gap — or a missing wallet-manager entry,
/// or a `persister.store` failure — lost the exact padded identity ID before
/// the JNI caller ever received it. The retry then re-scans, finds the notes
/// spent, and `recover_executed_one_time_claim` has no `expected_identity_id`
/// to bind against (a single-spend bundle's id embeds a randomly generated
/// dummy nullifier), so it returns the TERMINAL
/// [`PlatformWalletError::ShieldedInviteAlreadyClaimed`] and the identity is
/// stranded forever (#4313 review finding 325ce9fa8f84).
///
/// So the record — the only durable copy of that id — is RETAINED across the
/// native return and the host persistence handoff, and is dropped only by
/// [`acknowledge_one_time_claim_registration`], once local registration has
/// been durably acknowledged. A retry that arrives before that acknowledgement
/// finds the record, sees the notes already spent, and recovers the identity by
/// its declared id instead of failing terminally — which is exactly the
/// reconciliation state the finding asks for, expressed in the machinery that
/// already exists rather than a second one beside it.
///
/// Retaining is cheap and self-healing: the record is one row keyed by the
/// invitation, the resume path is idempotent, and the acknowledgement clears it
/// on the very next successful pass.
///
/// # What IS cleared here
///
/// Only the terminal [`PlatformWalletError::ShieldedInviteAlreadyClaimed`]: it
/// means the invitation produced no identity this wallet can claim, so no
/// future retry can get anything from the record. Every other error keeps it —
/// `ShieldedBroadcastUnconfirmed` (and unproven failures) are exactly the
/// outcomes whose retry must find the declared id again.
///
/// Returns the retained record's coordinates when the caller is expected to
/// acknowledge it, i.e. on success.
async fn finalize_one_time_claim_record<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    key: [u8; 32],
    result: &Result<(Identifier, Identity), PlatformWalletError>,
) -> Option<OneTimeClaimRecoveryRecord> {
    match result {
        Ok(_) => Some(OneTimeClaimRecoveryRecord {
            claim_records_id: id,
            claim_record_key: key,
        }),
        Err(PlatformWalletError::ShieldedInviteAlreadyClaimed { .. }) => {
            clear_one_time_claim_record(store, id, key).await;
            None
        }
        Err(_) => None,
    }
}

/// The retained recovery record of a SUCCESSFUL one-time-key claim: the durable
/// row that still holds the claim's transition, and with it the exact identity
/// id a padded single-note bundle cannot otherwise reproduce.
///
/// Handed back to the caller by [`identity_create_from_one_time_key`] so the
/// row outlives the native return and the host persistence handoff. The caller
/// registers the identity locally and, ONLY once that registration is durably
/// acknowledged, passes this to [`acknowledge_one_time_claim_registration`]
/// (#4313 review finding 325ce9fa8f84).
///
/// Dropping this value without acknowledging is safe and is the intended
/// behaviour on a registration failure: the record simply survives, and the
/// next claim of the same invitation resumes it and recovers the identity.
#[derive(Debug, Clone, Copy)]
pub struct OneTimeClaimRecoveryRecord {
    claim_records_id: SubwalletId,
    claim_record_key: [u8; 32],
}

/// A successful one-time-key claim: the created identity, plus the recovery
/// record that must be acknowledged once the identity is durably registered
/// locally. See [`OneTimeClaimRecoveryRecord`].
#[derive(Debug)]
pub struct OneTimeClaimOutcome {
    /// The new identity's id.
    pub identity_id: Identifier,
    /// The proof-verified identity, for local registration.
    pub identity: Identity,
    /// The retained recovery record, or `None` when this claim armed no record
    /// (the pre-broadcast preflight recovered an already-executed claim without
    /// ever building one).
    pub recovery_record: Option<OneTimeClaimRecoveryRecord>,
}

/// Drop a successful claim's retained recovery record, now that its identity is
/// durably registered in the caller's local state.
///
/// This is the acknowledgement half of the retention introduced for
/// #4313 review finding 325ce9fa8f84: until it runs, a retry of the same
/// invitation resumes the record and recovers the identity by its declared id
/// rather than failing terminally.
///
/// Call it ONLY after local registration is durable — after the identity is in
/// the manager AND its changeset reached the host persister. Calling it early
/// re-opens the exact window the retention closes; not calling it at all costs
/// one stale row that the next claim of this invitation resolves.
///
/// Best-effort by construction: a failure to clear leaves a settled record that
/// the next attempt re-resolves to the same identity.
pub async fn acknowledge_one_time_claim_registration<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    record: OneTimeClaimRecoveryRecord,
) {
    debug!(
        claim_record_key = %hex::encode(record.claim_record_key),
        "one-time claim: identity durably registered; releasing the retained recovery record"
    );
    clear_one_time_claim_record(store, record.claim_records_id, record.claim_record_key).await;
}

/// Describe the first way a resume attempt's arguments disagree with the
/// transition the earlier attempt submitted, or `None` when they agree.
///
/// Every field compared here is one the resume would otherwise act on with the
/// caller's value while broadcasting the *stored* bytes — the exact mis-binding
/// #4313 review finding 195efdd4ae21 describes. The comparison is on the
/// derived-from-transition side, so it is a statement about what is on the wire,
/// not about what some parallel record claims.
///
/// Key comparison is exact and whole-set: `IdentityPublicKey` compares by id,
/// purpose, security level, key type, read-only flag, contract bounds and key
/// data, so a retry that keeps the ids but swaps the key material — the case
/// that would register a foreign identity at this wallet's slot — is caught.
/// A retry that merely *reorders* the same keys is not a mismatch: both sides
/// are `BTreeMap`s keyed by key id.
#[allow(clippy::too_many_arguments)]
fn one_time_claim_binding_mismatch(
    stored_public_keys: &BTreeMap<u32, IdentityPublicKey>,
    stored_master_key_hash: Option<[u8; 20]>,
    stored_denomination: u64,
    stored_identity_index: Option<u32>,
    submitted_public_keys: &BTreeMap<u32, IdentityPublicKey>,
    submitted_master_key_hash: Option<[u8; 20]>,
    submitted_denomination: u64,
    submitted_identity_index: u32,
) -> Option<String> {
    // The one field compared against the RECORD rather than against the
    // transition, because the transition cannot witness it: the DIP-9 slot is a
    // purely local placement (#4313 review finding 5d4d6efa). Checked first —
    // it is the cheapest comparison, and a slot mismatch is the case whose
    // consequence is least visible: `IdentityManager::add_identity` rejects a
    // duplicate identity id but inserts into an OCCUPIED slot without
    // complaint, so a retry that presents the original keys at a different
    // index would silently displace whatever the wallet tracked there.
    //
    // `None` means the record predates the column; there is nothing to compare,
    // and that record keeps exactly the transitive binding it was written
    // under.
    if let Some(stored) = stored_identity_index {
        if stored != submitted_identity_index {
            return Some(format!(
                "identity index: the earlier attempt was registering at local slot {stored}, \
                 retry asked for slot {submitted_identity_index}"
            ));
        }
    }
    if stored_denomination != submitted_denomination {
        return Some(format!(
            "denomination: stored transition spends {stored_denomination}, retry asked for \
             {submitted_denomination}"
        ));
    }
    if stored_master_key_hash != submitted_master_key_hash {
        // The MASTER auth key hash is the handle `recover_executed_one_time_claim`
        // probes Platform with. Recovering under a hash that is not in the stored
        // transition can only find someone else's identity.
        return Some(format!(
            "master authentication key hash: stored transition carries {}, retry presented {}",
            stored_master_key_hash.map_or_else(|| "none".to_string(), hex::encode),
            submitted_master_key_hash.map_or_else(|| "none".to_string(), hex::encode),
        ));
    }
    if stored_public_keys != submitted_public_keys {
        return Some(format!(
            "public key set: stored transition carries {} key(s) (ids {:?}), retry presented {} \
             key(s) (ids {:?})",
            stored_public_keys.len(),
            stored_public_keys.keys().collect::<Vec<_>>(),
            submitted_public_keys.len(),
            submitted_public_keys.keys().collect::<Vec<_>>(),
        ));
    }
    None
}

/// Outcome of attempting to resume a persisted pending claim.
enum OneTimeClaimResume {
    /// The record drove the claim to an outcome — return it to the caller.
    Resolved(Result<(Identifier, Identity), PlatformWalletError>),
    /// The record cannot drive an outcome (corrupt, wrong transition type, or
    /// definitively rejected with its notes proven unspent). It has been
    /// cleared; the caller builds a fresh claim.
    RecordUnusable,
}

/// Resume a claim from its persisted record (#4204 review finding
/// c0781f9d387f): recover by the DECLARED id when the notes are already
/// consumed, otherwise re-broadcast the byte-identical stored transition —
/// never rebuild while the record is live, because a rebuilt padded bundle
/// derives a fresh random id and orphans the recorded one.
///
/// # The resumed claim is bound to the STORED transition, not to this call
///
/// (#4313 review finding 195efdd4ae21.) The record is found by wallet id and
/// one-time FVK alone, so nothing about the lookup says *which* identity the
/// original attempt was creating. This call's `master_key_hash`,
/// `submitted_public_keys` and `denomination` are therefore treated as
/// **assertions to check**, never as inputs to act on: every one of them is
/// re-derived from `record.st_bytes` — the byte-exact transition the earlier
/// attempt actually put (or is about to put) on the wire — and the derived
/// values are what drive recovery, the empty-proof-result backfill and the
/// re-broadcast.
///
/// Deriving rather than persisting the binding is deliberate wherever the
/// transition can witness it (`public_keys` are exactly what the binding
/// signature committed to; `denomination` is the value that leaves the pool):
/// a derived binding cannot drift from what was submitted the way a separately
/// persisted copy could.
///
/// If the caller's arguments disagree with the transition, this is **not** a
/// resume of the same claim — it is a request to create a different identity
/// from an invitation already committed elsewhere — and it fails closed with
/// [`PlatformWalletError::ShieldedClaimBindingMismatch`]: nothing is
/// re-broadcast (so no chargeable resubmission and no burned proof), and the
/// record is left intact for a retry that presents the original arguments.
///
/// ## What this binds
///
/// Derived from `record.st_bytes`: the submitted key set (by id and content),
/// the MASTER authentication key hash used for idempotent recovery, and the
/// denomination.
///
/// Read from the record itself: `identity_index`, the local DIP-9 slot the
/// returned identity is registered at (#4313 review finding 5d4d6efa). It is
/// the one part of the binding the transition CANNOT witness — a purely local
/// placement that appears nowhere in `st_bytes` — so it is persisted with the
/// claim precisely because there is nothing to derive it from.
///
/// It used to be left to a *transitive* argument: the identity's keys are
/// derived from the wallet seed at that slot, so a retry naming a different
/// slot ought to present different keys and be refused by the key check. That
/// argument leaves the caller free to pair slot `i` with keys derived at slot
/// `j`, and the consequence is not symmetric with a first attempt's: a retry
/// that presents the ORIGINAL keys with a different index reaches
/// `IdentityManager::add_identity`, which rejects a duplicate identity id but
/// inserts into an occupied slot without complaint — silently displacing
/// whatever identity the wallet already tracked there. Persisting the slot and
/// comparing it closes that directly.
///
/// A record written before the column existed carries `None`; there is nothing
/// to compare it against, so the check is skipped and that record keeps exactly
/// the transitive binding it was written under.
#[allow(clippy::too_many_arguments)]
async fn resume_one_time_claim<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    claim_records_id: SubwalletId,
    record: &PendingRedrive,
    admission: super::store::AdmissionToken,
    master_key_hash: Option<[u8; 20]>,
    submitted_public_keys: BTreeMap<u32, IdentityPublicKey>,
    denomination: u64,
    identity_index: u32,
) -> OneTimeClaimResume {
    use dpp::serialization::PlatformDeserializable;
    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;

    let st = match StateTransition::deserialize_from_bytes(&record.st_bytes) {
        Ok(st) => st,
        Err(e) => {
            warn!(
                error = %e,
                "one-time claim resume: stored transition failed to deserialize; dropping the \
                 record and rebuilding"
            );
            clear_one_time_claim_record(store, claim_records_id, record.activity_id).await;
            return OneTimeClaimResume::RecordUnusable;
        }
    };
    // Everything the resume acts on comes from HERE — the stored transition —
    // not from this call's arguments. See the fn docs.
    let (declared_id, stored_public_keys, stored_denomination) = match &st {
        StateTransition::IdentityCreateFromShieldedPool(t) => {
            let keys: BTreeMap<u32, IdentityPublicKey> = t
                .public_keys()
                .iter()
                .map(|key_in_creation| {
                    let key: IdentityPublicKey = key_in_creation.into();
                    (key.id(), key)
                })
                .collect();
            (t.identity_id(), keys, t.denomination())
        }
        other => {
            warn!(
                transition = %other.name(),
                "one-time claim resume: stored record does not carry a shielded identity-create \
                 transition; dropping the record and rebuilding"
            );
            clear_one_time_claim_record(store, claim_records_id, record.activity_id).await;
            return OneTimeClaimResume::RecordUnusable;
        }
    };
    let stored_master_key_hash = master_auth_public_key_hash_of(stored_public_keys.values());

    // Fail closed on any disagreement between what the caller asked for and
    // what the earlier attempt committed. Checked BEFORE the spent-nullifier
    // probe and the re-broadcast, so a mismatched retry costs nothing and
    // changes nothing — in particular the record survives for a correct retry.
    if let Some(mismatch) = one_time_claim_binding_mismatch(
        &stored_public_keys,
        stored_master_key_hash,
        stored_denomination,
        record.identity_index,
        &submitted_public_keys,
        master_key_hash,
        denomination,
        identity_index,
    ) {
        warn!(
            declared_id = %declared_id,
            mismatch,
            "one-time claim resume: retry arguments do not match the stored transition; refusing \
             to resume rather than mis-binding the original identity"
        );
        return OneTimeClaimResume::Resolved(Err(
            PlatformWalletError::ShieldedClaimBindingMismatch { mismatch },
        ));
    }

    // Past the gate the two agree, so the derived values are used from here on
    // — the transition is the source of truth by construction, and reading them
    // from it keeps that true even if the check above is ever relaxed.
    let master_key_hash = stored_master_key_hash;
    let submitted_public_keys = stored_public_keys;
    let denomination = stored_denomination;

    info!(
        declared_id = %declared_id,
        nullifiers = record.nullifiers.len(),
        "one-time claim: resuming from the persisted pending-claim record"
    );

    let status = nullifier_spent_status(sdk, &record.nullifiers).await;
    if status == NullifierSpentStatus::Spent {
        // The recorded claim (or a competitor) already consumed the notes.
        // The DECLARED id — unrecoverable without the record for a padded
        // bundle — lets the reconciler bind a created identity to this claim.
        return OneTimeClaimResume::Resolved(
            recover_executed_one_time_claim(
                sdk,
                master_key_hash,
                Some(declared_id),
                false,
                "resume: the recorded pending claim's notes are already spent on chain",
            )
            .await,
        );
    }

    // Unspent or Unknown: re-drive the byte-identical transition through the
    // same broadcast/confirm classification as a fresh claim. Byte-identical
    // re-broadcast is fund-safe (identical nullifiers cannot double-spend) and
    // preserves the recorded id.
    //
    // Same chargeable-step gate as the fresh-build path: a re-broadcast is a
    // resubmission that can execute and be charged for, so it may not proceed
    // on a lease the heartbeat can no longer prove
    // (#4313 review finding f58ed9d910d8). Aborting here is clean — the stored
    // record and its unspent notes are untouched, and the next retry resumes
    // exactly this record again.
    if let Err(e) =
        assert_claim_lease_before_chargeable_step(store, admission, "the claim re-broadcast").await
    {
        return OneTimeClaimResume::Resolved(Err(e));
    }

    let result = broadcast_and_confirm_one_time_claim(
        sdk,
        st,
        declared_id,
        Some(declared_id),
        master_key_hash,
        &record.nullifiers,
        submitted_public_keys,
        denomination,
    )
    .await;

    if status == NullifierSpentStatus::Unspent {
        if let Err(PlatformWalletError::ShieldedBroadcastFailed(reason)) = &result {
            // Definitive rejection of the STORED transition while its notes
            // are proven unconsumed (e.g. its anchor aged out of Platform's
            // recorded set): this record can never land. Clear it and build a
            // fresh claim in this same call.
            warn!(
                declared_id = %declared_id,
                reason,
                "one-time claim resume: stored transition is definitively rejected and its notes \
                 are unspent; dropping the record and rebuilding"
            );
            clear_one_time_claim_record(store, claim_records_id, record.activity_id).await;
            return OneTimeClaimResume::RecordUnusable;
        }
    }

    OneTimeClaimResume::Resolved(result)
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
        identity_index: None,
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

/// On-chain spent status of a claim's nullifier set, as far as a single
/// query can establish it.
///
/// The three states matter because callers draw OPPOSITE conclusions from
/// them: `Spent` proves the invitation notes are consumed (something
/// executed), `Unspent` proves nothing has consumed them yet, and
/// `Unknown` proves NOTHING — a transport failure or an absent response
/// must never be read as either of the other two (#4204 review finding
/// 8d020115b274).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NullifierSpentStatus {
    /// At least one queried nullifier is proof-verified spent.
    Spent,
    /// The query succeeded and covered every queried nullifier; none is spent.
    Unspent,
    /// The query failed, returned no response, or covered only part of the
    /// queried set — no conclusion can be drawn.
    Unknown,
}

/// Classify a successful nullifier-status response against the queried set.
///
/// A response that omits some queried nullifiers proves nothing about the
/// omitted ones, so it downgrades an all-unspent answer to `Unknown`.
fn classify_nullifier_statuses(
    statuses: &[dash_sdk::query_types::ShieldedNullifierStatus],
    queried: &[[u8; 32]],
) -> NullifierSpentStatus {
    if statuses.iter().any(|s| s.is_spent) {
        return NullifierSpentStatus::Spent;
    }
    let covered = queried
        .iter()
        .all(|q| statuses.iter().any(|s| &s.nullifier == q));
    if covered {
        NullifierSpentStatus::Unspent
    } else {
        NullifierSpentStatus::Unknown
    }
}

/// On-chain check: are `nullifiers` already recorded spent in Platform's
/// shielded nullifier set? Reuses the proof-verified
/// [`ShieldedNullifierStatuses`](dash_sdk::query_types::ShieldedNullifierStatuses)
/// fetch (query type [`ShieldedNullifiersQuery`](dash_sdk::query_types::ShieldedNullifiersQuery)).
///
/// A query error or an absent response is [`NullifierSpentStatus::Unknown`],
/// never `Unspent`: the pre-broadcast preflight may treat unknown as
/// "proceed" (the idempotent broadcast path reconciles via the
/// `NullifierAlreadySpent` verdict, so that only costs a harmless rebuild),
/// but the post-verdict classification must NOT — declaring a definitive
/// non-execution on an unknown status would report an applied chargeable
/// fallback as retryable.
async fn nullifier_spent_status(
    sdk: &Arc<dash_sdk::Sdk>,
    nullifiers: &[[u8; 32]],
) -> NullifierSpentStatus {
    use dash_sdk::platform::Fetch;
    use dash_sdk::query_types::{ShieldedNullifierStatuses, ShieldedNullifiersQuery};

    if nullifiers.is_empty() {
        return NullifierSpentStatus::Unspent;
    }
    match ShieldedNullifierStatuses::fetch(sdk, ShieldedNullifiersQuery(nullifiers.to_vec())).await
    {
        Ok(Some(statuses)) => classify_nullifier_statuses(&statuses.0, nullifiers),
        Ok(None) => NullifierSpentStatus::Unknown,
        Err(e) => {
            warn!(
                error = %e,
                "IdentityCreateFromOneTimeKey: nullifier spent-status query failed; status unknown"
            );
            NullifierSpentStatus::Unknown
        }
    }
}

/// The 20-byte hash of the MASTER authentication key among `public_keys`
/// (`purpose = AUTHENTICATION`, `security_level = MASTER`). This is the unique,
/// Platform-indexed key hash an identity can be looked up by — the exact probe
/// [`IdentityWallet::discover_inner`] scans with
/// (`Identity::fetch(sdk, PublicKeyHash(..))`). The invitee re-derives these
/// same creation keys from its own seed on a retry, so this hash re-derives
/// deterministically and needs no persisted record.
fn master_auth_public_key_hash(
    public_keys: &[(IdentityPublicKey, IdentityPublicKeyInCreation)],
) -> Option<[u8; 20]> {
    master_auth_public_key_hash_of(public_keys.iter().map(|(key, _)| key))
}

/// [`master_auth_public_key_hash`] over any borrowed key sequence — used by the
/// resume path, whose keys come out of the stored transition rather than out of
/// the caller's `(IdentityPublicKey, IdentityPublicKeyInCreation)` pairs.
fn master_auth_public_key_hash_of<'a>(
    public_keys: impl IntoIterator<Item = &'a IdentityPublicKey>,
) -> Option<[u8; 20]> {
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{Purpose, SecurityLevel};

    public_keys
        .into_iter()
        .find(|key| {
            key.purpose() == Purpose::AUTHENTICATION
                && key.security_level() == SecurityLevel::MASTER
        })
        .and_then(|key| key.public_key_hash().ok())
}

/// Positive evidence that `identity` was created by **this** claim's Type-20
/// transition.
///
/// Two independent bindings must BOTH hold. Each one alone is satisfied by a
/// real on-chain outcome in which this claim did *not* create the identity, so
/// neither is sufficient on its own:
///
/// 1. **Id binding** — `identity.id()` equals `expected_identity_id`, the id
///    derived from this claim's published spend nullifiers
///    (`identity_id_from_nullifiers`). Consensus re-derives the id the same way
///    and rejects any transition whose declared id differs (see
///    `derive_identity_id_from_actions` in the Type-20 state validation), so an
///    identity carrying this id can only have been created by a transition that
///    published exactly this claim's nullifier set.
///
///    Without it, the MASTER-key-hash lookup accepts the **pre-existing**
///    identity that a chargeable `UnshieldAction` fallback collided with: when a
///    submitted unique key hash is already registered, Type-20 finalizes the
///    spend as an `UnshieldTransitionAction` (`chargeable_failure: true`) and
///    creates no identity, yet the nullifier is consumed and the colliding
///    identity *is* findable under our own key hash.
///
/// 2. **Key binding** — the identity's **on-chain** key set contains this
///    claim's submitted MASTER authentication key hash.
///
///    Without it, the derived-id lookup accepts an identity created by a
///    *different* holder of the same bearer one-time key: the id is derived from
///    nullifiers only, never from identity keys, so two holders racing the same
///    invitation derive the same id under different keys.
///
/// The key binding is checked against the keys the fetch actually returned — an
/// identity that comes back without public keys fails closed rather than being
/// topped up with locally-submitted keys that were never proven to exist on
/// chain.
///
/// `expected_identity_id == None` means the id is not re-derivable for this
/// claim, so binding 1 cannot be established and this returns `false`. That is
/// the single-spend case: `BundleType::DEFAULT` pads a one-action bundle to
/// Orchard's 2-action minimum and the padding action's **randomly generated**
/// dummy nullifier participates in the id derivation, so a retry cannot
/// reproduce the original id.
fn recovered_identity_matches_claim(
    identity: &Identity,
    expected_identity_id: Option<Identifier>,
    master_key_hash: Option<[u8; 20]>,
) -> bool {
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::{Purpose, SecurityLevel};

    // Both handles must be available; a missing one is not evidence.
    let (Some(expected_id), Some(expected_hash)) = (expected_identity_id, master_key_hash) else {
        return false;
    };

    // Binding 1: the id must be the one derived from this claim's nullifiers.
    if identity.id() != expected_id {
        return false;
    }

    // Binding 2: the on-chain key set must carry this claim's MASTER auth key.
    identity.public_keys().values().any(|key| {
        key.purpose() == Purpose::AUTHENTICATION
            && key.security_level() == SecurityLevel::MASTER
            && key
                .public_key_hash()
                .is_ok_and(|hash| hash == expected_hash)
    })
}

/// Recover the identity a claim created by looking it up under its MASTER auth
/// key hash, with the same bounded retry cadence as
/// [`fetch_identity_with_retries`] to ride out DAPI indexing lag. Reuses
/// `discover_inner`'s unique-hash primitive (`Identity::fetch(sdk,
/// PublicKeyHash(..))`).
async fn fetch_identity_by_key_hash_with_retries(
    sdk: &Arc<dash_sdk::Sdk>,
    key_hash: [u8; 20],
) -> Option<Identity> {
    use dash_sdk::platform::types::identity::PublicKeyHash;
    use dash_sdk::platform::Fetch;

    for attempt in 0..IDENTITY_CREATE_FETCH_RETRIES {
        match Identity::fetch(sdk, PublicKeyHash(key_hash)).await {
            Ok(Some(identity)) => return Some(identity),
            Ok(None) => {
                trace!(
                    key_hash = %hex::encode(key_hash),
                    attempt,
                    "IdentityCreateFromOneTimeKey recovery: identity not found by key hash yet"
                );
            }
            Err(e) => {
                trace!(
                    key_hash = %hex::encode(key_hash),
                    attempt,
                    error = %e,
                    "IdentityCreateFromOneTimeKey recovery: key-hash lookup errored; will retry"
                );
            }
        }
        if attempt + 1 < IDENTITY_CREATE_FETCH_RETRIES {
            tokio::time::sleep(IDENTITY_CREATE_FETCH_RETRY_DELAY).await;
        }
    }
    None
}

/// This one-time-key claim's note is already spent on chain (the spent-nullifier
/// preflight saw it, or the broadcast/wait returned `NullifierAlreadySpent`).
/// Decide what that actually means and return the matching outcome.
///
/// A spent nullifier proves only that *something* consumed the invitation note —
/// **not** that this claim created an identity. Type-20 also consumes the note on
/// its chargeable `UnshieldAction` fallback, which creates no identity at all.
/// So every candidate identity found here must clear both ownership bindings in
/// [`recovered_identity_matches_claim`] before it can be reported as this
/// claim's result.
///
/// Two lookup handles are tried, each with bounded retries for DAPI indexing lag:
/// 1. the invitee's MASTER auth key hash (`discover_inner`'s unique-hash probe),
/// 2. the id derived from this claim's published nullifiers.
///
/// Outcomes:
/// - **`Ok`** — a fetched identity cleared both bindings: this claim created it.
/// - **[`PlatformWalletError::ShieldedInviteAlreadyClaimed`]** — an identity was
///   fetched but failed a binding (chargeable fallback, a competing holder of
///   the same bearer key, or — when `master_key_hash` is `None` — a key binding
///   that can never be established for this claim), *or* the id is not
///   re-derivable so no binding can ever be established. Terminal: the note is
///   spent, so retrying cannot help.
/// - **[`PlatformWalletError::ShieldedBroadcastUnconfirmed`]** — nothing resolved
///   yet, but the id *is* re-derivable, so a later retry can still reconcile once
///   indexing catches up. Only reachable when `expected_identity_id` is `Some`,
///   so the carried id is always the one this claim's nullifiers derive.
///
/// `spend_finalized` — the caller holds POSITIVE evidence that this claim's own
/// broadcast reached a definitive consensus verdict AND the notes are proven
/// consumed. Under that evidence, "no identity carries this claim's bindings"
/// is not indexing lag: an applied Type-20 that returned an error verdict
/// created no identity (the chargeable `UnshieldAction` fallback), and the
/// colliding unique key need not be MASTER — a collision on any other submitted
/// unique key leaves NOTHING findable under the MASTER-hash probe or the
/// derived id. The nothing-found outcome is then the terminal
/// `ShieldedInviteAlreadyClaimed`, not `ShieldedBroadcastUnconfirmed`
/// (#4204 review finding 8d020115b274).
async fn recover_executed_one_time_claim(
    sdk: &Arc<dash_sdk::Sdk>,
    master_key_hash: Option<[u8; 20]>,
    expected_identity_id: Option<Identifier>,
    spend_finalized: bool,
    evidence: &str,
) -> Result<(Identifier, Identity), PlatformWalletError> {
    warn!(
        ?expected_identity_id,
        evidence,
        "IdentityCreateFromOneTimeKey: invitation note already spent on chain; checking whether \
         this claim actually created an identity"
    );

    // The id is not re-derivable (single-spend bundle padded with a random dummy
    // nullifier), so no candidate identity can ever be bound to this claim.
    // Report the invitation as claimed rather than inventing a success.
    let Some(expected_id) = expected_identity_id else {
        return Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
            reason: format!(
                "the note was spent by an earlier transition whose identity id cannot be \
                 re-derived (single-spend bundles are padded with a randomly generated dummy \
                 nullifier that participates in the id derivation): {evidence}"
            ),
        });
    };

    // Handle 1: the invitee's own MASTER auth key hash.
    if let Some(key_hash) = master_key_hash {
        if let Some(identity) = fetch_identity_by_key_hash_with_retries(sdk, key_hash).await {
            if recovered_identity_matches_claim(&identity, expected_identity_id, master_key_hash) {
                info!(
                    identity_id = %identity.id(),
                    "IdentityCreateFromOneTimeKey: recovered this claim's identity by its master \
                     auth key hash (id and key bindings both verified)"
                );
                return Ok((identity.id(), identity));
            }
            // Found under our key hash but NOT created by this claim — the
            // chargeable-`UnshieldAction` outcome: the spend was finalized, the
            // value went to the fallback address, and this pre-existing identity
            // merely owns the colliding key hash.
            warn!(
                found_id = %identity.id(),
                expected_id = %expected_id,
                "IdentityCreateFromOneTimeKey: an identity owns this claim's master auth key hash \
                 but its id is not the one this claim's nullifiers derive; the spend was finalized \
                 as a chargeable failure and created no identity"
            );
            return Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
                reason: format!(
                    "identity {} owns the submitted master auth key hash but was not created by \
                     this claim (expected id {}); the shielded spend was finalized as a chargeable \
                     failure and its value went to the creation-failure address: {evidence}",
                    identity.id(),
                    expected_id
                ),
            });
        }
    }

    // Handle 2: the id derived from this claim's published nullifiers.
    if let Some(identity) = fetch_identity_with_retries(sdk, expected_id).await {
        if recovered_identity_matches_claim(&identity, expected_identity_id, master_key_hash) {
            info!(
                derived_id = %expected_id,
                "IdentityCreateFromOneTimeKey: recovered this claim's identity by its derived id \
                 (id and key bindings both verified)"
            );
            return Ok((identity.id(), identity));
        }
        // `recovered_identity_matches_claim` also fails closed when NO master
        // auth key hash was resolvable from the submitted keys (`master_key_hash
        // == None` — nothing was submitted, or `public_key_hash()` errored for
        // an unusual key type). The key binding can then never be established
        // for this claim, which is NOT evidence of a competing holder — report
        // the real cause. Terminal either way: the note is spent, and a retry
        // resubmits the same key set, so the hash stays unresolvable.
        if master_key_hash.is_none() {
            warn!(
                derived_id = %expected_id,
                "IdentityCreateFromOneTimeKey: an identity exists at this claim's derived id but \
                 this claim carries no resolvable master auth key hash, so ownership can be \
                 neither proven nor disproven"
            );
            return Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
                reason: format!(
                    "identity {expected_id} was created from this invitation's notes, but this \
                     claim submitted no resolvable master authentication key hash, so its \
                     ownership cannot be verified: {evidence}"
                ),
            });
        }
        // The id matches (same nullifier set) but the on-chain keys are not ours:
        // another holder of the same bearer one-time key won the race.
        warn!(
            derived_id = %expected_id,
            "IdentityCreateFromOneTimeKey: an identity exists at this claim's derived id but does \
             not carry the submitted master auth key; another holder of the same one-time key \
             claimed the invitation first"
        );
        return Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
            reason: format!(
                "identity {expected_id} was created from this invitation's notes but does not \
                 carry the submitted master authentication key, so it belongs to another holder \
                 of the one-time key: {evidence}"
            ),
        });
    }

    if spend_finalized {
        // Both probes came up empty under a definitive verdict + proven-spent
        // notes: the spend finalized without creating an identity that carries
        // this claim's bindings. That is the chargeable-`UnshieldAction`
        // fallback (the collision may have been on any submitted unique key,
        // not just MASTER) or a competing claim — terminal either way; the
        // value, if any, went to the creation-failure address.
        return Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
            reason: format!(
                "the claim's consensus verdict is definitive and the invitation notes are spent, \
                 but no identity carries this claim's bindings; the spend was finalized as a \
                 chargeable failure (or a competing claim) and created no identity for this \
                 wallet: {evidence}"
            ),
        });
    }

    Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
        identity_id: expected_id,
        reason: format!(
            "one-time-key claim executed (nullifier already spent) but the identity is not yet \
             resolvable by key hash or derived id: {evidence}"
        ),
    })
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
mod foreign_claim_guard_tests {
    use super::ForeignClaimGuards;
    use std::sync::Arc;

    /// Two callers with the same key must share ONE lifecycle mutex — that
    /// identity is what makes the claim single-flight (#4313 review finding
    /// 979bbc2fcb3c); different keys must not contend.
    #[test]
    fn same_key_shares_one_mutex_and_keys_are_independent() {
        let guards = ForeignClaimGuards::default();
        let a1 = guards.entry_for([1u8; 32]);
        let a2 = guards.entry_for([1u8; 32]);
        let b = guards.entry_for([2u8; 32]);
        assert!(
            Arc::ptr_eq(&a1, &a2),
            "same-key callers must receive the SAME lifecycle mutex"
        );
        assert!(
            !Arc::ptr_eq(&a1, &b),
            "distinct keys must receive distinct mutexes"
        );
    }

    /// The complete-lifecycle serialization: while one claim holds the
    /// guard (parked at an await, as across scan/proof/broadcast), a
    /// second same-key claim cannot enter; it proceeds only after the
    /// first releases — including release by CANCELLATION (future drop),
    /// so an abandoned claim can never wedge its invitation key.
    #[tokio::test]
    async fn same_key_claims_serialize_and_cancellation_releases() {
        let guards = Arc::new(ForeignClaimGuards::default());
        let key = [7u8; 32];

        let entry = guards.entry_for(key);
        let held = entry.lock().await;
        // Second same-key claim: must NOT be able to enter while held.
        let second = guards.entry_for(key);
        assert!(
            second.try_lock().is_err(),
            "a concurrent same-key claim must park while the lifecycle guard is held"
        );
        drop(held);
        assert!(
            second.try_lock().is_ok(),
            "the parked claim must proceed once the holder settles"
        );

        // Cancellation-safety: drop a future that acquired the guard at an
        // await point; the key must be immediately claimable again.
        let entry2 = guards.entry_for(key);
        let task = tokio::spawn(async move {
            let _g = entry2.lock().await;
            std::future::pending::<()>().await; // parked "mid-claim" forever
        });
        tokio::task::yield_now().await;
        task.abort();
        let _ = task.await;
        assert!(
            guards.entry_for(key).try_lock().is_ok(),
            "an aborted (cancelled) claim must release the key on drop"
        );
    }

    /// Abandoned keys cost nothing: once no claim holds a key's mutex, its
    /// registry row is pruned on the next acquisition, so hostile key churn
    /// cannot grow the map beyond the keys currently in flight.
    #[test]
    fn dead_entries_are_pruned() {
        let guards = ForeignClaimGuards::default();
        for i in 0..64u8 {
            let _ = guards.entry_for([i; 32]); // dropped immediately
        }
        let _live = guards.entry_for([0xFF; 32]);
        let len = guards
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        assert_eq!(
            len, 1,
            "only keys with a live claimant may occupy the registry"
        );
    }
}

#[cfg(test)]
mod shield_recipient_tests {
    use super::*;
    use crate::wallet::shielded::keys::OrchardKeySet;
    use dashcore::Network;

    fn keyset(seed_byte: u8) -> OrchardKeySet {
        OrchardKeySet::from_seed(&[seed_byte; 32], Network::Testnet, 0)
            .expect("ZIP-32 derivation from a fixed seed should succeed")
    }

    /// `None` = the self-shield: the default address, `Shield`/`In`,
    /// no counterparty — exactly what the pre-recipient path produced.
    #[test]
    fn no_recipient_resolves_to_the_default_address_as_shield_in() {
        let keys = keyset(0x42).viewing_keys();

        let resolved =
            resolve_shield_recipient(&keys, None).expect("self-shield must always resolve");

        assert_eq!(
            resolved.address.to_raw_bytes(),
            keys.default_address.to_raw_address_bytes(),
            "the self-shield note must go to the account's default address"
        );
        assert_eq!(resolved.counterparty, None);
        assert_eq!(resolved.kind, ShieldedActivityKind::Shield);
        assert_eq!(resolved.direction, ShieldedDirection::In);
    }

    /// A third-party address resolves to `Sent`/`Out` with the raw
    /// 43-byte address as counterparty — the classification the scan
    /// deriver produces for an OVK-recovered send to a non-own address,
    /// so live and restored rows agree.
    #[test]
    fn external_recipient_resolves_as_sent_out_with_raw_counterparty() {
        let keys = keyset(0x42).viewing_keys();
        let external = keyset(0x24).viewing_keys().default_address;

        let resolved = resolve_shield_recipient(&keys, Some(&external))
            .expect("a third-party recipient must resolve");

        assert_eq!(
            resolved.address.to_raw_bytes(),
            external.to_raw_address_bytes(),
            "the note must be built for the recipient's address"
        );
        assert_eq!(
            resolved.counterparty,
            Some(external.to_raw_address_bytes().to_vec()),
            "the activity row must carry the recipient as raw 43 bytes"
        );
        assert_eq!(resolved.kind, ShieldedActivityKind::Sent);
        assert_eq!(resolved.direction, ShieldedDirection::Out);
    }

    /// The account's own default address is not a third party: a
    /// `Sent`/`Out` live row for it would diverge from the self-pay row
    /// a restore's scan derives, so it must be rejected up front.
    #[test]
    fn own_default_address_as_recipient_is_rejected() {
        let keys = keyset(0x42).viewing_keys();
        let own = keys.default_address;

        let error = resolve_shield_recipient(&keys, Some(&own))
            .expect_err("the account's own address must be rejected");
        assert!(
            error
                .to_string()
                .contains("belongs to this shielded account"),
            "unexpected error: {error}"
        );
    }

    /// Orchard addresses are diversified, so ownership cannot be a
    /// fixed-address comparison: a non-default diversified index of the
    /// SAME account must also be recognized (via the IVK) and rejected.
    #[test]
    fn own_diversified_address_as_recipient_is_rejected() {
        let ks = keyset(0x42);
        let diversified = ks.address_at(7);
        let keys = ks.viewing_keys();
        assert_ne!(
            diversified.to_raw_address_bytes(),
            keys.default_address.to_raw_address_bytes(),
            "test needs a non-default diversified address"
        );

        resolve_shield_recipient(&keys, Some(&diversified))
            .expect_err("an own diversified address must be rejected");
    }
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
            identity_index: None,
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
mod nullifier_status_and_claim_record_tests {
    use super::*;
    use crate::wallet::shielded::file_store::FileBackedShieldedStore;
    use crate::wallet::shielded::store::InMemoryShieldedStore;
    use dash_sdk::query_types::ShieldedNullifierStatus;

    fn status(nullifier: [u8; 32], is_spent: bool) -> ShieldedNullifierStatus {
        ShieldedNullifierStatus {
            nullifier,
            is_spent,
        }
    }

    /// Any spent entry wins regardless of coverage: `Spent` is positive proof.
    #[test]
    fn classify_any_spent_is_spent() {
        let queried = [[1u8; 32], [2u8; 32]];
        let statuses = vec![status([1u8; 32], false), status([2u8; 32], true)];
        assert_eq!(
            classify_nullifier_statuses(&statuses, &queried),
            NullifierSpentStatus::Spent
        );
    }

    /// All queried nullifiers covered and none spent — proven unspent.
    #[test]
    fn classify_full_coverage_unspent_is_unspent() {
        let queried = [[1u8; 32], [2u8; 32]];
        let statuses = vec![status([1u8; 32], false), status([2u8; 32], false)];
        assert_eq!(
            classify_nullifier_statuses(&statuses, &queried),
            NullifierSpentStatus::Unspent
        );
    }

    /// A response that omits a queried nullifier proves nothing about it:
    /// partial coverage must NOT read as `Unspent` — that is the path that
    /// would misreport an applied chargeable fallback as a retryable
    /// non-execution (#4204 review finding 8d020115b274).
    #[test]
    fn classify_partial_coverage_is_unknown() {
        let queried = [[1u8; 32], [2u8; 32]];
        let statuses = vec![status([1u8; 32], false)];
        assert_eq!(
            classify_nullifier_statuses(&statuses, &queried),
            NullifierSpentStatus::Unknown
        );
        assert_eq!(
            classify_nullifier_statuses(&[], &queried),
            NullifierSpentStatus::Unknown
        );
    }

    /// The record key is deterministic per one-time key (a retry must find the
    /// record a crashed attempt armed) and distinct across keys.
    #[test]
    fn claim_record_key_is_deterministic_and_distinct() {
        use grovedb_commitment_tree::{FullViewingKey, SpendingKey};

        let fvk = |b: u8| {
            let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes([b; 32]))
                .expect("test byte pattern must be a valid spending key");
            FullViewingKey::from(&sk)
        };
        let a = fvk(1);
        let b = fvk(2);
        assert_eq!(one_time_claim_record_key(&a), one_time_claim_record_key(&a));
        assert_ne!(one_time_claim_record_key(&a), one_time_claim_record_key(&b));
    }

    /// Arm → find → clear round-trip through the reserved claim-records
    /// subwallet, and `finalize_one_time_claim_record`'s settlement rule:
    /// terminal `ShieldedInviteAlreadyClaimed` clears the record, while
    /// `ShieldedBroadcastUnconfirmed` — the outcome whose retry NEEDS the
    /// record — keeps it (#4204 review finding c0781f9d387f).
    #[tokio::test]
    async fn claim_record_round_trip_and_finalize_rules() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id = [7u8; 32];
        let id = SubwalletId::new(wallet_id, ONE_TIME_CLAIM_RECORDS_ACCOUNT);
        let key = [0xA5u8; 32];

        assert!(find_one_time_claim_record(&store, id, key)
            .await
            .expect("lookup must succeed")
            .is_none());

        {
            let mut guard = store.write().await;
            guard
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: key,
                        anchor: [9u8; 32],
                        nullifiers: vec![[3u8; 32]],
                        st_bytes: vec![1, 2, 3],
                        attempts: 0,
                        identity_index: None,
                    },
                )
                .expect("arm must succeed");
        }
        let found = find_one_time_claim_record(&store, id, key)
            .await
            .expect("lookup must succeed")
            .expect("armed record must be found");
        assert_eq!(found.nullifiers, vec![[3u8; 32]]);

        // Unconfirmed keeps the record — its retry needs the declared id.
        let unconfirmed: Result<(Identifier, Identity), PlatformWalletError> =
            Err(PlatformWalletError::ShieldedBroadcastUnconfirmed {
                identity_id: Identifier::new([1u8; 32]),
                reason: "test".to_string(),
            });
        finalize_one_time_claim_record(&store, id, key, &unconfirmed).await;
        assert!(find_one_time_claim_record(&store, id, key)
            .await
            .expect("lookup must succeed")
            .is_some());

        // Terminal AlreadyClaimed settles it.
        let terminal: Result<(Identifier, Identity), PlatformWalletError> =
            Err(PlatformWalletError::ShieldedInviteAlreadyClaimed {
                reason: "test".to_string(),
            });
        finalize_one_time_claim_record(&store, id, key, &terminal).await;
        assert!(find_one_time_claim_record(&store, id, key)
            .await
            .expect("lookup must succeed")
            .is_none());
    }

    /// A SUCCESSFUL claim must RETAIN its recovery record until the caller
    /// acknowledges durable local registration (#4313 review finding
    /// 325ce9fa8f84).
    ///
    /// It used to clear on success, which dropped the row before control
    /// returned through `PlatformWallet::identity_create_from_one_time_key` —
    /// where the identity is added to the manager and handed to the persister.
    /// A process death in that gap, a missing wallet-manager entry, or a
    /// `persister.store` failure therefore lost the exact padded identity ID
    /// before the JNI caller ever received it, and the retry could only return
    /// the terminal `ShieldedInviteAlreadyClaimed`.
    #[tokio::test]
    async fn a_successful_claim_retains_its_record_until_registration_is_acknowledged() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = SubwalletId::new([0x7Bu8; 32], ONE_TIME_CLAIM_RECORDS_ACCOUNT);
        let key = [0xB6u8; 32];

        store
            .write()
            .await
            .arm_redrive(
                id,
                PendingRedrive {
                    activity_id: key,
                    anchor: [9u8; 32],
                    nullifiers: vec![[3u8; 32]],
                    st_bytes: vec![1, 2, 3],
                    attempts: 0,
                    identity_index: Some(4),
                },
            )
            .expect("arm must succeed");

        let success: Result<(Identifier, Identity), PlatformWalletError> = Ok((
            Identifier::new([0xEE; 32]),
            Identity::default_versioned(dpp::version::PlatformVersion::latest())
                .expect("default identity"),
        ));

        // The claim resolves. This is the instant the native call returns to
        // the host — and the row must still be there.
        let receipt = finalize_one_time_claim_record(&store, id, key, &success).await;
        assert!(
            receipt.is_some(),
            "a successful claim must hand back a record for the caller to acknowledge"
        );
        assert!(
            find_one_time_claim_record(&store, id, key)
                .await
                .expect("lookup must succeed")
                .is_some(),
            "the recovery record must survive the native return — it is the only durable copy of \
             a padded single-note claim's identity id"
        );

        // Registration succeeded and is durable: only NOW is it released.
        acknowledge_one_time_claim_registration(&store, receipt.expect("receipt")).await;
        assert!(
            find_one_time_claim_record(&store, id, key)
                .await
                .expect("lookup must succeed")
                .is_none(),
            "the acknowledgement is what finally drops the record"
        );
    }

    /// The half that makes the retention worth having: a claim killed between
    /// its transition succeeding and the host durably registering the identity
    /// must be RECOVERABLE, not terminal (#4313 review finding 325ce9fa8f84).
    ///
    /// The claim resolves, the record is retained, and then the process dies
    /// before acknowledging — modelled by dropping the receipt and reopening
    /// the store from disk, which is exactly what a relaunch does. The retry
    /// then finds the record, and with it the DECLARED identity id that a
    /// padded single-note bundle cannot re-derive. That id is what
    /// `recover_executed_one_time_claim` needs; without it, the retry's
    /// `expected_identity_id` is `None` and the spent-note path returns the
    /// terminal `ShieldedInviteAlreadyClaimed` before the master-key lookup is
    /// ever attempted.
    #[tokio::test]
    async fn a_claim_killed_before_acknowledgement_is_recoverable_after_a_restart() {
        use dpp::serialization::PlatformSerializable;

        let path = std::env::temp_dir().join(format!(
            "shielded_claim_ack_{}_{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let wallet_id = [0x9Cu8; 32];
        let id = SubwalletId::new(wallet_id, ONE_TIME_CLAIM_RECORDS_ACCOUNT);
        let key = [0xC7u8; 32];

        // A claim whose transition carries a padded, non-re-derivable id: the
        // single-spend case, where the record is the only handle to it.
        let declared_id = Identifier::new([0x2B; 32]);
        let st_bytes = {
            use dpp::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
            use dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;

            let transition: IdentityCreateFromShieldedPoolTransition =
                IdentityCreateFromShieldedPoolTransitionV0 {
                    public_keys: Vec::new(),
                    denomination: 10_000_000_000,
                    actions: Vec::new(),
                    anchor: [0x07; 32],
                    proof: vec![0x08; 8],
                    binding_signature: [0x09; 64],
                    send_to_address_on_creation_failure: dpp::address_funds::PlatformAddress::P2pkh(
                        [0u8; 20],
                    ),
                    identity_id: declared_id,
                }
                .into();
            StateTransition::IdentityCreateFromShieldedPool(transition)
                .serialize_to_bytes()
                .expect("serialize")
        };

        {
            let store: Arc<RwLock<FileBackedShieldedStore>> = Arc::new(RwLock::new(
                FileBackedShieldedStore::open_path(&path, 100).expect("store opens"),
            ));
            store
                .write()
                .await
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: key,
                        anchor: [0x0A; 32],
                        nullifiers: vec![[0x0F; 32]],
                        st_bytes,
                        attempts: 0,
                        identity_index: Some(4),
                    },
                )
                .expect("arm must succeed");

            let success: Result<(Identifier, Identity), PlatformWalletError> = Ok((
                declared_id,
                Identity::default_versioned(dpp::version::PlatformVersion::latest())
                    .expect("default identity"),
            ));
            let receipt = finalize_one_time_claim_record(&store, id, key, &success).await;
            assert!(receipt.is_some(), "success yields a receipt");
            // The process dies HERE: the receipt is dropped without ever being
            // acknowledged, so the identity never reached the host's durable
            // state.
            let _dropped_without_acknowledging = receipt;
        }

        // Relaunch: a fresh store over the same file, as `open_path` does on
        // every app start.
        let reopened = FileBackedShieldedStore::open_path(&path, 100).expect("store reopens");
        let rehydrated = reopened.pending_redrives(id).expect("records readable");
        assert_eq!(
            rehydrated.len(),
            1,
            "RED before the fix: the success path cleared the row, so the relaunch found nothing \
             and the retry could only report the invitation as already claimed"
        );

        // And the row still carries the declared id — the value the retry needs
        // and cannot otherwise reconstruct.
        let recovered_id = {
            use dpp::serialization::PlatformDeserializable;
            use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
            match StateTransition::deserialize_from_bytes(&rehydrated[0].st_bytes)
                .expect("stored transition deserializes")
            {
                StateTransition::IdentityCreateFromShieldedPool(t) => t.identity_id(),
                other => panic!("unexpected stored transition {}", other.name()),
            }
        };
        assert_eq!(
            recovered_id, declared_id,
            "the retained record is what lets the retry bind a recovered identity to this claim \
             instead of returning terminal ShieldedInviteAlreadyClaimed"
        );

        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    /// A corrupt stored transition must not wedge the claim: the resume path
    /// drops the record (so the fresh build proceeds) without touching the
    /// network (the mock SDK has no expectations — any fetch would error).
    #[tokio::test]
    async fn resume_drops_corrupt_record_and_rebuilds() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id = [8u8; 32];
        let id = SubwalletId::new(wallet_id, ONE_TIME_CLAIM_RECORDS_ACCOUNT);
        let key = [0x5Au8; 32];
        let record = PendingRedrive {
            activity_id: key,
            anchor: [0u8; 32],
            nullifiers: vec![[4u8; 32]],
            st_bytes: vec![0xDE, 0xAD], // never deserializes
            attempts: 0,
            identity_index: None,
        };
        store
            .write()
            .await
            .arm_redrive(id, record.clone())
            .expect("arm must succeed");

        let outcome = resume_one_time_claim(
            &sdk,
            &store,
            id,
            &record,
            // The corrupt record is rejected long before the
            // pre-broadcast lease gate, so any token reaches the assertion.
            crate::wallet::shielded::store::AdmissionToken::generate().expect("token"),
            None,
            BTreeMap::new(),
            100_000,
            0,
        )
        .await;
        assert!(matches!(outcome, OneTimeClaimResume::RecordUnusable));
        assert!(find_one_time_claim_record(&store, id, key)
            .await
            .expect("lookup must succeed")
            .is_none());
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
            PlatformWalletError::PlatformShieldCapacityExceeded { available, required }
                if *available == 3_623_849_220 && *required == 3_623_849_221
        ));
        assert_eq!(
            mapped.to_string(),
            "Platform shield capacity exceeded: available 3623849220, required 3623849221"
        );
    }
}

#[cfg(test)]
mod reserve_shield_fee_tests {
    use super::*;
    use dpp::version::LATEST_PLATFORM_VERSION;

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
    fn versioned_fee_keeps_input_zero_valid_and_reserve_tracks_the_fee() {
        let min_input_amount = LATEST_PLATFORM_VERSION
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        let shield_fee = compute_minimum_shielded_fee(SHIELD_NUM_ACTIONS, LATEST_PLATFORM_VERSION)
            .expect("latest shield fee must be computable");
        let reserve = shield_fee_reserve_credits(LATEST_PLATFORM_VERSION)
            .expect("latest shield fee reserve must be computable");
        let smallest_fee_inclusive_claim = shield_fee
            .checked_add(1)
            .expect("latest shield fee plus one credit must fit");

        assert!(
            smallest_fee_inclusive_claim >= min_input_amount,
            "adding the fee must lift even input 0's smallest positive base claim above the protocol minimum"
        );
        assert!(
            reserve >= shield_fee,
            "the retained input-0 headroom must cover the versioned shield fee"
        );
        assert!(
            reserve <= shield_fee.saturating_mul(4),
            "the reserve must stay a small multiple of the versioned fee — an oversized \
             reserve silently understates preflight capacity and strands the excess \
             below the input-0 viability threshold after a Max shield"
        );
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

/// Unit tests for the ONE-TIME-key claim path
/// ([`identity_create_from_one_time_key`] / [`super::sync::scan_notes_for_foreign_key`]).
///
/// The full op needs a live SDK note stream, so these cover the network-free
/// pieces the crate ADDS: deriving a note owned by a foreign one-time spending
/// key (the scan's per-note conversion — value / cmx / nullifier / serialization),
/// the exact-equality selection over the transiently-scanned set (exact / over /
/// under / no-note), and witnessing that foreign note against a Platform-recorded
/// anchor in the shared marked tree. The key-agnostic Type-20 BUILD with a
/// foreign key is proven by rs-dpp's own green builder tests
/// (`SpendingKey::from_bytes([..]) → fvk/ask → build … succeeds`).
#[cfg(test)]
mod one_time_key_tests {
    use super::*;
    use crate::wallet::shielded::file_store::FileBackedShieldedStore;
    use dpp::version::PlatformVersion;
    use grovedb_commitment_tree::{
        ExtractedNoteCommitment, FullViewingKey, Note, NoteValue, RandomSeed, Rho, Scope,
        SpendingKey,
    };

    /// Smallest member of the versioned exit-denomination set (0.1 DASH).
    const DENOMINATION: u64 = 10_000_000_000;

    /// A fixed, valid one-time Orchard spending key for the tests.
    const ONE_TIME_SK: [u8; 32] = [0x24; 32];

    fn temp_tree_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("one_time_key_{tag}_{nanos}.sqlite"))
    }

    fn filler_cmx(b: u8) -> [u8; 32] {
        let mut c = [0u8; 32];
        c[0] = b;
        c
    }

    /// The full-viewing key of the one-time spending key.
    fn one_time_fvk() -> FullViewingKey {
        let sk: SpendingKey = Option::from(SpendingKey::from_bytes(ONE_TIME_SK))
            .expect("fixed one-time SK is a valid Orchard SpendingKey");
        FullViewingKey::from(&sk)
    }

    /// Build one real Orchard note OWNED BY the one-time key, shaped exactly as
    /// [`super::sync::scan_notes_for_foreign_key`] would produce it: `cmx` is the
    /// note's real commitment, `nullifier` is derived under the one-time key's
    /// fvk, and `note_data` is the canonical 115-byte serialization.
    fn one_time_note(value: u64, position: u64) -> ShieldedNote {
        let fvk = one_time_fvk();
        let recipient = fvk.address_at(0u32, Scope::External);

        // rho / rseed must be canonical Pallas base-field elements — scan
        // deterministically (mirrors the existing note builders in this file).
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

        let note = Note::from_parts(recipient, NoteValue::from_raw(value), rho, rseed)
            .into_option()
            .expect("valid note parts");
        let cmx = ExtractedNoteCommitment::from(note.commitment()).to_bytes();
        let nullifier = note.nullifier(&fvk).to_bytes();

        let mut note_data = Vec::with_capacity(115);
        note_data.extend_from_slice(&note.recipient().to_raw_address_bytes());
        note_data.extend_from_slice(&note.value().inner().to_le_bytes());
        note_data.extend_from_slice(&note.rho().to_bytes());
        note_data.extend_from_slice(note.rseed().as_bytes());

        ShieldedNote {
            position,
            cmx,
            nullifier,
            block_height: 1,
            is_spent: false,
            value,
            note_data,
        }
    }

    /// The scan's per-note conversion is correct: a note owned by the one-time
    /// key round-trips through the wallet's 115-byte serialization, and its
    /// nullifier matches the one derived under that key's fvk (what the scan
    /// stamps). This is the piece [`super::sync::scan_notes_for_foreign_key`]
    /// runs on every discovered note.
    #[test]
    fn foreign_key_note_roundtrips_and_nullifier_matches() {
        let note = one_time_note(DENOMINATION, 0);

        // `note_data` deserializes back to an equal note.
        let decoded = deserialize_note(&note.note_data).expect("serialized note is valid");
        assert_eq!(
            decoded.value().inner(),
            DENOMINATION,
            "value survives round-trip"
        );

        // The stamped nullifier is exactly the one the one-time key's fvk derives.
        let fvk = one_time_fvk();
        assert_eq!(
            note.nullifier,
            decoded.nullifier(&fvk).to_bytes(),
            "stamped nullifier must match the fvk-derived nullifier"
        );

        // The stored cmx is the note's real extracted commitment.
        assert_eq!(
            note.cmx,
            ExtractedNoteCommitment::from(decoded.commitment()).to_bytes(),
            "stored cmx must be the note's real commitment"
        );
    }

    /// Exact-equality selection over the transiently-scanned set: exact funding
    /// (zero change), over-funding (change = excess routed to change_address),
    /// under-funding (typed `ShieldedInsufficientBalance`), and no-note (empty →
    /// `ShieldedNoUnspentNotes`, the op's fail-fast on an unfunded key).
    #[test]
    fn select_for_claim_exact_over_under_and_no_note() {
        let version = PlatformVersion::latest();

        // Exact: one note equal to the denomination → zero change.
        let exact = vec![one_time_note(DENOMINATION, 0)];
        let (sel, total, fee) =
            select_notes_for_denomination(&exact, DENOMINATION, 2, 1, version).expect("exact");
        assert_eq!(sel.len(), 1);
        assert_eq!(total, DENOMINATION);
        assert_eq!(total - DENOMINATION, 0, "exact funding leaves zero change");
        assert!(fee < DENOMINATION, "fee must leave a positive balance");

        // Over-funded: the excess above the denomination becomes the change note.
        let excess = 7_000_000_000u64;
        let over = vec![one_time_note(DENOMINATION + excess, 0)];
        let (sel, total, _) =
            select_notes_for_denomination(&over, DENOMINATION, 2, 1, version).expect("over");
        assert_eq!(sel.len(), 1);
        assert_eq!(
            total - DENOMINATION,
            excess,
            "over-funding routes the excess to change_address"
        );

        // Under-funded: a single note below the denomination.
        let under = vec![one_time_note(DENOMINATION - 1, 0)];
        match select_notes_for_denomination(&under, DENOMINATION, 2, 1, version) {
            Err(PlatformWalletError::ShieldedInsufficientBalance {
                available,
                required,
            }) => {
                assert_eq!(available, DENOMINATION - 1);
                assert_eq!(required, DENOMINATION);
            }
            other => panic!("expected ShieldedInsufficientBalance, got {other:?}"),
        }

        // No note found for the key: empty set → ShieldedNoUnspentNotes (the same
        // error the op raises on `discovered.is_empty()`).
        match select_notes_for_denomination(&[], DENOMINATION, 2, 1, version) {
            Err(PlatformWalletError::ShieldedNoUnspentNotes) => {}
            other => panic!("expected ShieldedNoUnspentNotes, got {other:?}"),
        }
    }

    /// The witness half: a note owned by the one-time key, appended to the shared
    /// fully-marked tree, is witnessable and produces a `SpendableNote` against a
    /// Platform-recorded anchor — the same probe the op runs before the build.
    #[test]
    fn foreign_key_note_witnesses_against_recorded_anchor() {
        let path = temp_tree_path("witness");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let note = one_time_note(DENOMINATION, 0);

        // One block, checkpointed on its boundary: depth-0 root is recorded.
        store.append_commitment(&note.cmx, true).unwrap();
        store.append_commitment(&filler_cmx(0xA1), true).unwrap();
        store.append_commitment(&filler_cmx(0xA2), true).unwrap();
        store.checkpoint_tree(3).unwrap();
        let root_depth0 = store.tree_anchor().unwrap();

        let recorded: HashSet<[u8; 32]> = [root_depth0].into_iter().collect();

        let (spends, anchor) =
            select_recorded_spends(&store, std::slice::from_ref(&note), &recorded)
                .expect("the one-time key's note witnesses against the recorded anchor");

        let _ = std::fs::remove_file(&path);

        assert_eq!(
            spends.len(),
            1,
            "the one-time key's single note is spendable"
        );
        assert_eq!(
            spends[0].note.value().inner(),
            DENOMINATION,
            "the witnessed SpendableNote carries the funded value"
        );
        assert_eq!(
            anchor.to_bytes(),
            root_depth0,
            "the spend is built against the Platform-recorded anchor"
        );
    }
}

/// Regression tests for one-time-key (shielded invitation) claim RECOVERY
/// ownership evidence.
///
/// A spent invitation nullifier proves only that *something* consumed the note.
/// It does **not** prove that this claim's Type-20 transition created an
/// identity, and these tests pin the two on-chain outcomes where the pre-fix
/// rule — "the nullifier is spent and an identity is findable under the
/// submitted MASTER auth key hash" — reported a successful claim that never
/// happened.
#[cfg(test)]
mod one_time_claim_evidence_tests {
    use super::*;
    use crate::wallet::shielded::file_store::FileBackedShieldedStore;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::version::PlatformVersion;

    /// This claim's submitted MASTER auth key hash.
    const OUR_MASTER_HASH: [u8; 20] = [0xA1; 20];
    /// Some other key's hash — used for the competing-claimant identity.
    const OTHER_MASTER_HASH: [u8; 20] = [0xB2; 20];
    /// Smallest member of the versioned exit-denomination set (0.1 DASH).
    const DENOMINATION: u64 = 10_000_000_000;
    /// The local DIP-9 slot the original attempt registered at.
    const IDENTITY_INDEX: u32 = 3;

    fn temp_store_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("one_time_claim_{tag}_{nanos}.sqlite"))
    }

    /// The two real note nullifiers this claim spends.
    fn our_nullifiers() -> Vec<[u8; 32]> {
        vec![[0x11; 32], [0x22; 32]]
    }

    /// An `ECDSA_HASH160` key whose `public_key_hash()` is exactly `hash` —
    /// `KeyType::ECDSA_HASH160` returns its 20-byte `data` verbatim, so the test
    /// controls the hash precisely without generating real key material.
    fn key_with_hash(
        id: u32,
        purpose: Purpose,
        security_level: SecurityLevel,
        hash: [u8; 20],
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(hash.to_vec()),
            disabled_at: None,
        })
    }

    fn identity_with_keys(id: Identifier, keys: Vec<IdentityPublicKey>) -> Identity {
        let map: BTreeMap<u32, IdentityPublicKey> = keys.into_iter().map(|k| (k.id(), k)).collect();
        Identity::new_with_id_and_keys(id, map, PlatformVersion::latest())
            .expect("test identity builds")
    }

    /// The MASTER auth key this claim submits.
    fn our_master_key() -> IdentityPublicKey {
        key_with_hash(
            0,
            Purpose::AUTHENTICATION,
            SecurityLevel::MASTER,
            OUR_MASTER_HASH,
        )
    }

    /// The **pre-fix** acceptance rule, encoded here as the behavior these tests
    /// exist to reject.
    ///
    /// Before the fix, both recovery handles returned `Ok((identity.id(),
    /// identity))` for *whatever* identity the lookup produced — the fetched
    /// identity was never inspected. So the old rule accepted unconditionally
    /// once a lookup succeeded, and every case below that asserts
    /// `recovered_identity_matches_claim(..) == false` is a case the old code
    /// returned as a successful claim.
    fn pre_fix_rule_accepts(_identity: &Identity) -> bool {
        true
    }

    /// BLOCKER 1 — chargeable `UnshieldAction` fallback must not read as success.
    ///
    /// When a submitted unique public-key hash is already registered, Type-20
    /// finalizes the shielded spend as an `UnshieldTransitionAction` with
    /// `chargeable_failure: true`: the nullifier IS consumed, the invitation
    /// value goes to the creation-failure address, and **no identity is
    /// created**. A retry then finds the *pre-existing* identity that owns the
    /// colliding key hash. Its id is not the one this claim's nullifiers derive,
    /// so the id binding must reject it.
    #[test]
    fn chargeable_unshield_fallback_identity_is_rejected() {
        let expected_id = identity_id_from_nullifiers(&our_nullifiers());

        // The pre-existing identity: it genuinely owns our MASTER key hash (that
        // is exactly why the unique-key-hash collision fired), but it was created
        // by some unrelated earlier transition, so it carries an unrelated id.
        let pre_existing = identity_with_keys(Identifier::from([0xEE; 32]), vec![our_master_key()]);

        assert!(
            pre_existing.id() != expected_id,
            "precondition: the colliding identity is not the one this claim derives"
        );
        assert!(
            pre_fix_rule_accepts(&pre_existing),
            "the pre-fix rule accepted this identity as a successful claim"
        );
        assert!(
            !recovered_identity_matches_claim(
                &pre_existing,
                Some(expected_id),
                Some(OUR_MASTER_HASH)
            ),
            "an identity that merely owns the submitted master auth key hash must NOT be \
             reported as this claim's result: the spend was finalized as a chargeable failure \
             and created no identity"
        );
    }

    /// BLOCKER 2 — a competing holder of the same bearer key must not read as
    /// success.
    ///
    /// The identity id is derived from published nullifiers only, never from
    /// identity keys. With two or more real spends no randomized padding action
    /// is added, so another holder of the same one-time key spending the same
    /// notes derives the SAME id under THEIR keys. The key binding must reject
    /// it — otherwise the foreign identity is registered at this wallet's
    /// caller-supplied identity index.
    #[test]
    fn competing_bearer_key_holder_identity_is_rejected() {
        let expected_id = identity_id_from_nullifiers(&our_nullifiers());

        // Same notes => same nullifiers => same derived id, but the winner
        // registered their own master key.
        let foreign = identity_with_keys(
            expected_id,
            vec![key_with_hash(
                0,
                Purpose::AUTHENTICATION,
                SecurityLevel::MASTER,
                OTHER_MASTER_HASH,
            )],
        );

        assert_eq!(
            foreign.id(),
            expected_id,
            "precondition: the race winner's identity shares this claim's derived id"
        );
        assert!(
            pre_fix_rule_accepts(&foreign),
            "the pre-fix rule accepted this identity as a successful claim"
        );
        assert!(
            !recovered_identity_matches_claim(&foreign, Some(expected_id), Some(OUR_MASTER_HASH)),
            "an identity at this claim's derived id that does not carry the submitted master \
             auth key belongs to another holder of the one-time key and must NOT be returned"
        );
    }

    /// A keyless fetch must fail closed rather than be topped up with the
    /// locally-submitted keys — those were never proven to exist on chain.
    #[test]
    fn identity_fetched_without_public_keys_is_rejected() {
        let expected_id = identity_id_from_nullifiers(&our_nullifiers());
        let keyless = identity_with_keys(expected_id, vec![]);

        assert!(
            pre_fix_rule_accepts(&keyless),
            "the pre-fix rule accepted this identity and then inserted the submitted keys locally"
        );
        assert!(
            !recovered_identity_matches_claim(&keyless, Some(expected_id), Some(OUR_MASTER_HASH)),
            "an identity fetched without public keys cannot prove the key binding"
        );
    }

    /// A single-spend claim's id is not re-derivable (the bundle is padded to
    /// Orchard's 2-action minimum with a randomly generated dummy nullifier that
    /// participates in the derivation), so no candidate can ever be bound to it.
    #[test]
    fn unre_derivable_id_is_rejected() {
        let identity = identity_with_keys(Identifier::from([0xEE; 32]), vec![our_master_key()]);

        assert!(
            !recovered_identity_matches_claim(&identity, None, Some(OUR_MASTER_HASH)),
            "without a re-derivable id there is no evidence this claim created the identity"
        );
    }

    /// A missing MASTER auth key hash is not evidence either.
    #[test]
    fn absent_master_key_hash_is_rejected() {
        let expected_id = identity_id_from_nullifiers(&our_nullifiers());
        let identity = identity_with_keys(expected_id, vec![our_master_key()]);

        assert!(
            !recovered_identity_matches_claim(&identity, Some(expected_id), None),
            "without a submitted master auth key hash the key binding cannot be established"
        );
    }

    /// A key with the right hash but the wrong purpose/security level does not
    /// satisfy the key binding — the binding is specifically on the MASTER
    /// AUTHENTICATION key, which is the uniquely Platform-indexed handle.
    #[test]
    fn non_master_key_with_matching_hash_is_rejected() {
        let expected_id = identity_id_from_nullifiers(&our_nullifiers());
        let identity = identity_with_keys(
            expected_id,
            vec![
                key_with_hash(
                    0,
                    Purpose::AUTHENTICATION,
                    SecurityLevel::HIGH,
                    OUR_MASTER_HASH,
                ),
                key_with_hash(
                    1,
                    Purpose::TRANSFER,
                    SecurityLevel::CRITICAL,
                    OUR_MASTER_HASH,
                ),
            ],
        );

        assert!(
            !recovered_identity_matches_claim(&identity, Some(expected_id), Some(OUR_MASTER_HASH)),
            "only a MASTER AUTHENTICATION key satisfies the key binding"
        );
    }

    /// The positive case: both bindings hold, so this claim provably created the
    /// identity and recovery returns it.
    #[test]
    fn identity_with_matching_id_and_master_key_is_accepted() {
        let expected_id = identity_id_from_nullifiers(&our_nullifiers());
        let ours = identity_with_keys(
            expected_id,
            vec![
                our_master_key(),
                key_with_hash(1, Purpose::TRANSFER, SecurityLevel::CRITICAL, [0xC3; 20]),
            ],
        );

        assert!(
            recovered_identity_matches_claim(&ours, Some(expected_id), Some(OUR_MASTER_HASH)),
            "an identity carrying this claim's derived id AND its submitted master auth key was \
             created by this claim"
        );
    }

    /// The id binding is only meaningful because the derivation is over the
    /// claim's own nullifier set: a different note selection derives a different
    /// id, so it cannot be passed off as this claim's result.
    #[test]
    fn a_different_nullifier_set_derives_a_different_id() {
        let ours = identity_id_from_nullifiers(&our_nullifiers());
        let theirs = identity_id_from_nullifiers(&[[0x11; 32], [0x33; 32]]);

        assert_ne!(
            ours, theirs,
            "the derived id is a function of the published nullifier set"
        );

        let identity = identity_with_keys(theirs, vec![our_master_key()]);
        assert!(
            !recovered_identity_matches_claim(&identity, Some(ours), Some(OUR_MASTER_HASH)),
            "an identity created from a different nullifier set is not this claim's identity"
        );
    }

    // ── Resumed-claim binding (#4313 review finding 195efdd4ae21) ──────────
    //
    // The pending-claim record is found by wallet id and one-time FVK alone, so
    // the resume must take its binding from the STORED TRANSITION and refuse a
    // retry whose arguments disagree — never act on the caller's values while
    // re-broadcasting someone else's bytes.

    /// Serialize a shielded identity-create transition carrying exactly `keys`
    /// and `denomination`, shaped as `arm_one_time_claim_record` stores it.
    fn stored_claim_transition(
        keys: &[IdentityPublicKey],
        denomination: u64,
    ) -> (StateTransition, Vec<u8>) {
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
        use dpp::state_transition::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;

        let transition: IdentityCreateFromShieldedPoolTransition =
            IdentityCreateFromShieldedPoolTransitionV0 {
                public_keys: keys
                    .iter()
                    .map(|key| IdentityPublicKeyInCreation::from(key.clone()))
                    .collect(),
                denomination,
                actions: Vec::new(),
                anchor: [0x07; 32],
                proof: vec![0x08; 8],
                binding_signature: [0x09; 64],
                send_to_address_on_creation_failure: dpp::address_funds::PlatformAddress::P2pkh(
                    [0u8; 20],
                ),
                identity_id: identity_id_from_nullifiers(&our_nullifiers()),
            }
            .into();
        let st = StateTransition::IdentityCreateFromShieldedPool(transition);
        let bytes = st.serialize_to_bytes().expect("transition serializes");
        (st, bytes)
    }

    /// A pending-claim record over `st_bytes`, keyed like a real one.
    fn stored_claim_record(st_bytes: Vec<u8>) -> PendingRedrive {
        PendingRedrive {
            activity_id: [0x5A; 32],
            anchor: [0x07; 32],
            nullifiers: our_nullifiers(),
            st_bytes,
            attempts: 0,
            identity_index: Some(IDENTITY_INDEX),
        }
    }

    fn keys_map(keys: &[IdentityPublicKey]) -> BTreeMap<u32, IdentityPublicKey> {
        keys.iter().map(|key| (key.id(), key.clone())).collect()
    }

    /// A second key set that differs from `our_master_key()` only in the key
    /// MATERIAL — same id, purpose and security level. This is the dangerous
    /// shape: ids alone still line up, so anything comparing only ids would
    /// wave it through and register a foreign identity at this wallet's slot.
    fn other_master_key() -> IdentityPublicKey {
        key_with_hash(
            0,
            Purpose::AUTHENTICATION,
            SecurityLevel::MASTER,
            OTHER_MASTER_HASH,
        )
    }

    /// THE DERIVE PATH: everything the resume needs is recoverable from the
    /// serialized transition, which is why no record-schema change is required.
    /// A round-trip through `StateTransition` must reproduce the exact key set
    /// (by id AND content), the denomination, and the MASTER auth key hash that
    /// idempotent recovery probes Platform with.
    #[test]
    fn claim_binding_is_recoverable_from_the_stored_transition() {
        use dpp::serialization::PlatformDeserializable;
        use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;

        let submitted = vec![
            our_master_key(),
            key_with_hash(1, Purpose::TRANSFER, SecurityLevel::CRITICAL, [0xC3; 20]),
        ];
        let (_, st_bytes) = stored_claim_transition(&submitted, DENOMINATION);

        let restored =
            StateTransition::deserialize_from_bytes(&st_bytes).expect("stored bytes deserialize");
        let StateTransition::IdentityCreateFromShieldedPool(transition) = &restored else {
            panic!("stored record must carry a shielded identity-create transition");
        };

        let derived: BTreeMap<u32, IdentityPublicKey> = transition
            .public_keys()
            .iter()
            .map(|key_in_creation| {
                let key: IdentityPublicKey = key_in_creation.into();
                (key.id(), key)
            })
            .collect();

        assert_eq!(
            derived,
            keys_map(&submitted),
            "the submitted key set must be recoverable from the transition itself"
        );
        assert_eq!(transition.denomination(), DENOMINATION);
        assert_eq!(
            master_auth_public_key_hash_of(derived.values()),
            Some(OUR_MASTER_HASH),
            "the recovery handle must be derivable from the transition, not supplied by the retry"
        );
    }

    /// MATCHING ARGS: a retry presenting exactly what the earlier attempt
    /// submitted is a genuine resume and must pass the binding gate.
    #[test]
    fn matching_retry_arguments_resume() {
        let submitted = vec![our_master_key()];
        let keys = keys_map(&submitted);

        assert_eq!(
            one_time_claim_binding_mismatch(
                &keys,
                Some(OUR_MASTER_HASH),
                DENOMINATION,
                Some(IDENTITY_INDEX),
                &keys,
                Some(OUR_MASTER_HASH),
                DENOMINATION,
                IDENTITY_INDEX,
            ),
            None,
            "identical arguments must not be treated as a mis-binding"
        );
    }

    /// Key ORDER is not a mismatch: both sides are keyed by key id, so a caller
    /// that assembles the same keys in a different order still resumes.
    #[test]
    fn key_order_is_not_a_binding_mismatch() {
        let forward = keys_map(&[
            our_master_key(),
            key_with_hash(1, Purpose::TRANSFER, SecurityLevel::CRITICAL, [0xC3; 20]),
        ]);
        let reversed = keys_map(&[
            key_with_hash(1, Purpose::TRANSFER, SecurityLevel::CRITICAL, [0xC3; 20]),
            our_master_key(),
        ]);

        assert_eq!(
            one_time_claim_binding_mismatch(
                &forward,
                Some(OUR_MASTER_HASH),
                DENOMINATION,
                Some(IDENTITY_INDEX),
                &reversed,
                Some(OUR_MASTER_HASH),
                DENOMINATION,
                IDENTITY_INDEX,
            ),
            None
        );
    }

    /// MISMATCHED ARGS, per field. Each of these is a way the pre-fix resume
    /// would have acted on the caller's value while broadcasting the stored
    /// bytes: a swapped key set registers a foreign identity at this wallet's
    /// slot and backfills an empty proof result with keys that were never in the
    /// transition; a swapped master hash makes idempotent recovery probe
    /// Platform for someone else's identity; a swapped denomination misreports
    /// the value that left the pool.
    #[test]
    fn mismatched_retry_arguments_are_refused_per_field() {
        let stored = keys_map(&[our_master_key()]);
        let swapped = keys_map(&[other_master_key()]);

        // Same key ids, different key material — ids alone would not catch it.
        assert_eq!(
            stored.keys().collect::<Vec<_>>(),
            swapped.keys().collect::<Vec<_>>(),
            "precondition: the swap keeps the key ids identical"
        );

        let key_mismatch = one_time_claim_binding_mismatch(
            &stored,
            Some(OUR_MASTER_HASH),
            DENOMINATION,
            Some(IDENTITY_INDEX),
            &swapped,
            Some(OUR_MASTER_HASH),
            DENOMINATION,
            IDENTITY_INDEX,
        );
        assert!(
            key_mismatch.is_some_and(|m| m.contains("public key set")),
            "a swapped key set must be refused"
        );

        let hash_mismatch = one_time_claim_binding_mismatch(
            &stored,
            Some(OUR_MASTER_HASH),
            DENOMINATION,
            Some(IDENTITY_INDEX),
            &stored,
            Some(OTHER_MASTER_HASH),
            DENOMINATION,
            IDENTITY_INDEX,
        );
        assert!(
            hash_mismatch.is_some_and(|m| m.contains("master authentication key hash")),
            "a recovery handle that is not in the stored transition must be refused"
        );

        let denomination_mismatch = one_time_claim_binding_mismatch(
            &stored,
            Some(OUR_MASTER_HASH),
            DENOMINATION,
            Some(IDENTITY_INDEX),
            &stored,
            Some(OUR_MASTER_HASH),
            DENOMINATION * 3,
            IDENTITY_INDEX,
        );
        assert!(
            denomination_mismatch.is_some_and(|m| m.contains("denomination")),
            "a different denomination must be refused"
        );
    }

    /// END TO END, and the property that matters most: a mismatched retry must
    /// fail CLOSED — refused with `ShieldedClaimBindingMismatch` **before** any
    /// network work, with the pending record left intact so the correct retry
    /// can still resume. The SDK here is a bare mock with no expectations
    /// registered: reaching the spent-nullifier probe or the re-broadcast would
    /// surface as something other than this error.
    #[tokio::test]
    async fn mismatched_retry_refuses_without_broadcasting_or_clearing_the_record() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let path = temp_store_path("resume_binding");
        let store = Arc::new(RwLock::new(
            FileBackedShieldedStore::open_path(&path, 100).expect("store opens"),
        ));
        let claim_records_id = SubwalletId::new([0x77; 32], ONE_TIME_CLAIM_RECORDS_ACCOUNT);

        let (_, st_bytes) = stored_claim_transition(&[our_master_key()], DENOMINATION);
        let record = stored_claim_record(st_bytes);
        store
            .write()
            .await
            .arm_redrive(claim_records_id, record.clone())
            .expect("record arms");

        // The retry presents a DIFFERENT identity's keys — the mis-slot case.
        let outcome = resume_one_time_claim(
            &sdk,
            &store,
            claim_records_id,
            &record,
            // The binding mismatch is refused before the pre-broadcast
            // lease gate is ever reached.
            crate::wallet::shielded::store::AdmissionToken::generate().expect("token"),
            Some(OTHER_MASTER_HASH),
            keys_map(&[other_master_key()]),
            DENOMINATION,
            IDENTITY_INDEX,
        )
        .await;

        match outcome {
            OneTimeClaimResume::Resolved(Err(
                PlatformWalletError::ShieldedClaimBindingMismatch { mismatch },
            )) => assert!(
                mismatch.contains("master authentication key hash")
                    || mismatch.contains("public key set"),
                "the refusal must name the binding that failed, got: {mismatch}"
            ),
            other => panic!(
                "a retry with a different identity's keys must be refused, got {}",
                match other {
                    OneTimeClaimResume::RecordUnusable => "RecordUnusable".to_string(),
                    OneTimeClaimResume::Resolved(r) => format!("Resolved({r:?})"),
                }
            ),
        }

        // Fail-closed: the record survives, so the ORIGINAL claim is still
        // resumable. Clearing it here would strand a padded single-note claim
        // forever — its declared id exists nowhere else.
        let surviving = store
            .read()
            .await
            .pending_redrives(claim_records_id)
            .expect("records readable");
        assert_eq!(
            surviving.len(),
            1,
            "a refused retry must not clear the pending-claim record"
        );
        assert_eq!(
            surviving[0].st_bytes, record.st_bytes,
            "the stored transition must be untouched"
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    /// THE BUG (#4313 review finding 5d4d6efa): `identity_index` was bound only
    /// TRANSITIVELY — "a different slot means different keys, so the key check
    /// catches it". A retry that presents the ORIGINAL keys with a different
    /// slot breaks that chain, and its consequence is not symmetric with a
    /// first attempt's: `IdentityManager::add_identity` rejects a duplicate
    /// identity id but inserts into an OCCUPIED slot without complaint, so the
    /// retry silently displaces whatever identity the wallet tracked there.
    ///
    /// The record now carries the slot, so the mismatch is caught with
    /// everything else byte-identical — exactly the case the transitive
    /// argument could not cover.
    #[test]
    fn a_resume_at_a_different_identity_index_is_refused() {
        let keys = keys_map(&[our_master_key()]);

        let mismatch = one_time_claim_binding_mismatch(
            &keys,
            Some(OUR_MASTER_HASH),
            DENOMINATION,
            Some(IDENTITY_INDEX),
            // Everything else is identical — only the slot moved.
            &keys,
            Some(OUR_MASTER_HASH),
            DENOMINATION,
            IDENTITY_INDEX + 1,
        );
        let mismatch = mismatch.expect("a slot mismatch must be refused");
        assert!(
            mismatch.contains("identity index"),
            "the refusal must name the slot, got: {mismatch}"
        );
        assert!(
            mismatch.contains(&IDENTITY_INDEX.to_string())
                && mismatch.contains(&(IDENTITY_INDEX + 1).to_string()),
            "the refusal must carry both slots, got: {mismatch}"
        );
    }

    /// A record written before the column existed carries `None`. There is
    /// nothing to compare it against, so it keeps exactly the transitive
    /// binding it was written under rather than being refused outright — an
    /// upgrade must not strand a claim that is mid-flight across it.
    #[test]
    fn a_pre_migration_record_still_resumes_at_any_index() {
        let keys = keys_map(&[our_master_key()]);

        assert_eq!(
            one_time_claim_binding_mismatch(
                &keys,
                Some(OUR_MASTER_HASH),
                DENOMINATION,
                None,
                &keys,
                Some(OUR_MASTER_HASH),
                DENOMINATION,
                IDENTITY_INDEX + 7,
            ),
            None,
            "a record with no persisted slot must not be refused on the slot"
        );
    }

    /// END TO END on the real file store: arm a claim at slot N, attempt to
    /// resume it at N+1, and get the typed refusal BEFORE any network work —
    /// with the record left intact for a correct retry. The SDK is a bare mock
    /// with no expectations registered, so reaching the spent-nullifier probe
    /// or the re-broadcast would surface as something other than this error.
    #[tokio::test]
    async fn resuming_at_a_mismatched_identity_index_fails_closed() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let path = temp_store_path("resume_slot_binding");
        let store = Arc::new(RwLock::new(
            FileBackedShieldedStore::open_path(&path, 100).expect("store opens"),
        ));
        let claim_records_id = SubwalletId::new([0x78; 32], ONE_TIME_CLAIM_RECORDS_ACCOUNT);

        let (_, st_bytes) = stored_claim_transition(&[our_master_key()], DENOMINATION);
        let record = stored_claim_record(st_bytes);
        assert_eq!(
            record.identity_index,
            Some(IDENTITY_INDEX),
            "precondition: the armed record carries the slot"
        );
        store
            .write()
            .await
            .arm_redrive(claim_records_id, record.clone())
            .expect("record arms");

        // The retry presents the ORIGINAL keys, hash and denomination — only
        // the slot differs. Nothing but the persisted index can catch this.
        let outcome = resume_one_time_claim(
            &sdk,
            &store,
            claim_records_id,
            &record,
            // As above: refused on the slot mismatch, before the gate.
            crate::wallet::shielded::store::AdmissionToken::generate().expect("token"),
            Some(OUR_MASTER_HASH),
            keys_map(&[our_master_key()]),
            DENOMINATION,
            IDENTITY_INDEX + 1,
        )
        .await;

        match outcome {
            OneTimeClaimResume::Resolved(Err(
                PlatformWalletError::ShieldedClaimBindingMismatch { mismatch },
            )) => assert!(
                mismatch.contains("identity index"),
                "the refusal must name the slot, got: {mismatch}"
            ),
            other => panic!(
                "a retry at a different slot must be refused, got {}",
                match other {
                    OneTimeClaimResume::RecordUnusable => "RecordUnusable".to_string(),
                    OneTimeClaimResume::Resolved(r) => format!("Resolved({r:?})"),
                }
            ),
        }

        // Fail-closed: the record survives for a retry that names slot N.
        let surviving = store
            .read()
            .await
            .pending_redrives(claim_records_id)
            .expect("records readable");
        assert_eq!(surviving.len(), 1);
        assert_eq!(surviving[0].identity_index, Some(IDENTITY_INDEX));

        // …and the slot is DURABLE, not just in-memory: a cold reopen still
        // carries it, which is the whole point of the schema change.
        drop(store);
        let reopened = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
        let rehydrated = reopened
            .pending_redrives(claim_records_id)
            .expect("records readable");
        assert_eq!(rehydrated.len(), 1);
        assert_eq!(
            rehydrated[0].identity_index,
            Some(IDENTITY_INDEX),
            "the persisted slot must survive a process restart"
        );
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    /// END TO END for #4313 review finding f58ed9d910d8: a resume whose
    /// purge-protection lease is gone must refuse RETRYABLY and must not
    /// re-broadcast.
    ///
    /// A re-broadcast is a chargeable resubmission, and continuing it without a
    /// provable lease is what lets a concurrent clear or wallet removal count
    /// zero live claims and delete the recovery record while the transition is
    /// in flight. The SDK is a bare mock with no expectations registered, so a
    /// re-broadcast would surface as a broadcast-shaped error rather than as
    /// `ShieldedLifecycleBusy`.
    ///
    /// The record must SURVIVE: refusing is only safe because the retry can
    /// still resume it.
    #[tokio::test]
    async fn a_resume_without_a_provable_lease_refuses_instead_of_rebroadcasting() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let path = temp_store_path("resume_lease_gate");
        let store = Arc::new(RwLock::new(
            FileBackedShieldedStore::open_path(&path, 100).expect("store opens"),
        ));
        let claim_records_id = SubwalletId::new([0x78; 32], ONE_TIME_CLAIM_RECORDS_ACCOUNT);

        let (_, st_bytes) = stored_claim_transition(&[our_master_key()], DENOMINATION);
        let record = stored_claim_record(st_bytes);
        store
            .write()
            .await
            .arm_redrive(claim_records_id, record.clone())
            .expect("record arms");

        // A token that owns no lease — the state a reaped or displaced claim is
        // left in. Every binding below MATCHES the stored transition, so the
        // mismatch gate cannot be what refuses this.
        let orphaned = crate::wallet::shielded::store::AdmissionToken([0x7F; 16]);
        let outcome = resume_one_time_claim(
            &sdk,
            &store,
            claim_records_id,
            &record,
            orphaned,
            Some(OUR_MASTER_HASH),
            keys_map(&[our_master_key()]),
            DENOMINATION,
            IDENTITY_INDEX,
        )
        .await;

        match outcome {
            OneTimeClaimResume::Resolved(Err(PlatformWalletError::ShieldedLifecycleBusy {
                reason,
            })) => assert!(
                reason.contains("lease"),
                "the refusal must name the lost lease, got: {reason}"
            ),
            other => panic!(
                "a resume without a provable lease must refuse retryably, got {}",
                match other {
                    OneTimeClaimResume::RecordUnusable => "RecordUnusable".to_string(),
                    OneTimeClaimResume::Resolved(r) => format!("Resolved({r:?})"),
                }
            ),
        }

        let surviving = store
            .read()
            .await
            .pending_redrives(claim_records_id)
            .expect("records readable");
        assert_eq!(
            surviving.len(),
            1,
            "refusing is only safe because the record survives for the retry"
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(test)]
mod claim_lease_heartbeat_tests {
    use super::*;
    use crate::wallet::shielded::store::{
        admission_now_ms, AdmissionToken, InMemoryShieldedStore, CLAIM_LEASE_MS,
        CLAIM_LEASE_RENEW_INTERVAL,
    };

    /// How long the initial lease is stamped for in these tests: long enough
    /// that the first heartbeat tick still finds it live (wall-clock time
    /// barely advances under a paused runtime), short enough that the probe
    /// below can tell "renewed" from "not renewed".
    const SHORT_LEASE_MS: u64 = 5_000;

    /// A body shaped like the RESUME path: it does real awaiting and then
    /// returns its own outcome, without ever reaching the fresh-build
    /// broadcast the heartbeat used to wrap.
    async fn resume_shaped_body() -> &'static str {
        for _ in 0..3 {
            tokio::time::sleep(CLAIM_LEASE_RENEW_INTERVAL).await;
        }
        "resumed"
    }

    /// THE BUG (#4313 review finding 8de8d05a): the heartbeat wrapped only the
    /// fresh-build broadcast, so a claim that took the RESUME path — nullifier
    /// queries, repeated identity recovery, re-broadcast, an unbounded
    /// confirmation wait — ran under the initial lease alone. Outrun it and the
    /// lease is reaped, at which point a concurrent purge counts zero live
    /// claims and deletes the record the claim needs.
    ///
    /// Control half: run the same body bare and watch the lease lapse.
    #[tokio::test(start_paused = true)]
    async fn a_resume_shaped_body_run_bare_lets_its_lease_lapse() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id: WalletId = [0x41; 32];
        let token = AdmissionToken([0x41; 16]);
        let t0 = admission_now_ms();
        assert!(store
            .write()
            .await
            .begin_claim_admission(wallet_id, token, t0, SHORT_LEASE_MS)
            .expect("lease"));

        assert_eq!(resume_shaped_body().await, "resumed");

        // Probe: is the lease still live at a point past its ORIGINAL expiry?
        // Nothing re-stamped it, so no.
        assert!(
            !store
                .write()
                .await
                .renew_claim_admission(token, t0 + SHORT_LEASE_MS + 1, CLAIM_LEASE_MS)
                .expect("probe"),
            "without a heartbeat the resume path's lease lapses — this is the bug"
        );
    }

    /// The fix: the heartbeat wraps the COMPLETE admitted claim body, so the
    /// resume path is covered by exactly the same renewal the fresh-build path
    /// gets. Same body, same clock, opposite outcome.
    #[tokio::test(start_paused = true)]
    async fn the_heartbeat_keeps_a_resume_shaped_body_s_lease_live() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id: WalletId = [0x42; 32];
        let token = AdmissionToken([0x42; 16]);
        let t0 = admission_now_ms();
        assert!(store
            .write()
            .await
            .begin_claim_admission(wallet_id, token, t0, SHORT_LEASE_MS)
            .expect("lease"));

        let outcome = under_renewed_claim_lease(&store, token, resume_shaped_body()).await;
        assert_eq!(
            outcome, "resumed",
            "the helper must return the body's value"
        );

        assert!(
            store
                .write()
                .await
                .renew_claim_admission(token, t0 + SHORT_LEASE_MS + 1, CLAIM_LEASE_MS)
                .expect("probe"),
            "the heartbeat must have re-stamped the lease past its original expiry"
        );
    }

    /// The claim-key reservation rides the same token, so the heartbeat holds
    /// the invitation too — a long resume must not lose its claim key to expiry
    /// while its lease is being kept alive.
    #[tokio::test(start_paused = true)]
    async fn the_heartbeat_also_holds_the_claim_key_reservation() {
        use crate::wallet::shielded::store::ClaimKeyReservation;

        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id: WalletId = [0x43; 32];
        let claim_key = [0xC5; 32];
        let claim_records_id = SubwalletId::new(wallet_id, ONE_TIME_CLAIM_RECORDS_ACCOUNT);
        let holder = AdmissionToken([0x43; 16]);
        let rival = AdmissionToken([0x44; 16]);
        let t0 = admission_now_ms();
        {
            let mut guard = store.write().await;
            assert!(guard
                .begin_claim_admission(wallet_id, holder, t0, SHORT_LEASE_MS)
                .expect("lease"));
            assert_eq!(
                guard
                    .reserve_one_time_claim_key(
                        claim_records_id,
                        claim_key,
                        holder,
                        t0,
                        SHORT_LEASE_MS
                    )
                    .expect("reserve")
                    .reservation,
                ClaimKeyReservation::Acquired
            );
        }

        under_renewed_claim_lease(&store, holder, resume_shaped_body()).await;

        // Past the ORIGINAL reservation expiry, a rival must still be refused.
        let contended = {
            let mut guard = store.write().await;
            assert!(guard
                .begin_claim_admission(wallet_id, rival, t0 + SHORT_LEASE_MS + 1, CLAIM_LEASE_MS)
                .expect("rival lease"));
            guard
                .reserve_one_time_claim_key(
                    claim_records_id,
                    claim_key,
                    rival,
                    t0 + SHORT_LEASE_MS + 1,
                    CLAIM_LEASE_MS,
                )
                .expect("rival reserve")
        };
        assert!(
            !contended.is_acquired(),
            "the heartbeat must carry the claim-key reservation past its original expiry"
        );
    }

    /// A live lease authorizes the chargeable step, and re-stamps itself while
    /// doing so — the broadcast that follows therefore runs at the start of a
    /// full lease window rather than at whatever a long proof build left of one.
    #[tokio::test(start_paused = true)]
    async fn a_live_lease_authorizes_the_chargeable_step_and_is_restamped() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id: WalletId = [0x51; 32];
        let holder = AdmissionToken([0x51; 16]);
        let rival = AdmissionToken([0x52; 16]);
        let t0 = admission_now_ms();
        assert!(store
            .write()
            .await
            .begin_claim_admission(wallet_id, holder, t0, SHORT_LEASE_MS)
            .expect("lease"));

        assert_claim_lease_before_chargeable_step(&store, holder, "the claim broadcast")
            .await
            .expect("a live lease must authorize the broadcast");

        // Re-stamped: past the ORIGINAL expiry the lease is still counted, so a
        // destructive pass still sees a live claim.
        let live = store
            .write()
            .await
            .begin_destructive_admission(
                Some(wallet_id),
                rival,
                t0 + SHORT_LEASE_MS + 1,
                CLAIM_LEASE_MS,
            )
            .expect("destructive admission");
        assert_eq!(
            live, 1,
            "the gate must re-stamp the lease it proves, so the broadcast runs inside a fresh \
             protection window"
        );
    }

    /// THE BUG (#4313 review finding f58ed9d910d8): the heartbeat only LOGGED a
    /// failed renewal and let the claim run on. Once the lease is gone,
    /// `renew_claim_admission` refuses to resurrect it and destructive
    /// admission reaps it before counting live claims — so a concurrent clear
    /// or wallet removal counts zero claims and deletes the pending row while
    /// the transition is on the wire. For a padded single-note bundle that row
    /// is the only handle to the randomized identity id.
    ///
    /// So a chargeable step must refuse, retryably, when the lease is gone.
    #[tokio::test(start_paused = true)]
    async fn a_lapsed_lease_refuses_the_chargeable_step() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id: WalletId = [0x53; 32];
        let holder = AdmissionToken([0x53; 16]);
        let t0 = admission_now_ms();
        assert!(store
            .write()
            .await
            .begin_claim_admission(wallet_id, holder, t0, SHORT_LEASE_MS)
            .expect("lease"));

        // The lease is displaced exactly as a purge's reap would displace it.
        store
            .write()
            .await
            .end_claim_admission(holder)
            .expect("release the lease");

        let refused =
            assert_claim_lease_before_chargeable_step(&store, holder, "the claim broadcast").await;
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::ShieldedLifecycleBusy { .. })
            ),
            "a chargeable step must refuse — RETRYABLY — without a lease it can prove; got \
             {refused:?}"
        );
    }

    /// The same refusal for a lease that merely EXPIRED on the wall clock,
    /// which is the case a forward clock adjustment or a long executor
    /// suspension produces: `renew_claim_admission` will not resurrect it, so
    /// ownership can no longer be proven and the claim must not broadcast.
    #[tokio::test]
    async fn an_expired_lease_cannot_be_resurrected_to_authorize_a_broadcast() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id: WalletId = [0x54; 32];
        let holder = AdmissionToken([0x54; 16]);
        let t0 = admission_now_ms();
        // A zero-length lease: `expires_at == t0`, already expired against any
        // `now >= t0` the gate reads. Expressing the expiry through the STAMP
        // rather than by advancing time is what makes this deterministic —
        // `admission_now_ms` is wall-clock, which a paused runtime never moves,
        // so a short-but-nonzero lease races the gate.
        assert!(store
            .write()
            .await
            .begin_claim_admission(wallet_id, holder, t0, 0)
            .expect("lease"));

        // `renew_claim_admission`'s `expires_at > now` predicate deliberately
        // refuses to resurrect it — the store-level behaviour this gate is
        // built on, and the reason a lapsed claim cannot renew its way back
        // into ownership.
        let renewed = store
            .write()
            .await
            .renew_claim_admission(holder, admission_now_ms(), CLAIM_LEASE_MS)
            .expect("renew must not error");
        assert!(
            !renewed,
            "precondition: an expired lease is deliberately not resurrectable"
        );

        let refused =
            assert_claim_lease_before_chargeable_step(&store, holder, "the claim re-broadcast")
                .await;
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::ShieldedLifecycleBusy { .. })
            ),
            "an expired lease must not authorize a re-broadcast; got {refused:?}"
        );
    }
}

/// The claim's removal fence (#4313 review finding 4a2c679745bb).
#[cfg(test)]
mod claim_removal_fence_tests {
    use super::*;
    use crate::wallet::shielded::keys::OrchardKeySet;
    use crate::wallet::shielded::store::{
        admission_now_ms, AdmissionToken, InMemoryShieldedStore, DESTRUCTIVE_BARRIER_MS,
    };
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use std::sync::atomic::AtomicBool;

    /// Smallest member of the versioned exit-denomination set (0.1 DASH).
    const DENOMINATION: u64 = 10_000_000_000;

    /// A prover that cannot prove. Reaching it means the claim got past the
    /// removal fence and started building a bundle, which is the whole failure
    /// this test exists to catch — so it is `unreachable!`, not a stub.
    #[derive(Debug)]
    struct NeverProver;

    impl OrchardProver for NeverProver {
        fn proving_key(&self) -> &grovedb_commitment_tree::ProvingKey {
            unreachable!("a claim refused at the removal fence must never build a bundle")
        }
    }

    /// A signer that cannot sign, for the same reason.
    #[derive(Debug)]
    struct NeverSigner;

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for NeverSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, dpp::ProtocolError> {
            unreachable!("a claim refused at the removal fence must never sign")
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("a claim refused at the removal fence must never sign")
        }

        fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
            true
        }
    }

    fn master_auth_key() -> (IdentityPublicKey, IdentityPublicKeyInCreation) {
        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(vec![0xA1; 20]),
            disabled_at: None,
        });
        let in_creation: IdentityPublicKeyInCreation = (&key).into();
        (key, in_creation)
    }

    /// The claimer's own default Orchard address, for the change note.
    fn change_address() -> OrchardAddress {
        let keys = OrchardKeySet::from_seed(&[0x42u8; 64], dashcore::Network::Testnet, 0)
            .expect("derive the claimer's key set");
        OrchardAddress::from_raw_bytes(&keys.default_address.to_raw_address_bytes())
            .expect("the derived default address is a valid Orchard address")
    }

    /// A claim that begins AFTER its wallet's removal has committed must be
    /// refused before it scans, builds, or broadcasts anything
    /// (#4313 review finding 4a2c679745bb).
    ///
    /// The reviewer's sequence: an FFI caller resolves and retains the wallet
    /// and coordinator, `unregister_wallet_with` then commits the detach,
    /// purges the store, RELEASES its destructive barrier and drops the wallet
    /// from the manager — and only then does the stale handle start executing.
    /// With the barrier already released, the claim's own admission succeeds,
    /// so admission alone does not refuse it; before the fix it went on to arm
    /// a pending row and broadcast for a wallet the host had already removed,
    /// and its registration tail — finding no manager entry — left that row
    /// behind.
    ///
    /// This drives that state directly: `detached` is already `true` and no
    /// destructive admission is held, exactly as after a completed removal.
    #[tokio::test]
    async fn a_claim_that_starts_after_a_committed_removal_is_refused() {
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let guards = ForeignClaimGuards::default();
        let checkpoints = super::super::sync::ForeignScanCheckpointCache::default();
        let wallet_id: WalletId = [0x11; 32];

        // The removal has fully committed AND released its barrier.
        let detached = AtomicBool::new(true);

        let (_key, _in_creation) = master_auth_key();
        let result = identity_create_from_one_time_key(
            &sdk,
            &store,
            &guards,
            &checkpoints,
            &detached,
            wallet_id,
            zeroize::Zeroizing::new([0x01u8; 32]),
            None,
            &change_address(),
            3,
            vec![master_auth_key()],
            DENOMINATION,
            PlatformAddress::P2pkh([0x07; 20]),
            &NeverSigner,
            &NeverProver,
        )
        .await;

        // RED before the fix: the claim ran on, and the `unreachable!` prover /
        // signer above (or a mock-SDK scan error) stood in for the broadcast it
        // would have made for a removed wallet.
        match result {
            Err(PlatformWalletError::WalletNotFound(reason)) => {
                assert!(
                    reason.contains(&hex::encode(wallet_id)),
                    "the refusal must name the removed wallet, got: {reason}"
                );
            }
            other => panic!(
                "a claim beginning after a committed removal must be refused as WalletNotFound, \
                 got {other:?}"
            ),
        }

        // Nothing was armed for the removed wallet — the point of refusing
        // under admission rather than after the broadcast.
        let record_id = SubwalletId::new(wallet_id, ONE_TIME_CLAIM_RECORDS_ACCOUNT);
        assert!(
            store
                .read()
                .await
                .pending_redrives(record_id)
                .expect("records readable")
                .is_empty(),
            "a refused claim must leave no pending record behind"
        );

        // And the admission the refusal took was RELEASED, not left to expire:
        // a destructive operation on this wallet must see zero live claims.
        // The early return is on the fence path, so this is the only test that
        // exercises that path's release.
        let token = AdmissionToken::generate().expect("token");
        let live = store
            .write()
            .await
            .begin_destructive_admission(
                Some(wallet_id),
                token,
                admission_now_ms(),
                DESTRUCTIVE_BARRIER_MS,
            )
            .expect("admission");
        assert_eq!(
            live, 0,
            "the refusal must release its claim lease before returning"
        );
    }
}
