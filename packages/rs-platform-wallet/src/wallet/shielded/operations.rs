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
    ensure_allowed_denomination, ensure_positive_amount, select_notes_for_denomination,
    select_notes_with_fee, ShieldedFeeKind,
};
use super::store::{
    PendingRedrive, PrivateContactPayment, ShieldedNote, ShieldedStore, SubwalletId,
};
use crate::broadcast_outcome::{
    broadcast_definitely_failed, broadcast_tracking_delivery, carries_consensus_rejection,
    BroadcastDelivery,
};
use crate::changeset::{PlatformWalletChangeSet, ShieldedChangeSet};
use crate::error::{preserve_signer_key_unavailable_or, PlatformWalletError};
use crate::wallet::identity::network::PaymentStarted;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

use std::collections::{BTreeMap, HashSet};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::identity_create_from_shielded_pool::IdentityCreateFromShieldedPool;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityNonceFetcher;
use dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, OrchardAddress, PlatformAddress,
};
use dpp::fee::Credits;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::core_script::CoreScript;
use dpp::identity::identity_nonce::{
    IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES,
    MAX_MISSING_IDENTITY_REVISIONS, MISSING_IDENTITY_REVISIONS_FILTER,
};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::BinaryData;
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::shielded::builder::{
    build_identity_create_from_shielded_pool_transition,
    build_identity_top_up_from_shielded_pool_transition,
    build_private_document_create_from_shielded_pool_transition,
    build_shield_from_identity_transition, build_shield_transition,
    build_shielded_transfer_transition, build_shielded_withdrawal_transition,
    build_unshield_transition, OrchardProver, SpendableNote,
};
use dpp::shielded::compute_minimum_shielded_fee;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;
use dpp::ProtocolError;
use grovedb_commitment_tree::{Anchor, PaymentAddress};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
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
pub(super) fn queue_shielded_changeset(
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
    // `actions()` is on each type's accessor trait; Shield exposes `actions`
    // only as a public field on its V0 struct, so match down to that
    // variant for it.
    use dpp::state_transition::shield_from_asset_lock_transition::accessors::ShieldFromAssetLockTransitionAccessorsV0;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::shielded_transfer_transition::accessors::ShieldedTransferTransitionAccessorsV0;
    use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;
    use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
    use dpp::state_transition::identity_top_up_from_shielded_pool_transition::accessors::IdentityTopUpFromShieldedPoolTransitionAccessorsV0;
    use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
    use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;
    use dpp::state_transition::private_document_create_from_shielded_pool_transition::accessors::PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0;

    match st {
        StateTransition::Shield(ShieldTransition::V0(v0)) => &v0.actions,
        StateTransition::ShieldedTransfer(t) => t.actions(),
        StateTransition::Unshield(t) => t.actions(),
        StateTransition::ShieldFromAssetLock(t) => t.actions(),
        StateTransition::ShieldedWithdrawal(t) => t.actions(),
        StateTransition::IdentityCreateFromShieldedPool(t) => t.actions(),
        StateTransition::ShieldFromIdentity(t) => t.actions(),
        StateTransition::IdentityTopUpFromShieldedPool(t) => t.actions(),
        StateTransition::PrivateDocumentCreateFromShieldedPool(t) => t.actions(),
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
// ShieldFromIdentity: identity balance -> shielded pool (Type 21)
// -------------------------------------------------------------------------

/// Return whether this wallet already has an exact ShieldFromIdentity
/// transition whose outcome is unresolved. These records are persisted by
/// the file-backed store before broadcast, so a process restart cannot turn
/// an ambiguous debit into a fresh transition with the next identity nonce.
fn has_unresolved_identity_shield<S: ShieldedStore>(
    store: &S,
    wallet_id: WalletId,
    identity_id: [u8; 32],
) -> Result<bool, S::Error> {
    use dpp::serialization::PlatformDeserializableTrusted;
    use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;

    for (_, redrive) in store.pending_redrives_for_wallet(wallet_id)? {
        if redrive.identity_user_abandoned && redrive.nullifiers.is_empty() {
            continue;
        }
        match StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes) {
            Ok(StateTransition::ShieldFromIdentity(transition))
                if transition.identity_id().to_buffer() == identity_id =>
            {
                return Ok(true);
            }
            // An empty-nullifier redrive is reserved for identity-funded
            // shielding. If it is unreadable or decodes to another transition
            // kind, fail closed because its identity cannot be recovered safely.
            Err(_) if redrive.nullifiers.is_empty() => return Ok(true),
            Ok(StateTransition::ShieldFromIdentity(_)) => {}
            Ok(_) if redrive.nullifiers.is_empty() => return Ok(true),
            _ => {}
        }
    }
    Ok(false)
}

/// Persist the exact signed identity debit before any network request. It is
/// safe to re-broadcast these bytes because their identity nonce is fixed; the
/// same record also blocks the public API from building a second debit while
/// the first outcome is unresolved.
async fn arm_identity_shield_redrive<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    state_transition: &StateTransition,
) -> Result<[u8; 32], PlatformWalletError> {
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;

    let entry = pending_entry.as_ref().ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError(
            "ShieldFromIdentity produced no wallet-visible activity entry".to_string(),
        )
    })?;
    let StateTransition::ShieldFromIdentity(transition) = state_transition else {
        return Err(PlatformWalletError::ShieldedBuildError(
            "expected a ShieldFromIdentity transition".to_string(),
        ));
    };
    let st_bytes = state_transition.serialize_to_bytes().map_err(|e| {
        PlatformWalletError::ShieldedBuildError(format!(
            "serialize ShieldFromIdentity retry guard: {e}"
        ))
    })?;
    store
        .write()
        .await
        .arm_redrive(
            id,
            PendingRedrive {
                activity_id: entry.id,
                anchor: transition.anchor(),
                nullifiers: vec![],
                st_bytes,
                attempts: 0,
                identity_nonce_finalized: false,
                identity_user_abandoned: false,
            },
        )
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    Ok(entry.id)
}

/// Shield credits from a Platform identity's balance straight into the shielded
/// pool, with the resulting note assigned to `account`'s default Orchard address
/// (`recipient` `None`) or to a third-party Orchard address.
///
/// Unlike [`shield_to`] there are no transparent inputs, no fee strategy, and no
/// address witnesses: the identity's TRANSFER key signs the whole transition and
/// consensus debits `amount` plus the metered fee (note writes + identity writes)
/// plus the shielded compute fee from the identity balance. `nonce` is fetched
/// from Platform here, so the caller only supplies the identity.
///
/// The activity row records no fee: the exact fee is metered at execution and
/// never returned to the wallet, and `ShieldedActivityEntry::fee` is an exact
/// value (the FFI reports it as such), so the consensus floor must not stand in
/// for it.
///
/// Before broadcast, the bundle must produce a wallet-visible activity entry
/// and the exact signed transition must be persisted. Missing activity or a
/// failed durable guard write aborts the call without broadcasting. Once
/// submitted, remote failure verdicts never authorize building
/// a replacement: an unused nonce can still execute later, and a consumed nonce
/// may belong to this payment. The row stays pending until the scan observes its
/// outputs. Proven nonce finality stops rebroadcasts while retaining the guard.
/// Further calls for the same identity return
/// [`PlatformWalletError::ShieldedIdentityDebitPending`] before building a proof.
///
/// Returns the identity's proven post-debit balance, which requires the
/// identity's own `VerifiedPartialIdentity` proof carrying a balance (the SDK's
/// `ShieldFromIdentity` check); a proof for another identity, one without a
/// balance, or any other result reports `ShieldedSpendUnconfirmed`. That balance
/// proof is an affected-state snapshot: it binds neither this transition nor the
/// created note, so it does NOT confirm the activity row. The row stays `Pending`
/// until the shielded scan observes the note commitments on-chain (the
/// coordinator's confirmation pass), exactly like the ambiguous post-broadcast
/// paths. The caller owns the managed identity and persists the balance
/// (`PlatformWallet::shielded_shield_from_identity`).
#[allow(clippy::too_many_arguments)]
pub(in crate::wallet) async fn shield_from_identity_to<
    S: ShieldedStore,
    Sig: Signer<IdentityPublicKey>,
    P: OrchardProver,
>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &AccountViewingKeys,
    account: u32,
    recipient: Option<&PaymentAddress>,
    identity: &Identity,
    amount: u64,
    memo: [u8; 36],
    signer: &Sig,
    prover: &P,
) -> Result<(Credits, ResponseMetadata), PlatformWalletError> {
    let ShieldRecipient {
        address: recipient_addr,
        counterparty: external_counterparty,
        kind,
        direction,
    } = resolve_shield_recipient(keys, recipient)?;
    let id = SubwalletId::new(wallet_id, account);
    let identity_id = identity.id();

    let has_unresolved = {
        let store = store.read().await;
        has_unresolved_identity_shield(&*store, wallet_id, identity_id.to_buffer()).map_err(
            |e| {
                PlatformWalletError::ShieldedStoreError(format!(
                    "could not inspect unresolved identity debits: {e}"
                ))
            },
        )?
    };
    if has_unresolved {
        return Err(PlatformWalletError::ShieldedIdentityDebitPending {
            identity_id: identity_id.to_buffer(),
        });
    }

    // A self-shield funded by an identity is its own activity kind so the
    // history shows which identity paid; a third-party recipient stays `Sent`.
    let kind = match kind {
        ShieldedActivityKind::Shield => ShieldedActivityKind::ShieldFromIdentity {
            identity_id: identity_id.to_buffer(),
        },
        other => other,
    };

    let nonce = sdk
        .get_identity_nonce(identity_id, true, None)
        .await
        .map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!("fetch identity nonce: {e}"))
        })?;

    info!(
        account,
        credits = amount,
        identity = %identity_id,
        external = external_counterparty.is_some(),
        "ShieldFromIdentity: building proof"
    );

    let state_transition = build_shield_from_identity_transition(
        identity,
        &recipient_addr,
        amount,
        nonce,
        signer,
        None,
        0, // user_fee_increase
        prover,
        memo,
        Some(keys.outgoing_viewing_key.clone()),
        sdk.version(),
    )
    .await
    .map_err(map_shield_from_identity_build_error)?;

    trace!("ShieldFromIdentity: state transition built, broadcasting...");

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
            // Exact fee unknown (metered at execution); see the function docs.
            fee: None,
            counterparty: external_counterparty,
            memo: non_zero_memo(&memo),
            actions: shielded_actions(&state_transition),
            spent_notes: &[],
        },
    )
    .await;

    let activity_id =
        arm_identity_shield_redrive(store, id, &pending_entry, &state_transition).await?;

    match state_transition.broadcast(sdk, None).await {
        Ok(()) => {}
        Err(e) if broadcast_definitely_failed(&e) => {
            // Unauthenticated verdict: settle it against the proven nonce.
            return Err(settle_identity_shield_failure(
                sdk,
                store,
                id,
                activity_id,
                identity_id,
                nonce,
                e,
            )
            .await);
        }
        Err(e) => {
            warn!(
                account,
                error = %e,
                "ShieldFromIdentity broadcast returned no verdict; the transition may have \
                 been admitted; falling through to the result wait"
            );
        }
    }

    // The proof authenticates the identity's post-debit balance (an affected-state
    // snapshot, not execution evidence). Remote rejection errors are not
    // authenticated and therefore leave the debit unresolved.
    // `wait_for_affected_state` only converts the proof generically, so the variant
    // and the identity are enforced here: only this identity's balance proof is
    // accepted as the post-debit balance.
    let proof_outcome: Result<(Credits, ResponseMetadata), String> = match state_transition
        .wait_for_affected_state_with_metadata::<StateTransitionProofResult>(sdk, None)
        .await
    {
        Ok((StateTransitionProofResult::VerifiedPartialIdentity(partial), metadata))
            if partial.id == identity_id =>
        {
            partial
                .balance
                .ok_or_else(|| "the identity proof did not include the updated balance".to_string())
                .map(|balance| (balance, metadata))
        }
        Ok((StateTransitionProofResult::VerifiedPartialIdentity(partial), _)) => Err(format!(
            "the proof returned identity {} but {} initiated the shield",
            partial.id, identity_id
        )),
        Ok(other) => Err(format!(
            "an identity balance proof was expected, received {other:?}"
        )),
        Err(wait_err) => {
            if carries_consensus_rejection(&wait_err) {
                // The wait-side error envelope is unauthenticated too: settle it
                // against the proven nonce before recording a failure.
                return Err(settle_identity_shield_failure(
                    sdk,
                    store,
                    id,
                    activity_id,
                    identity_id,
                    nonce,
                    wait_err,
                )
                .await);
            }
            Err(wait_err.to_string())
        }
    };
    let new_balance = match proof_outcome {
        Ok(balance) => balance,
        Err(reason) => {
            warn!(
                account,
                %reason,
                "ShieldFromIdentity broadcast accepted but result confirmation failed; \
                 leaving the activity row pending"
            );
            return Err(PlatformWalletError::ShieldedSpendUnconfirmed {
                operation: "shield from identity",
                reason,
            });
        }
    };

    // Not confirmed here: the balance proof does not bind this transition or its
    // note, so the row stays `Pending` until the scan's confirmation pass observes
    // the note commitments on-chain (see the function docs).
    info!(
        account,
        credits = amount,
        identity = %identity_id,
        "ShieldFromIdentity admitted with a proven post-debit balance; the activity row \
         awaits on-chain confirmation by the shielded scan"
    );
    Ok(new_balance)
}

/// Whether a proven identity nonce word still leaves `used` available to execute.
/// This is a snapshot, not a promise of non-execution: a relayed transaction can
/// consume an unused nonce after the proof was fetched.
///
/// Platform stores the identity nonce as the highest merged value (the low
/// `IDENTITY_NONCE_VALUE_FILTER` bits) plus a bitmask of the up to
/// `MAX_MISSING_IDENTITY_REVISIONS` values below it that were skipped (bit
/// `position - 1 + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES` set means `tip - position`
/// is still missing). `used` is proven unmerged when it lies above the tip, or when it
/// lies within the window below the tip and its missing bit is set. A value older than
/// the window can no longer be told apart from a merged one, so it is treated as
/// possibly used.
pub(crate) fn identity_nonce_proves_unused(stored: IdentityNonce, used: IdentityNonce) -> bool {
    let tip = stored & IDENTITY_NONCE_VALUE_FILTER;
    if used > tip {
        return true;
    }
    if used == tip {
        return false;
    }
    let position = tip - used;
    if position > MAX_MISSING_IDENTITY_REVISIONS {
        return false;
    }
    let missing = stored & MISSING_IDENTITY_REVISIONS_FILTER;
    missing & (1u64 << (position - 1 + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES)) != 0
}

/// Park exact-byte retries once a proof makes the nonce unavailable. Both
/// outcomes retain the payment guard: unused may execute later, while finalized
/// may already have executed. Never resurrect a guard the scan already cleared.
fn reconcile_identity_shield_nonce<S: ShieldedStore>(
    store: &mut S,
    id: SubwalletId,
    activity_id: [u8; 32],
    stored_nonce: Option<IdentityNonce>,
    nonce: IdentityNonce,
) -> Result<bool, S::Error> {
    if stored_nonce
        .map(|stored| identity_nonce_proves_unused(stored, nonce))
        .unwrap_or(true)
    {
        return Ok(false);
    }
    if let Some(mut redrive) = store
        .pending_redrives(id)?
        .into_iter()
        .find(|redrive| redrive.activity_id == activity_id)
    {
        if !redrive.identity_nonce_finalized && !redrive.identity_user_abandoned {
            redrive.identity_nonce_finalized = true;
            store.arm_redrive(id, redrive)?;
        }
    }
    Ok(true)
}

/// A relay verdict, including an apparent consensus rejection, cannot prove
/// non-execution. A valid nonce proof may stop futile rebroadcasting but can
/// never turn that verdict into permission for a fresh-nonce replacement.
async fn settle_identity_shield_failure<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    activity_id: [u8; 32],
    identity_id: Identifier,
    nonce: IdentityNonce,
    verdict: dash_sdk::Error,
) -> PlatformWalletError {
    let reason = match IdentityNonceFetcher::fetch(sdk, identity_id).await {
        Ok(fetched) => {
            let mut store = store.write().await;
            match reconcile_identity_shield_nonce(
                &mut *store,
                id,
                activity_id,
                fetched.map(|fetcher| fetcher.0),
                nonce,
            ) {
                Ok(true) => {
                    "the nonce can no longer execute; awaiting output confirmation".to_string()
                }
                Ok(false) => {
                    "the nonce is unused now, but the signed debit may still execute later"
                        .to_string()
                }
                Err(e) => format!("could not persist nonce reconciliation: {e}"),
            }
        }
        Err(e) => format!("the identity nonce could not be verified: {e}"),
    };
    PlatformWalletError::ShieldedSpendUnconfirmed {
        operation: "shield from identity",
        reason: format!("{verdict}; {reason}; the original payment remains pending"),
    }
}

#[cfg(test)]
mod identity_nonce_proves_unused_tests {
    use super::*;

    fn missing_bit(position: u64) -> u64 {
        1u64 << (position - 1 + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES)
    }

    /// Above the tip: never merged. At the tip: merged.
    #[test]
    fn tip_comparisons() {
        assert!(
            identity_nonce_proves_unused(0, 1),
            "fresh identity, nonce 1 unused"
        );
        assert!(identity_nonce_proves_unused(5, 6));
        assert!(!identity_nonce_proves_unused(5, 5));
    }

    /// Below the tip within the window: unused only when its missing bit is set.
    #[test]
    fn window_below_tip_reads_the_missing_mask() {
        let tip = 10u64;
        // nonce 8 is two below the tip: position 2.
        assert!(!identity_nonce_proves_unused(tip, 8), "no mask: merged");
        assert!(identity_nonce_proves_unused(tip | missing_bit(2), 8));
        assert!(
            !identity_nonce_proves_unused(tip | missing_bit(3), 8),
            "a different position's bit must not count"
        );
    }

    /// Older than the window: cannot be told apart from merged, so possibly used.
    #[test]
    fn older_than_window_is_possibly_used() {
        let tip = 100u64;
        assert!(!identity_nonce_proves_unused(
            tip,
            tip - MAX_MISSING_IDENTITY_REVISIONS - 1
        ));
        assert!(identity_nonce_proves_unused(
            tip | missing_bit(MAX_MISSING_IDENTITY_REVISIONS),
            tip - MAX_MISSING_IDENTITY_REVISIONS
        ));
    }
}

/// Map a `ShieldFromIdentity` builder failure. The builder signs with the
/// identity signer, so a structured key-unavailable completion (the reserved
/// marker at position 0 of a `ProtocolError::Generic` payload) can surface
/// here; it is preserved verbatim under [`PlatformWalletError::Sdk`] so the
/// FFI boundary restores code 31 (`ErrorSigningKeyUnavailable`) instead of
/// flattening it into the generic build error. Every other failure keeps the
/// protocol error's own rendering as a `ShieldedBuildError`.
fn map_shield_from_identity_build_error(error: ProtocolError) -> PlatformWalletError {
    preserve_signer_key_unavailable_or(dash_sdk::Error::Protocol(error), |e| match e {
        dash_sdk::Error::Protocol(protocol_error) => {
            PlatformWalletError::ShieldedBuildError(protocol_error.to_string())
        }
        other => PlatformWalletError::ShieldedBuildError(other.to_string()),
    })
}

#[cfg(test)]
mod shield_from_identity_build_error_tests {
    use super::*;
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;

    /// A key-unavailable signer completion keeps its structured shape so the
    /// FFI can restore code 31; the marker must sit at position 0.
    #[test]
    fn preserves_signer_key_unavailable_completion() {
        let error = ProtocolError::Generic(format!(
            "{SIGNER_KEY_UNAVAILABLE_PREFIX}transfer key not in keychain"
        ));
        let mapped = map_shield_from_identity_build_error(error);
        assert!(
            matches!(
                &mapped,
                PlatformWalletError::Sdk(dash_sdk::Error::Protocol(ProtocolError::Generic(s)))
                    if s.starts_with(SIGNER_KEY_UNAVAILABLE_PREFIX)
            ),
            "expected the signer completion preserved under Sdk, got {mapped:?}"
        );
    }

    /// Any other builder failure is a build error carrying the protocol
    /// error's own rendering (the SDK wrapper's "Protocol error:" prefix is
    /// not added), including a generic error that merely mentions the marker
    /// mid-string.
    #[test]
    fn stringifies_other_builder_failures() {
        let mapped = map_shield_from_identity_build_error(ProtocolError::Generic(
            "amount must be > 0".to_string(),
        ));
        assert!(
            matches!(
                &mapped,
                PlatformWalletError::ShieldedBuildError(s) if s == "Generic Error: amount must be > 0"
            ),
            "got {mapped:?}"
        );

        let mid_string = map_shield_from_identity_build_error(ProtocolError::Generic(format!(
            "signer failed: {SIGNER_KEY_UNAVAILABLE_PREFIX}not at position 0"
        )));
        assert!(
            matches!(mid_string, PlatformWalletError::ShieldedBuildError(_)),
            "a mid-string marker must not be promoted, got {mid_string:?}"
        );
    }
}

// -------------------------------------------------------------------------
// IdentityTopUpFromShieldedPool: shielded pool -> existing identity (Type 22)
// -------------------------------------------------------------------------

/// Top up an existing Platform identity's balance from `account`'s shielded
/// notes. Mirrors [`unshield`]: notes are reserved for `amount + fee` under
/// [`ShieldedFeeKind::IdentityTopUp`], the spend bundle binds the identity and
/// gross amount into its sighash, and the broadcast is redrive-safe. The
/// identity receives `amount` (the fee is carved from the value balance).
/// Waits for proven execution before marking notes spent.
///
/// Returns the identity's proof-attested post-top-up balance when the result
/// proof carried this identity (`None` only if the proven result had an
/// unexpected shape, which is logged). The caller persists it for a managed
/// identity (`PlatformWallet::shielded_identity_top_up_from_pool`).
#[allow(clippy::too_many_arguments)]
pub async fn identity_top_up_from_pool<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    identity_id: Identifier,
    amount: u64,
    prover: &P,
) -> Result<Option<Credits>, PlatformWalletError> {
    identity_top_up_from_pool_with_metadata(
        sdk,
        store,
        persister,
        wallet_id,
        keys,
        account,
        identity_id,
        amount,
        prover,
    )
    .await
    .map(|(result, _)| result)
}

#[allow(clippy::too_many_arguments)]
pub(in crate::wallet) async fn identity_top_up_from_pool_with_metadata<
    S: ShieldedStore,
    P: OrchardProver,
>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    identity_id: Identifier,
    amount: u64,
    prover: &P,
) -> Result<(Option<Credits>, ResponseMetadata), PlatformWalletError> {
    let views = keys.viewing_keys();
    let change_addr = default_orchard_address(&views)?;
    let id = SubwalletId::new(wallet_id, account);

    let (selected_notes, total_input, exact_fee, mut reservation) =
        reserve_unspent_notes(sdk, store, id, amount, 2, ShieldedFeeKind::IdentityTopUp).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        identity = %identity_id,
        "IdentityTopUpFromShieldedPool"
    );

    let mut pending_entry = None;
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
        let anchor_bytes = anchor.to_bytes();

        let (state_transition, fee_used) = build_identity_top_up_from_shielded_pool_transition(
            spends,
            identity_id,
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
        debug_assert_eq!(
            fee_used, exact_fee,
            "builder fee must match the reserved identity top up fee"
        );

        pending_entry = record_pending_activity(
            store,
            persister,
            wallet_id,
            id,
            &views,
            LiveEntryParams {
                kind: ShieldedActivityKind::IdentityTopUp {
                    identity_id: identity_id.to_buffer(),
                },
                direction: ShieldedDirection::Out,
                amount,
                fee: Some(fee_used),
                counterparty: Some(identity_id.to_vec()),
                memo: None,
                actions: shielded_actions(&state_transition),
                spent_notes: &selected_notes,
            },
        )
        .await;
        if arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await? {
            // Each reservation carries its anchor: a pruned anchor releases it.
            reservation.hand_over();
        }

        trace!("IdentityTopUpFromShieldedPool: state transition built, broadcasting...");
        // Type 22 is classified affected state: its nullifiers are shared with any
        // competing spend of the same notes and the identity's balance is a
        // snapshot, so the strict wait would reject every success. The snapshot
        // still proves the reserved notes are consumed and authenticates the
        // credited identity's balance; the shield, shield-from-identity and
        // identity-create paths accept the same class of outcome.
        broadcast_shielded_spend_with_redrive_with_metadata(
            sdk,
            store,
            id,
            &pending_entry,
            anchor_bytes,
            &selected_notes,
            &state_transition,
            "identity top up from shielded pool",
            SpendResultWait::AffectedState,
        )
        .await
    }
    .await;

    match result {
        Ok((proof, metadata)) => {
            // It executed: the outcome owns the notes, which it marks spent.
            reservation.hand_over();
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
            if let Err(e) = finalize_pending(store, persister, wallet_id, id, &selected_notes).await
            {
                warn!(
                    account,
                    error = %e,
                    "IdentityTopUpFromShieldedPool broadcast succeeded but local spent-state \
                     update failed; will heal on next sync"
                );
            }
            info!(account, credits = amount, identity = %identity_id, "IdentityTopUpFromShieldedPool broadcast succeeded");
            // The affected-state wait returns the spent nullifiers plus the
            // credited identity as a snapshot at the proof's block. The notes are
            // spent either way; the identity's proven balance is the balance at
            // that block, never derived from the requested amount.
            //
            // The row is confirmed here on purpose. The snapshot cannot tell this
            // top-up apart from a competing spend of the same notes, but only this
            // wallet's spending key can author one and the notes were reserved
            // locally, so spent nullifiers are treated as this transition's
            // execution, the same policy the transfer, unshield and withdrawal
            // paths apply. Until the protocol stores a transition-bound receipt
            // (a follow-up that would make every nullifier-spend family
            // execution-proving), a second device on the same seed can show a
            // confirmed row for a top-up its sibling displaced; immediate
            // confirmation was chosen over a pending row for that edge case.
            let proven_balance = match proof {
                StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers(proven, _)
                    if proven.id() == identity_id =>
                {
                    Some(proven.balance())
                }
                other => {
                    warn!(
                        account,
                        identity = %identity_id,
                        result = ?other,
                        "IdentityTopUpFromShieldedPool proof did not carry the credited \
                         identity; managed balance left for the next identity refresh"
                    );
                    None
                }
            };
            Ok((proven_balance, metadata))
        }
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => {
            // It may have executed: the reservation stays, released by sync.
            reservation.hand_over();
            Err(e)
        }
        Err(e) => {
            // It did not execute: the guard owns the notes until they are
            // released, so a drop on the way still releases them.
            reservation.reclaim();
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
            reservation.cancel().await;
            Err(e)
        }
    }
}

// -------------------------------------------------------------------------
// PrivateDocumentCreateFromShieldedPool: shielded pool -> entries of a
// private document type's store (Type 29)
// -------------------------------------------------------------------------

/// Append `entries` to the store of the private document type
/// `document_type_name` of `data_contract_id` (`PrivateDocumentCreateFromShieldedPool`,
/// type 29), paying the fee from `account`'s notes, the change going back to
/// the account. Nothing but the fee leaves the pool. Every entry must take the
/// type's `entrySize`, as `platform_encryption::seal_private_document_entry`
/// builds them.
///
/// Waits for the spent nullifiers to be proven before marking the notes
/// spent, and returns the metadata of that proof. `payment_started` is
/// marked right before the transition is broadcast.
#[allow(clippy::too_many_arguments)]
pub(in crate::wallet) async fn private_document_create_from_pool_with_metadata<
    S: ShieldedStore,
    P: OrchardProver,
>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    keys: &OrchardKeySet,
    account: u32,
    data_contract_id: Identifier,
    document_type_name: &str,
    entries: Vec<BinaryData>,
    private_contact: Option<PrivateContactPayment>,
    payment_started: &PaymentStarted,
    prover: &P,
) -> Result<ResponseMetadata, PlatformWalletError> {
    let views = keys.viewing_keys();
    let change_addr = default_orchard_address(&views)?;
    let id = SubwalletId::new(wallet_id, account);
    let fee_kind = ShieldedFeeKind::PrivateDocumentCreate {
        entry_count: entries.len(),
        entry_size: entries.first().map_or(0, |entry| entry.len() as u32),
    };

    let (selected_notes, total_input, exact_fee, reservation) =
        reserve_notes_covering(sdk, store, id, 0, 2, fee_kind).await?;

    info!(
        account,
        fee = exact_fee,
        entries = entries.len(),
        inputs = selected_notes.len(),
        total_input,
        data_contract = %data_contract_id,
        document_type = document_type_name,
        "PrivateDocumentCreateFromShieldedPool"
    );

    // The journal records of a private contact request payment, keyed by
    // these commitments, are discharged here only when the transition never
    // left the wallet.
    let journaled = private_contact.map(|_| {
        entries
            .iter()
            .filter_map(|entry| {
                entry
                    .as_slice()
                    .get(..32)
                    .and_then(|commitment| <[u8; 32]>::try_from(commitment).ok())
            })
            .collect::<Vec<[u8; 32]>>()
    });
    let built = async {
        let (spends, anchor) = extract_spends_and_anchor(sdk, store, &selected_notes).await?;
        let (state_transition, fee_used) =
            build_private_document_create_from_shielded_pool_transition(
                spends,
                data_contract_id,
                document_type_name.to_string(),
                entries,
                &change_addr,
                &keys.full_viewing_key,
                &keys.spend_auth_key,
                anchor,
                prover,
                sdk.version(),
            )
            .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;
        debug_assert_eq!(
            fee_used, exact_fee,
            "builder fee must match the reserved private document create fee"
        );
        Ok::<_, PlatformWalletError>((state_transition, fee_used, anchor.to_bytes()))
    }
    .await;
    let (state_transition, fee_used, anchor_bytes) = match built {
        Ok(built) => built,
        Err(e) => {
            // Nothing was saved or broadcast.
            reservation.cancel().await;
            return Err(e);
        }
    };
    // The spend's only outputs are change, so it records as an own spend
    // with its fee and no counterparty.
    let pending_entry = record_pending_activity(
        store,
        persister,
        wallet_id,
        id,
        &views,
        LiveEntryParams {
            kind: ShieldedActivityKind::ShieldedSpend,
            direction: ShieldedDirection::Out,
            amount: 0,
            fee: Some(fee_used),
            counterparty: None,
            memo: None,
            actions: shielded_actions(&state_transition),
            spent_notes: &selected_notes,
        },
    )
    .await;
    // Type 29 proves its spent nullifiers as a snapshot, which a competing
    // spend of the same notes shares, as type 22's does: the affected-state
    // wait accepts it (see `identity_top_up_from_pool_with_metadata`).
    submit_private_document_payment(
        PrivateDocumentSubmission {
            store,
            persister,
            wallet_id,
            id,
            pending_entry,
            selected_notes: &selected_notes,
            reservation,
            state_transition,
            anchor: anchor_bytes,
            private_contact,
            journaled: journaled.as_deref(),
            payment_started,
        },
        |state_transition| async move {
            broadcast_shielded_spend_classified(
                sdk,
                &state_transition,
                "private document create from shielded pool",
                SpendResultWait::AffectedState,
            )
            .await
        },
    )
    .await
}

/// A built private document create, and everything submitting it takes
/// ([`submit_private_document_payment`]).
struct PrivateDocumentSubmission<'a, S: ShieldedStore> {
    store: &'a Arc<RwLock<S>>,
    persister: Option<&'a WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    /// The spend's activity entry, recorded pending; it keys its records.
    pending_entry: Option<super::activity::ShieldedActivityEntry>,
    selected_notes: &'a [ShieldedNote],
    reservation: NoteReservation<S>,
    state_transition: StateTransition,
    anchor: [u8; 32],
    private_contact: Option<PrivateContactPayment>,
    journaled: Option<&'a [[u8; 32]]>,
    payment_started: &'a PaymentStarted,
}

/// Record, save and broadcast a built private document create, then settle
/// its notes and saved record ([`settle_private_document_payment`]).
/// `broadcast` sends the transition and waits for its result: the production
/// flow passes the classified broadcast, a test intercepts it.
///
/// Saved before the broadcast, not after an unconfirmed wait: a process that
/// exits once the node admitted the transition, before the wait returns,
/// still leaves the record a restart restores the request's reservation from,
/// and the redrive resolves. The payment is marked started once the record is
/// saved, with nothing awaited in between, and the notes are handed over only
/// then: a send dropped while the save waits for the store has saved and
/// broadcast nothing, and its guard releases the notes. A save that fails
/// stops the payment before the broadcast.
async fn submit_private_document_payment<S, B, Fut>(
    submission: PrivateDocumentSubmission<'_, S>,
    broadcast: B,
) -> Result<ResponseMetadata, PlatformWalletError>
where
    S: ShieldedStore,
    B: FnOnce(StateTransition) -> Fut,
    Fut: Future<Output = Result<(StateTransitionProofResult, ResponseMetadata), SpendFailure>>,
{
    let PrivateDocumentSubmission {
        store,
        persister,
        wallet_id,
        id,
        pending_entry,
        selected_notes,
        mut reservation,
        state_transition,
        anchor,
        private_contact,
        journaled,
        payment_started,
    } = submission;
    let result = async {
        let anchored =
            arm_pending_release(store, id, anchor, &pending_entry, selected_notes).await?;
        arm_redrive_before_broadcast(
            store,
            id,
            pending_entry.as_ref().map(|entry| entry.id),
            anchor,
            selected_notes.iter().map(|note| note.nullifier).collect(),
            &state_transition,
            "private document create from shielded pool",
            private_contact,
            Some(payment_started),
        )
        .await?;
        if anchored {
            // Each reservation carries its anchor: a pruned anchor releases it.
            reservation.hand_over();
        }
        trace!("PrivateDocumentCreateFromShieldedPool: state transition saved, broadcasting...");
        broadcast(state_transition).await
    }
    .await;

    settle_private_document_payment(
        store,
        persister,
        wallet_id,
        id,
        &pending_entry,
        reservation,
        selected_notes,
        journaled,
        payment_started,
        result.map(|(_proof, metadata)| metadata),
    )
    .await
}

/// Settle a private document payment's notes and saved record once its
/// broadcast and result wait returned `result`, as
/// [`private_document_create_from_pool_with_metadata`] does.
///
/// A payment that executed marks its notes spent; one that may have executed
/// (unconfirmed) keeps them reserved and its saved record, which the redrive
/// pass resolves; one that did not execute releases them, and its record with
/// them. The private payment journal's records of a private contact request
/// payment (`journaled`, its entries' commitments) outlive all of that: they
/// are discharged here only when the transition never left the wallet
/// ([`SpendFailure::never_delivered`]), as nothing else can then execute it,
/// and `payment_started` tells the caller so
/// ([`PaymentStarted::note_never_delivered`]). Otherwise they stay until the
/// entry is read back.
#[allow(clippy::too_many_arguments)]
async fn settle_private_document_payment<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    id: SubwalletId,
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    mut reservation: NoteReservation<S>,
    selected_notes: &[ShieldedNote],
    journaled: Option<&[[u8; 32]]>,
    payment_started: &PaymentStarted,
    result: Result<ResponseMetadata, SpendFailure>,
) -> Result<ResponseMetadata, PlatformWalletError> {
    match result {
        Ok(metadata) => {
            // It executed: the outcome owns the notes, which it marks spent.
            reservation.hand_over();
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                pending_entry,
                ShieldedActivityStatus::Confirmed,
                None,
            )
            .await;
            if let Err(e) = finalize_pending(store, persister, wallet_id, id, selected_notes).await
            {
                warn!(
                    account = id.account_index,
                    error = %e,
                    "PrivateDocumentCreateFromShieldedPool broadcast succeeded but local \
                     spent-state update failed; will heal on next sync"
                );
            }
            info!(
                account = id.account_index,
                "PrivateDocumentCreateFromShieldedPool broadcast succeeded"
            );
            Ok(metadata)
        }
        Err(SpendFailure {
            error: e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. },
            ..
        }) => {
            // It may have executed: the reservation stays, released by sync.
            reservation.hand_over();
            Err(e)
        }
        Err(SpendFailure {
            error,
            never_delivered,
        }) => {
            // It did not execute: the guard owns the notes until they are
            // released, so a drop on the way still releases them.
            reservation.reclaim();
            record_activity_status(
                store,
                persister,
                wallet_id,
                id,
                pending_entry,
                ShieldedActivityStatus::Failed,
                None,
            )
            .await;
            if never_delivered {
                if let Some(commitments) = journaled {
                    if discharge_journal_records(store, persister, wallet_id, commitments).await {
                        payment_started.note_never_delivered();
                    }
                }
            }
            reservation.cancel().await;
            Err(error)
        }
    }
}

/// Discharge `wallet_id`'s journal records of `commitments`, whose payment's
/// transition never left the wallet, noting each one discharged on
/// `persister` so a restoration that read it before skips it. Returns whether
/// every one is gone; a failure leaves the records, which over-holds their
/// pairs.
async fn discharge_journal_records<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    persister: Option<&WalletPersister>,
    wallet_id: WalletId,
    commitments: &[[u8; 32]],
) -> bool {
    let mut store = store.write().await;
    let mut discharged = Vec::with_capacity(commitments.len());
    for commitment in commitments {
        match store.discharge_private_payment(wallet_id, commitment) {
            Ok(_) => discharged.push(*commitment),
            Err(e) => warn!(
                error = %e,
                "a never-delivered private payment's journal record was not discharged; its \
                 pair stays held"
            ),
        }
    }
    let every_one = discharged.len() == commitments.len();
    if let Some(persister) = persister {
        persister.note_discharged_private_entries(discharged);
    }
    every_one
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
    let (selected_notes, total_input, exact_fee, mut reservation) =
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
        if arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await? {
            // Each reservation carries its anchor: a pruned anchor releases it.
            reservation.hand_over();
        }

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
            SpendResultWait::ExecutionProved,
        )
        .await
    }
    .await;

    match result {
        Ok(_) => {
            // It executed: the outcome owns the notes, which it marks spent.
            reservation.hand_over();
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
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => {
            // It may have executed: the reservation stays, released by sync.
            reservation.hand_over();
            Err(e)
        }
        Err(e) => {
            // It did not execute: the guard owns the notes until they are
            // released, so a drop on the way still releases them.
            reservation.reclaim();
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
            reservation.cancel().await;
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
    let (selected_notes, total_input, exact_fee, mut reservation) =
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
        if arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await? {
            // Each reservation carries its anchor: a pruned anchor releases it.
            reservation.hand_over();
        }

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
            SpendResultWait::ExecutionProved,
        )
        .await
    }
    .await;

    match result {
        Ok(_) => {
            // It executed: the outcome owns the notes, which it marks spent.
            reservation.hand_over();
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
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => {
            // It may have executed: the reservation stays, released by sync.
            reservation.hand_over();
            Err(e)
        }
        Err(e) => {
            // It did not execute: the guard owns the notes until they are
            // released, so a drop on the way still releases them.
            reservation.reclaim();
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
            reservation.cancel().await;
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
    let (selected_notes, total_input, exact_fee, mut reservation) =
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
        if arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await? {
            // Each reservation carries its anchor: a pruned anchor releases it.
            reservation.hand_over();
        }

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
            SpendResultWait::ExecutionProved,
        )
        .await
    }
    .await;

    match result {
        Ok(_) => {
            // It executed: the outcome owns the notes, which it marks spent.
            reservation.hand_over();
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
        Err(e @ PlatformWalletError::ShieldedSpendUnconfirmed { .. }) => {
            // It may have executed: the reservation stays, released by sync.
            reservation.hand_over();
            Err(e)
        }
        Err(e) => {
            // It did not execute: the guard owns the notes until they are
            // released, so a drop on the way still releases them.
            reservation.reclaim();
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
            reservation.cancel().await;
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
    let (selected_notes, total_input, predicted_fee, mut reservation) =
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
        if arm_pending_release(store, id, anchor_bytes, &pending_entry, &selected_notes).await? {
            // Each reservation carries its anchor: a pruned anchor releases it.
            reservation.hand_over();
        }

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

        // Broadcast (relay-ACK only). Only a transition that never left the wallet
        // (`BroadcastDelivery::NeverDelivered`: refused before anything was sent) maps to
        // `ShieldedBroadcastFailed`,
        // which lets the outer match release the reservation. Every other failure may have
        // delivered it and falls through to the result wait below, whose ambiguous arm owns the
        // fetch-by-derived-id fallback — so the in-flight tx gets confirmed (or held as
        // unconfirmed) instead of being reported as failed.
        match broadcast_tracking_delivery(sdk, &st).await {
            BroadcastDelivery::Admitted => {}
            BroadcastDelivery::NeverDelivered(e) => {
                return Err(PlatformWalletError::ShieldedBroadcastFailed(e.to_string()));
            }
            BroadcastDelivery::MaybeDelivered(e) => {
                warn!(
                    derived_id = %identity_id,
                    error = %e,
                    "IdentityCreateFromShieldedPool: broadcast may have been delivered — \
                     falling through to the result wait"
                );
            }
        }

        // Wait for proven execution. Every failure is AMBIGUOUS: the broadcast reached an
        //   endpoint and the transition may have executed even though we couldn't fetch or
        //   verify its result proof. That includes a `StateTransitionBroadcastError` with a
        //   consensus `cause`: the endpoint that reports it is unauthenticated, no proof or quorum
        //   signature backs it, and one that kept or forwarded the transition can answer with any
        //   verdict. It also includes cause-less ones, as DAPI encodes its own wait-side failures
        //   (timeouts, internal errors) that way. Fall back to fetching the identity by its
        //   pre-derived id before deciding it is unconfirmed.
        // An identity-create-from-shielded-pool proof authenticates the spent
        // nullifiers and resulting identity as a snapshot; it cannot bind the
        // complete Orchard request, so accept the affected-state outcome.
        let proof_result = match st
            .wait_for_affected_state::<StateTransitionProofResult>(sdk, None)
            .await
        {
            Ok(result) => result,
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
            // It executed: the outcome owns the notes, which it marks spent.
            reservation.hand_over();
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
        // found by a direct fetch. Do NOT release the notes here: `pending_nullifiers` is in-memory
        // only (see `SubwalletState`, "never persisted; the next sync after a crash reconciles"),
        // and `mark_spent` during nullifier sync clears matching reservations. So if the transition
        // actually executed, the next sync promotes these notes to spent; if it truly never landed,
        // an app restart drops the in-memory reservation and frees them. Releasing them now would
        // invite double-spend attempts against notes that may already be consumed on chain — the
        // very hazard this variant exists to prevent. The activity row likewise stays `Pending`
        // (a later scan that finds the identity's change note flips it to Confirmed).
        Err(e @ PlatformWalletError::ShieldedBroadcastUnconfirmed { .. }) => {
            // It may have executed: the reservation stays, released by sync.
            reservation.hand_over();
            Err(e)
        }
        Err(e) => {
            if error_releases_note_reservation(&e) {
                // It did not execute: the guard owns the notes until they are
                // released, so a drop on the way still releases them.
                reservation.reclaim();
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
                reservation.cancel().await;
            } else {
                // It may have executed: the reservation stays, released by sync.
                reservation.hand_over();
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
/// broadcast success) or [`NoteReservation::cancel`] (on failure) so the
/// reservation is always released; until they hand it over
/// ([`NoteReservation::hand_over`]), the returned guard releases it when
/// the spend is dropped.
async fn reserve_unspent_notes<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    amount: u64,
    outputs: usize,
    fee_kind: ShieldedFeeKind,
) -> Result<(Vec<ShieldedNote>, u64, u64, NoteReservation<S>), PlatformWalletError> {
    // All amount-based spends reject zero before waiting on a concurrent sync's store lock.
    ensure_positive_amount(amount)?;
    reserve_notes_covering(sdk, store, id, amount, outputs, fee_kind).await
}

/// Select and reserve, in one write-locked critical section, notes covering
/// `amount` plus the fee of `fee_kind`, which [`reserve_unspent_notes`] does
/// for a positive amount. A spend paying only its fee (an
/// `PrivateDocumentCreateFromShieldedPool`) reserves with an `amount` of 0.
/// Callers must pair this with [`finalize_pending`] / [`NoteReservation::cancel`], and
/// hand the guard over once they own the reservation.
async fn reserve_notes_covering<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    amount: u64,
    outputs: usize,
    fee_kind: ShieldedFeeKind,
) -> Result<(Vec<ShieldedNote>, u64, u64, NoteReservation<S>), PlatformWalletError> {
    let mut guard = store.write().await;
    let unspent = guard
        .get_unspent_notes(id)
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    let (selected, total_input, exact_fee) =
        select_notes_with_fee(&unspent, amount, outputs, fee_kind, sdk.version())?.into_owned();
    let reservation = mark_notes_pending(&mut *guard, store, id, &selected)?;
    Ok((selected, total_input, exact_fee, reservation))
}

/// Mark `notes` pending under the held store lock, returning the guard that
/// releases them unless handed over. A failure part way releases those
/// already marked.
fn mark_notes_pending<S: ShieldedStore>(
    locked: &mut S,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    notes: &[ShieldedNote],
) -> Result<NoteReservation<S>, PlatformWalletError> {
    let reservation = next_note_reservation();
    let mut marked = Vec::with_capacity(notes.len());
    for note in notes {
        if let Err(e) = locked.mark_pending_by(id, &note.nullifier, reservation) {
            release_reserved_notes(locked, id, &marked, reservation);
            return Err(PlatformWalletError::ShieldedStoreError(e.to_string()));
        }
        marked.push(note.nullifier);
    }
    Ok(NoteReservation {
        store: Arc::clone(store),
        id,
        nullifiers: marked,
        reservation,
        handed_over: false,
    })
}

/// A spend attempt's reservation id, unique in the process: what its note
/// reservations are owned by ([`ShieldedStore::mark_pending_by`]).
fn next_note_reservation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, AtomicOrdering::Relaxed)
}

/// Notes one spend reserved ([`ShieldedStore::mark_pending`]), released when
/// dropped before the spend hands them over ([`Self::hand_over`]): a spend
/// future dropped before its notes have an owner that frees them (an anchor
/// on each reservation, which a pruned anchor releases, or the outcome
/// handling that finalizes or cancels them) leaves them spendable again, as
/// [`NoteReservation::cancel`] would, rather than excluded for the rest of the
/// session. The release takes the store lock when it is free, and otherwise
/// runs on a spawned task once it frees; outside a runtime the notes stay
/// reserved until restart.
pub(super) struct NoteReservation<S: ShieldedStore> {
    store: Arc<RwLock<S>>,
    id: SubwalletId,
    nullifiers: Vec<[u8; 32]>,
    /// The attempt that owns the reservations: a release clears a note
    /// only while it still holds it, so a cleanup that runs after a purge,
    /// a rebind or a wallet's replacement never releases a newer attempt's
    /// notes or deletes its records.
    reservation: u64,
    handed_over: bool,
}

impl<S: ShieldedStore> NoteReservation<S> {
    /// The spend owns the reservation from now on.
    pub(super) fn hand_over(&mut self) {
        self.handed_over = true;
    }

    /// Own the reservation again, for a spend now known not to have executed: dropped from
    /// here on, the guard releases the notes, whatever owned them before.
    pub(super) fn reclaim(&mut self) {
        self.handed_over = false;
    }

    /// Release the notes now, for a spend that did not execute.
    /// The guard owns them until the release is done: a spend dropped while it waits for the
    /// store lock still releases them on drop.
    pub(super) async fn cancel(mut self) {
        self.handed_over = false;
        let mut store = self.store.write().await;
        release_reserved_notes(&mut *store, self.id, &self.nullifiers, self.reservation);
        self.nullifiers.clear();
    }
}

impl<S: ShieldedStore> Drop for NoteReservation<S> {
    fn drop(&mut self) {
        if self.handed_over || self.nullifiers.is_empty() {
            return;
        }
        let (id, reservation) = (self.id, self.reservation);
        let nullifiers = std::mem::take(&mut self.nullifiers);
        if let Ok(mut store) = self.store.try_write() {
            release_reserved_notes(&mut *store, id, &nullifiers, reservation);
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            warn!(
                "a dropped spend's notes were reserved outside a runtime; they stay until restart"
            );
            return;
        };
        let store = Arc::clone(&self.store);
        runtime.spawn(async move {
            let mut store = store.write().await;
            release_reserved_notes(&mut *store, id, &nullifiers, reservation);
        });
    }
}

/// Clear the reservations of `nullifiers` the spend attempt `reservation`
/// still holds ([`ShieldedStore::clear_pending_reserved_by`]).
fn release_reserved_notes<S: ShieldedStore>(
    store: &mut S,
    id: SubwalletId,
    nullifiers: &[[u8; 32]],
    reservation: u64,
) {
    for nullifier in nullifiers {
        if let Err(e) = store.clear_pending_reserved_by(id, nullifier, reservation) {
            warn!(
                error = %e,
                "releasing a dropped spend's note failed; the next note scan will reconcile"
            );
        }
    }
}

/// Exact-equality sibling of [`reserve_unspent_notes`] for
/// `IdentityCreateFromShieldedPool`: select + reserve notes covering exactly `denomination`
/// (the fee is metered FROM the denomination, not added to the target) in one write-locked
/// critical section, gating on `denomination > predicted_fee` via
/// [`select_notes_for_denomination`]. Returns the selected notes, total input value, and the
/// predicted fee. Callers must pair this with [`finalize_pending`] / [`NoteReservation::cancel`], and
/// hand the guard over once they own the reservation.
async fn reserve_unspent_notes_for_denomination<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    denomination: u64,
    min_actions: usize,
    num_keys: usize,
) -> Result<(Vec<ShieldedNote>, u64, u64, NoteReservation<S>), PlatformWalletError> {
    // Reject zero or any other non-member before waiting on a concurrent sync's store lock.
    ensure_allowed_denomination(denomination, sdk.version())?;

    let mut guard = store.write().await;
    let unspent = guard
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
    let reservation = mark_notes_pending(&mut *guard, store, id, &selected)?;
    Ok((selected, total_input, predicted_fee, reservation))
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

/// Record the recorded `anchor` the spend was built against and the
/// linked activity entry on every selected note's reservation, so a
/// spend that ends broadcast-accepted-but-unconfirmed can be released
/// on a later sync once that anchor is pruned from Platform's recorded
/// set (see `NetworkShieldedCoordinator::release_stranded_spends`).
///
/// No-op when no activity entry was recorded (an output-less bundle —
/// unreachable for our own builders, which always carry a visible
/// output). A success (`finalize_pending`) or a definite failure
/// (`NoteReservation::cancel`) removes the entry, so only an ambiguous
/// unconfirmed outcome leaves it carrying the anchor.
///
/// Returns whether every reservation now carries the anchor, so a pruned
/// anchor releases each. A store write that fails stops the spend before it
/// is broadcast ([`PlatformWalletError::ShieldedStoreError`]): the caller's
/// reservation guard still owns the notes, so they are released, rather
/// than a spend that may execute leaving reservations no pruned anchor
/// frees.
async fn arm_pending_release<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    anchor: [u8; 32],
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    notes: &[ShieldedNote],
) -> Result<bool, PlatformWalletError> {
    let Some(entry) = pending_entry else {
        return Ok(false);
    };
    let mut store = store.write().await;
    for note in notes {
        store
            .set_pending_spend(id, &note.nullifier, anchor, entry.id)
            .map_err(|e| {
                PlatformWalletError::ShieldedStoreError(format!(
                    "the spend's note reservation could not carry its anchor, so it is not \
                     broadcast: {e}"
                ))
            })?;
    }
    Ok(true)
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
    wait: SpendResultWait,
) -> Result<StateTransitionProofResult, PlatformWalletError> {
    broadcast_shielded_spend_with_redrive_with_metadata(
        sdk,
        store,
        id,
        pending_entry,
        anchor,
        notes,
        state_transition,
        operation,
        wait,
    )
    .await
    .map(|(result, _)| result)
}

#[allow(clippy::too_many_arguments)]
async fn broadcast_shielded_spend_with_redrive_with_metadata<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    pending_entry: &Option<super::activity::ShieldedActivityEntry>,
    anchor: [u8; 32],
    notes: &[ShieldedNote],
    state_transition: &StateTransition,
    operation: &'static str,
    wait: SpendResultWait,
) -> Result<(StateTransitionProofResult, ResponseMetadata), PlatformWalletError> {
    let result = broadcast_shielded_spend(sdk, state_transition, operation, wait).await;
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

/// Save the re-drivable record of a spend durably before it is broadcast,
/// as identity-funded shields arm theirs: an exit after the node admitted
/// the transition, before its result returns, leaves the record. The record
/// is keyed by the spend's `activity_id` and carries its `nullifiers`, and
/// the private contact request the spend pays for when it does. A spend
/// paying for one is a private document create (type 29), and the private
/// payment journal keeps a record of each entry it appends, read from the
/// transition, in the same durable write ([`ShieldedStore::arm_private_contact_payment`]).
/// A spend whose record cannot be saved, or whose entries do not read, is
/// refused before anything is broadcast, as the unknown outcome could then
/// not be recovered. `payment_started`, when given, is marked right after
/// the record is saved, under the same store lock: dropped while it waits
/// for the lock, the spend leaves neither a record nor a mark.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn arm_redrive_before_broadcast<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    activity_id: Option<[u8; 32]>,
    anchor: [u8; 32],
    nullifiers: Vec<[u8; 32]>,
    state_transition: &StateTransition,
    operation: &'static str,
    private_contact: Option<PrivateContactPayment>,
    payment_started: Option<&PaymentStarted>,
) -> Result<(), PlatformWalletError> {
    use dpp::serialization::PlatformSerializable;

    let activity_id = activity_id.ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError(format!(
            "{operation}: no activity entry keys the record of this spend, so it is not \
             broadcast"
        ))
    })?;
    let st_bytes = state_transition.serialize_to_bytes().map_err(|e| {
        PlatformWalletError::ShieldedBuildError(format!(
            "{operation}: the transition does not serialize for its record, so it is not \
             broadcast: {e}"
        ))
    })?;
    let journal = match private_contact {
        Some(payment) => Some((
            payment,
            private_document_commitments(state_transition).ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "{operation}: the entries this private contact request payment appends do \
                     not read, so it is not broadcast"
                ))
            })?,
        )),
        None => None,
    };
    let redrive = PendingRedrive {
        activity_id,
        anchor,
        nullifiers,
        st_bytes,
        attempts: 0,
        identity_nonce_finalized: false,
        identity_user_abandoned: false,
    };
    let mut store = store.write().await;
    match journal {
        Some((payment, commitments)) => {
            store.arm_private_contact_payment(id, redrive, payment, &commitments)
        }
        None => store.arm_redrive(id, redrive),
    }
    .map_err(|e| {
        PlatformWalletError::ShieldedStoreError(format!(
            "{operation}: the record of this spend could not be saved, so it is not broadcast: \
             {e}"
        ))
    })?;
    if let Some(payment_started) = payment_started {
        payment_started.mark();
    }
    Ok(())
}

/// Discharge `wallet_id`'s journal records of the private contact request
/// entries a private sync read back and recorded durably, as `persister`
/// noted them (`WalletPersister::note_read_back_private_entries`). One whose
/// discharge fails is noted again for the next call.
pub(crate) fn discharge_read_back_private_payments<S: ShieldedStore>(
    store: &mut S,
    persister: &WalletPersister,
    wallet_id: WalletId,
) {
    let mut kept = Vec::new();
    let mut discharged = Vec::new();
    for commitment in persister.take_read_back_private_entries() {
        match store.discharge_private_payment(wallet_id, &commitment) {
            Ok(_) => discharged.push(commitment),
            Err(e) => {
                warn!(
                    error = %e,
                    "a read-back private payment's journal record was not discharged; retried \
                     later"
                );
                kept.push(commitment);
            }
        }
    }
    persister.note_discharged_private_entries(discharged);
    if !kept.is_empty() {
        persister.note_read_back_private_entries(kept);
    }
}

/// The commitments of the entries the private document create (type 29)
/// `state_transition` appends, each entry's first 32 bytes; `None` for
/// another transition, one appending no entry, or an entry shorter than its
/// commitment.
pub(crate) fn private_document_commitments(
    state_transition: &StateTransition,
) -> Option<Vec<[u8; 32]>> {
    use dpp::state_transition::private_document_create_from_shielded_pool_transition::accessors::PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0;

    let StateTransition::PrivateDocumentCreateFromShieldedPool(transition) = state_transition
    else {
        return None;
    };
    if transition.entries().is_empty() {
        return None;
    }
    transition
        .entries()
        .iter()
        .map(|entry| {
            entry
                .as_slice()
                .get(..32)
                .and_then(|commitment| <[u8; 32]>::try_from(commitment).ok())
        })
        .collect()
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
        identity_nonce_finalized: false,
        identity_user_abandoned: false,
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
    /// Any other outcome, a consensus verdict included: the original's
    /// broadcast was ambiguous, so a refusal of the re-broadcast does not
    /// prove it can never execute.
    Inconclusive,
}

fn classify_redrive_broadcast(result: &Result<(), dash_sdk::Error>) -> RedriveBroadcastOutcome {
    match result {
        Ok(()) => RedriveBroadcastOutcome::Accepted,
        Err(e) if is_nullifier_already_spent(e) => RedriveBroadcastOutcome::AlreadyExecuted,
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

/// Re-broadcast every unresolved ShieldFromIdentity transition for `wallet_id`.
/// These records have no nullifiers or spending anchor, so they must not depend
/// on the note-spend stranded-release pass. Exact-byte retries remain safe even
/// after the ordinary spend attempt budget is exhausted: the fixed identity
/// nonce prevents the same transition from debiting twice, while continuing
/// retries lets a wallet recover after switching away from a failing endpoint.
pub(super) async fn redrive_pending_identity_shields<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    wallet_id: WalletId,
    cancel: &CancellationToken,
) {
    use dpp::serialization::PlatformDeserializableTrusted;
    use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;

    let redrives = match store.read().await.pending_redrives_for_wallet(wallet_id) {
        Ok(redrives) => redrives,
        Err(e) => {
            warn!(error = %e, "identity redrive: guard lookup failed");
            return;
        }
    };
    for (id, redrive) in redrives {
        // A stop ends the pass between records: no new request starts
        // once cancelled. A request already on the wire is left to
        // finish on its own (its outcome is handled conservatively).
        if cancel.is_cancelled() {
            debug!("identity redrive: pass cancelled; remaining guards wait for the next pass");
            return;
        }
        if !redrive.nullifiers.is_empty()
            || redrive.identity_nonce_finalized
            || redrive.identity_user_abandoned
        {
            continue;
        }
        let state_transition =
            match StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes) {
                Ok(state_transition @ StateTransition::ShieldFromIdentity(_)) => state_transition,
                _ => {
                    warn!("identity redrive: unreadable debit; retaining its retry guard");
                    continue;
                }
            };
        let StateTransition::ShieldFromIdentity(identity_transition) = &state_transition else {
            continue;
        };
        // The nonce read is the one wait here a stop can cut short: it
        // is read-only, so dropping it mid-flight loses nothing.
        let Some(fetched) = cancel
            .run_until_cancelled(IdentityNonceFetcher::fetch(
                sdk,
                identity_transition.identity_id(),
            ))
            .await
        else {
            debug!("identity redrive: pass cancelled during the nonce read");
            return;
        };
        if let Ok(fetched) = fetched {
            let result = {
                let mut store = store.write().await;
                reconcile_identity_shield_nonce(
                    &mut *store,
                    id,
                    redrive.activity_id,
                    fetched.map(|fetcher| fetcher.0),
                    identity_transition.nonce(),
                )
            };
            match result {
                Ok(true) => continue,
                Ok(false) => {}
                Err(e) => {
                    warn!(error = %e, "identity redrive: could not persist nonce finality");
                    continue;
                }
            }
        }
        // A host recovery decision may have archived this snapshot while the
        // nonce request was in flight. Do not initiate a new broadcast from it.
        // Already submitted network requests cannot be cancelled by this check.
        let still_active = store
            .read()
            .await
            .pending_redrives(id)
            .map(|current| {
                current.iter().any(|record| {
                    record.activity_id == redrive.activity_id
                        && record.st_bytes == redrive.st_bytes
                        && !record.identity_user_abandoned
                        && !record.identity_nonce_finalized
                })
            })
            .unwrap_or(false);
        if !still_active {
            continue;
        }
        // Last check before a broadcast starts; a broadcast already
        // submitted cannot be cancelled by this check.
        if cancel.is_cancelled() {
            debug!("identity redrive: pass cancelled before the re-broadcast");
            return;
        }
        // Unused or unverified state permits only the original signed bytes.
        // Even an apparently definitive rejection cannot release the guard:
        // this request, or an earlier relay, may deliver the debit afterward.
        if let Err(e) = state_transition.broadcast(sdk, None).await {
            debug!(error = %e, "identity redrive: original payment remains pending");
        }
        bump_redrive_attempts_logged(store, id, &redrive.activity_id).await;
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
/// - any other consensus verdict → inconclusive too: a node refusing the
///   re-broadcast does not show that no other node holds the original,
///   whose broadcast was ambiguous, so the notes stay reserved;
/// - pruned anchor / exhausted attempts → leave it to the
///   prune-backstop release pass, which releases the notes once the
///   anchor is pruned and the spend can never execute.
///
/// A record whose transition does not decode is kept, never dropped: it
/// may be the only evidence of a payment that can still execute. Its notes
/// stay reserved until the spend lands or the prune backstop releases them.
///
/// Runs after the scan's spent-note reconcile (a landed spend's record
/// was already dropped by the `mark_spent` hook, so anything still
/// armed here is genuinely unresolved).
pub(super) async fn redrive_pending_spends<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    id: SubwalletId,
    recorded: &std::collections::HashSet<[u8; 32]>,
    cancel: &CancellationToken,
) {
    use dpp::serialization::PlatformDeserializableTrusted;

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
        // A stop ends the pass between records: no new re-broadcast starts
        // once cancelled. One already submitted is left to finish and is
        // classified as usual — its outcome handling stays conservative.
        if cancel.is_cancelled() {
            debug!("redrive: pass cancelled; remaining records wait for the next pass");
            return;
        }
        // Empty-nullifier identity debits are wallet-wide and are driven by
        // `redrive_pending_identity_shields`, independently of spending anchors.
        if redrive.nullifiers.is_empty() {
            continue;
        }
        // A pruned anchor is the release pass's call, not ours; an
        // exhausted budget means we've said our three pieces.
        if !recorded.contains(&redrive.anchor) || redrive.attempts >= MAX_REDRIVE_ATTEMPTS {
            continue;
        }
        let st = match StateTransition::deserialize_from_bytes_trusted(&redrive.st_bytes) {
            Ok(st) => st,
            Err(e) => {
                warn!(
                    error = %e,
                    "redrive: stored transition failed to deserialize; keeping the record \
                     without re-broadcasting it (the prune backstop still frees the notes)"
                );
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
///   releases the note reservations via [`NoteReservation::cancel`];
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
/// How [`broadcast_shielded_spend`] waits for an already-broadcast spend's
/// result, mirroring the verifier's classification of the family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpendResultWait {
    /// The family is classified execution-proving (transfer, unshield,
    /// withdrawal): a snapshot outcome is an error.
    ExecutionProved,
    /// The proof only authenticates the affected keys' state at the proof's
    /// block (identity top-up: the spent nullifiers are shared with any
    /// competing spend of the same notes and the credited identity's balance
    /// is a snapshot). Accepted as that snapshot.
    AffectedState,
}

async fn broadcast_shielded_spend(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition: &StateTransition,
    operation: &'static str,
    wait: SpendResultWait,
) -> Result<(StateTransitionProofResult, ResponseMetadata), PlatformWalletError> {
    broadcast_shielded_spend_classified(sdk, state_transition, operation, wait)
        .await
        .map_err(|failure| failure.error)
}

/// A failed [`broadcast_shielded_spend_classified`]: the error it reports, and
/// whether the transition never left the wallet.
struct SpendFailure {
    error: PlatformWalletError,
    /// The broadcast proved nothing was delivered
    /// ([`BroadcastDelivery::NeverDelivered`]): nothing holds the transition's
    /// bytes but the wallet, so it can never execute.
    never_delivered: bool,
}

impl From<PlatformWalletError> for SpendFailure {
    fn from(error: PlatformWalletError) -> Self {
        Self {
            error,
            never_delivered: false,
        }
    }
}

/// [`broadcast_shielded_spend`], telling a failure whose transition never
/// left the wallet from one that may have reached the network.
async fn broadcast_shielded_spend_classified(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition: &StateTransition,
    operation: &'static str,
    wait: SpendResultWait,
) -> Result<(StateTransitionProofResult, ResponseMetadata), SpendFailure> {
    match broadcast_tracking_delivery(sdk, state_transition).await {
        BroadcastDelivery::Admitted => {}
        BroadcastDelivery::NeverDelivered(e) => {
            return Err(SpendFailure {
                error: definite_broadcast_failure(&e),
                never_delivered: true,
            });
        }
        BroadcastDelivery::MaybeDelivered(e) => {
            warn!(
                operation,
                error = %e,
                "Shielded spend broadcast may have been delivered; falling through to the \
                 result wait"
            );
        }
    }

    // The verifier's classification of the family decides the wait: a
    // proof-binding family rejects a snapshot outcome, an affected-state family
    // accepts it. Either way the proven result is returned so callers that
    // credit a known identity can apply its proof-attested balance.
    let waited = match wait {
        SpendResultWait::ExecutionProved => {
            state_transition
                .wait_for_response_with_metadata::<StateTransitionProofResult>(sdk, None)
                .await
        }
        SpendResultWait::AffectedState => {
            state_transition
                .wait_for_affected_state_with_metadata::<StateTransitionProofResult>(sdk, None)
                .await
        }
    };
    waited
        .map_err(|wait_err| classify_spend_wait_failure(operation, &wait_err))
        .map_err(SpendFailure::from)
}

/// The error a spend whose transition never left the wallet
/// ([`BroadcastDelivery::NeverDelivered`], refused before anything was sent)
/// is reported as: a failed broadcast. Its notes are released. Every
/// broadcast that reached an endpoint waits for its result instead, and
/// reads as unconfirmed when that wait fails.
fn definite_broadcast_failure(e: &dash_sdk::Error) -> PlatformWalletError {
    PlatformWalletError::ShieldedBroadcastFailed(e.to_string())
}

/// Classify a `wait_for_response` failure for an already-broadcast
/// shielded spend (see [`broadcast_shielded_spend`]): unconfirmed, whatever
/// it is, the notes staying reserved.
///
/// A consensus verdict on the wait ([`carries_consensus_rejection`]) is no
/// proof either: the endpoint that sends it is unauthenticated, no proof or
/// quorum signature backs it, and one that kept or forwarded the signed
/// transition can answer with any verdict. DAPI also encodes its own
/// wait-side failures (timeouts, internal errors; see
/// `build_wait_for_state_transition_error_response` in rs-dapi) as
/// `StateTransitionBroadcastError`s with EMPTY consensus data. A spend
/// reported failed while it may still execute could be paid again with other
/// notes; one that never executes is released by the redrive and prune
/// passes once its anchor is pruned.
fn classify_spend_wait_failure(
    operation: &'static str,
    wait_err: &dash_sdk::Error,
) -> PlatformWalletError {
    warn!(
        operation,
        error = %wait_err,
        rejected = carries_consensus_rejection(wait_err),
        "Shielded spend broadcast but its result is unconfirmed; leaving the note reservations \
         in place"
    );
    PlatformWalletError::ShieldedSpendUnconfirmed {
        operation,
        reason: wait_err.to_string(),
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
    use crate::wallet::identity::network::PaymentStarted;
    use crate::wallet::shielded::activity::ShieldedActivityEntry;
    use crate::wallet::shielded::store::InMemoryShieldedStore;
    use crate::wallet::shielded::FileBackedShieldedStore;
    use dash_sdk::dpp::dashcore::Network;
    use dash_sdk::dpp::prelude::CoreBlockHeight;
    use dash_sdk::error::ContextProviderError;
    use dash_sdk::error::StateTransitionBroadcastError;
    use dash_sdk::platform::ContextProvider;
    use dash_sdk::sdk::AddressList;
    use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::private_document_create_from_shielded_pool_transition::methods::PrivateDocumentCreateFromShieldedPoolTransitionMethodsV0;
    use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
    use dpp::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
    use dpp::version::PlatformVersion;
    use futures::{poll, FutureExt};
    use grovedb_commitment_tree::ProvingKey;
    use std::str::FromStr;
    use std::task::Poll;

    fn identity_shield_transition(identity_id: [u8; 32], nonce: u64) -> StateTransition {
        ShieldFromIdentityTransitionV0 {
            identity_id: identity_id.into(),
            amount: 1_000,
            actions: vec![],
            anchor: [1; 32],
            proof: vec![],
            binding_signature: [0; 64],
            nonce,
            user_fee_increase: 0,
            signature_public_key_id: 1,
            signature: BinaryData::new(vec![]),
        }
        .into()
    }

    #[tokio::test]
    async fn should_release_only_explicitly_abandoned_guard_and_never_redrive_it() {
        let wallet_id = [4; 32];
        let identity_id = [5; 32];
        let id = SubwalletId::new(wallet_id, 3);
        let mut store = InMemoryShieldedStore::new();
        let archived = PendingRedrive {
            activity_id: [6; 32],
            anchor: [1; 32],
            nullifiers: vec![],
            st_bytes: identity_shield_transition(identity_id, 7)
                .serialize_to_bytes()
                .unwrap(),
            attempts: 0,
            identity_nonce_finalized: false,
            identity_user_abandoned: true,
        };
        store.arm_redrive(id, archived.clone()).unwrap();
        assert!(!has_unresolved_identity_shield(&store, wallet_id, identity_id).unwrap());
        let store = Arc::new(RwLock::new(store));
        // No mock nonce responses: accidentally retrying would reach an unconfigured SDK.
        redrive_pending_identity_shields(
            &Arc::new(dash_sdk::Sdk::new_mock()),
            &store,
            wallet_id,
            &CancellationToken::new(),
        )
        .await;
        assert_eq!(
            store.read().await.pending_redrives(id).unwrap(),
            vec![archived.clone()]
        );
        let mut active = archived;
        active.activity_id = [7; 32];
        active.identity_user_abandoned = false;
        store
            .write()
            .await
            .arm_redrive(SubwalletId::new(wallet_id, 4), active)
            .unwrap();
        assert!(
            has_unresolved_identity_shield(&*store.read().await, wallet_id, identity_id).unwrap()
        );
    }

    #[tokio::test]
    async fn should_keep_delayed_identity_debit_guard_when_nonce_is_unused_then_finalized() {
        use dpp::identity::identity_nonce::validate_identity_nonce_update;
        let wallet_id = [4; 32];
        let identity_id = [5; 32];
        let id = SubwalletId::new(wallet_id, 3);
        let activity_id = [6; 32];
        let transition = identity_shield_transition(identity_id, 7);
        let signed_bytes = transition.serialize_to_bytes().unwrap();
        let mut store = InMemoryShieldedStore::new();
        store
            .arm_redrive(
                id,
                PendingRedrive {
                    activity_id,
                    anchor: [1; 32],
                    nullifiers: vec![],
                    st_bytes: signed_bytes.clone(),
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                },
            )
            .unwrap();
        // A relay returned rejection while retaining the signed debit. Its
        // accompanying genuine nonce snapshot is still six. Seven can land later.
        assert!(!reconcile_identity_shield_nonce(&mut store, id, activity_id, Some(6), 7).unwrap());
        assert!(validate_identity_nonce_update(6, 7, identity_id.into()).is_valid());
        assert!(
            has_unresolved_identity_shield(&store, wallet_id, identity_id).unwrap(),
            "fresh-nonce replacement must stay blocked"
        );
        let pending = store.pending_redrives(id).unwrap();
        assert_eq!(pending[0].st_bytes, signed_bytes);
        assert!(
            !pending[0].identity_nonce_finalized,
            "original bytes remain eligible for safe retry"
        );
        // The original then executes. A new nonce would be valid too, which is
        // why nonce finality must not remove the unresolved-payment guard.
        assert!(validate_identity_nonce_update(7, 8, identity_id.into()).is_valid());
        assert!(reconcile_identity_shield_nonce(&mut store, id, activity_id, Some(7), 7).unwrap());
        assert!(has_unresolved_identity_shield(&store, wallet_id, identity_id).unwrap());
        assert!(store.pending_redrives(id).unwrap()[0].identity_nonce_finalized);
        let store = Arc::new(RwLock::new(store));
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        redrive_pending_identity_shields(&sdk, &store, wallet_id, &CancellationToken::new()).await;
        assert_eq!(
            store.read().await.pending_redrives(id).unwrap()[0].attempts,
            0,
            "parked guard must perform no more broadcasts"
        );
        // The confirmation path can retire the guard, and nonce observation
        // racing behind it must not resurrect the resolved payment.
        store.write().await.clear_redrive(id, &activity_id).unwrap();
        reconcile_identity_shield_nonce(&mut *store.write().await, id, activity_id, Some(7), 7)
            .unwrap();
        assert!(
            !has_unresolved_identity_shield(&*store.read().await, wallet_id, identity_id).unwrap()
        );
    }

    #[tokio::test]
    async fn should_abort_before_broadcast_when_pending_activity_is_missing() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = SubwalletId::new([4; 32], 0);
        let error =
            arm_identity_shield_redrive(&store, id, &None, &identity_shield_transition([5; 32], 1))
                .await
                .unwrap_err();
        assert!(matches!(error, PlatformWalletError::ShieldedBuildError(_)));
        assert!(store.read().await.pending_redrives(id).unwrap().is_empty());
    }

    #[test]
    fn unresolved_identity_debit_is_found_across_shielded_accounts() {
        let wallet_id = [4; 32];
        let identity_id = [5; 32];
        let transition = identity_shield_transition(identity_id, 7);
        let mut store = InMemoryShieldedStore::new();
        store
            .arm_redrive(
                SubwalletId::new(wallet_id, 3),
                PendingRedrive {
                    activity_id: [6; 32],
                    anchor: [1; 32],
                    nullifiers: vec![],
                    st_bytes: transition
                        .serialize_to_bytes()
                        .expect("identity shield should serialize"),
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                },
            )
            .expect("redrive should arm");

        assert!(
            has_unresolved_identity_shield(&store, wallet_id, identity_id).expect("guard lookup")
        );
        assert!(
            !has_unresolved_identity_shield(&store, wallet_id, [9; 32]).expect("guard lookup"),
            "an unrelated identity must remain usable"
        );
    }

    #[test]
    fn corrupt_identity_debit_guard_fails_closed() {
        let wallet_id = [7; 32];
        let mut store = InMemoryShieldedStore::new();
        store
            .arm_redrive(
                SubwalletId::new(wallet_id, 0),
                PendingRedrive {
                    activity_id: [8; 32],
                    anchor: [1; 32],
                    nullifiers: vec![],
                    st_bytes: vec![0xde, 0xad],
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                },
            )
            .expect("redrive should arm");

        assert!(
            has_unresolved_identity_shield(&store, wallet_id, [9; 32]).expect("guard lookup"),
            "an unreadable debit marker must not permit a fresh identity nonce"
        );
    }

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
            RedriveBroadcastOutcome::Inconclusive,
            "a refused re-broadcast does not prove the original never executes"
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

    #[tokio::test]
    async fn identity_redrive_ignores_spend_anchor_and_attempt_limit() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id = [0x45; 32];
        let previous_account = SubwalletId::new(wallet_id, 7);
        let activity_id = [0x67; 32];
        let transition = identity_shield_transition([0x89; 32], 11);
        store
            .write()
            .await
            .arm_redrive(
                previous_account,
                PendingRedrive {
                    activity_id,
                    anchor: [0xAB; 32],
                    nullifiers: vec![],
                    st_bytes: transition.serialize_to_bytes().expect("serialize"),
                    attempts: MAX_REDRIVE_ATTEMPTS,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                },
            )
            .expect("arm identity guard");

        redrive_pending_identity_shields(&sdk, &store, wallet_id, &CancellationToken::new()).await;

        let retained = store
            .read()
            .await
            .pending_redrives(previous_account)
            .expect("retained redrive");
        assert_eq!(
            retained.len(),
            1,
            "an endpoint error cannot clear the guard"
        );
        assert_eq!(
            retained[0].attempts,
            MAX_REDRIVE_ATTEMPTS + 1,
            "identity retries stay live beyond the note-spend attempt budget"
        );
    }

    /// The production payment flow saves a private contact request payment
    /// durably before its broadcast runs: intercepting the broadcast, a second
    /// handle on the store's file reads its journal record and redrive record,
    /// and the payment is marked started. A payment whose save fails is never
    /// broadcast, is not marked started, and releases its notes.
    #[tokio::test]
    async fn should_save_a_private_payment_durably_before_its_broadcast() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 5,
        };
        let wallet_id = [0x68; 32];
        let id = SubwalletId::new(wallet_id, 0);
        for save_fails in [false, true] {
            let dir = tempfile::tempdir().expect("a directory");
            let path = dir.path().join("tree.sqlite");
            let store = Arc::new(RwLock::new(
                FileBackedShieldedStore::open_path(&path, 100).expect("the store"),
            ));
            let note = ShieldedNote {
                position: 0,
                cmx: [1; 32],
                nullifier: [0x5F; 32],
                block_height: 1,
                is_spent: false,
                value: 1_000_000_000_000,
                note_data: vec![],
            };
            store.write().await.save_note(id, &note).expect("a note");
            let (selected, _, _, reservation) =
                reserve_notes_covering(&sdk, &store, id, 1, 2, ShieldedFeeKind::Base)
                    .await
                    .expect("reserved");
            if save_fails {
                rusqlite::Connection::open(&path)
                    .expect("the store's file")
                    .execute_batch(
                        "CREATE TRIGGER reject_redrive_insert BEFORE INSERT ON \
                         shielded_pending_spends BEGIN SELECT RAISE(FAIL, 'injected'); END;",
                    )
                    .expect("the trigger");
            }
            let transition = PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
                Identifier::from([0x44; 32]),
                "privateContactRequest".to_string(),
                vec![BinaryData::new(vec![9; 589])],
                vec![action.clone(), action.clone()],
                0,
                [7; 32],
                vec![8; 64],
                [9; 64],
                PlatformVersion::latest(),
            )
            .expect("a type 29 transition");
            let started = PaymentStarted::default();
            let broadcast = AtomicBool::new(false);
            let submitted = submit_private_document_payment(
                PrivateDocumentSubmission {
                    store: &store,
                    persister: None,
                    wallet_id,
                    id,
                    pending_entry: Some(ShieldedActivityEntry {
                        id: [0x77; 32],
                        kind: ShieldedActivityKind::ShieldedSpend,
                        direction: ShieldedDirection::Out,
                        amount: 0,
                        fee: Some(0),
                        counterparty: None,
                        memo: None,
                        block_height: None,
                        status: ShieldedActivityStatus::Pending,
                        created_at_ms: 1,
                        min_note_position: None,
                        note_cmxs: vec![],
                        spent_nullifiers: vec![],
                    }),
                    selected_notes: &selected,
                    reservation,
                    state_transition: transition,
                    anchor: [7; 32],
                    private_contact: Some(payment),
                    journaled: Some(&[[9; 32]]),
                    payment_started: &started,
                },
                |_| {
                    broadcast.store(true, Ordering::SeqCst);
                    let durable =
                        FileBackedShieldedStore::open_path(&path, 100).expect("a second handle");
                    assert_eq!(
                        durable
                            .private_payment_journal(wallet_id)
                            .expect("the journal")
                            .len(),
                        1,
                        "the journal record is on the file"
                    );
                    assert_eq!(
                        durable
                            .private_contact_redrives_for_wallet(wallet_id)
                            .expect("the records")
                            .len(),
                        1,
                        "the redrive record and its association are on the file"
                    );
                    assert!(started.is_marked(), "marked once saved");
                    std::future::ready(Err(SpendFailure {
                        error: PlatformWalletError::ShieldedSpendUnconfirmed {
                            operation: "private document create from shielded pool",
                            reason: "intercepted".to_string(),
                        },
                        never_delivered: false,
                    }))
                },
            )
            .await;
            let unspent = store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .len();
            if save_fails {
                assert!(
                    matches!(submitted, Err(PlatformWalletError::ShieldedStoreError(_))),
                    "{submitted:?}"
                );
                assert!(!broadcast.load(Ordering::SeqCst), "never broadcast");
                assert!(!started.is_marked(), "never marked");
                assert_eq!(unspent, 1, "the notes are released");
                assert!(store
                    .read()
                    .await
                    .private_payment_journal(wallet_id)
                    .expect("the journal")
                    .is_empty());
            } else {
                assert!(
                    matches!(
                        submitted,
                        Err(PlatformWalletError::ShieldedSpendUnconfirmed { .. })
                    ),
                    "{submitted:?}"
                );
                assert!(broadcast.load(Ordering::SeqCst));
                assert_eq!(unspent, 0, "the notes stay reserved");
            }
        }
    }

    /// A payment dropped while its recovery save waits for the store has saved
    /// and broadcast nothing, and its note guard releases the notes once the
    /// store frees: the notes are handed over only after the save.
    #[tokio::test]
    async fn should_release_the_notes_of_a_payment_dropped_while_its_save_waits() {
        use std::future::Future;
        use std::task::{Context, Poll, Waker};

        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        let wallet_id = [0x69; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let note = ShieldedNote {
            position: 0,
            cmx: [1; 32],
            nullifier: [0x5E; 32],
            block_height: 1,
            is_spent: false,
            value: 1_000_000_000_000,
            note_data: vec![],
        };
        store.write().await.save_note(id, &note).expect("a note");
        let (selected, _, _, reservation) =
            reserve_notes_covering(&sdk, &store, id, 1, 2, ShieldedFeeKind::Base)
                .await
                .expect("reserved");
        let transition = PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
            Identifier::from([0x44; 32]),
            "privateContactRequest".to_string(),
            vec![BinaryData::new(vec![9; 589])],
            vec![action.clone(), action],
            0,
            [7; 32],
            vec![8; 64],
            [9; 64],
            PlatformVersion::latest(),
        )
        .expect("a type 29 transition");
        let started = PaymentStarted::default();
        let mut cx = Context::from_waker(Waker::noop());

        let held = store.write().await;
        let mut submitted = Box::pin(submit_private_document_payment(
            PrivateDocumentSubmission {
                store: &store,
                persister: None,
                wallet_id,
                id,
                pending_entry: Some(ShieldedActivityEntry {
                    id: [0x78; 32],
                    kind: ShieldedActivityKind::ShieldedSpend,
                    direction: ShieldedDirection::Out,
                    amount: 0,
                    fee: Some(0),
                    counterparty: None,
                    memo: None,
                    block_height: None,
                    status: ShieldedActivityStatus::Pending,
                    created_at_ms: 1,
                    min_note_position: None,
                    note_cmxs: vec![],
                    spent_nullifiers: vec![],
                }),
                selected_notes: &selected,
                reservation,
                state_transition: transition,
                anchor: [7; 32],
                private_contact: Some(PrivateContactPayment {
                    sender: [0xB1; 32],
                    recipient: [0xA1; 32],
                    account_reference: 5,
                }),
                journaled: Some(&[[9; 32]]),
                payment_started: &started,
            },
            |_| async { panic!("never broadcast") },
        ));
        // The submission waits for the store before its anchor step.
        assert!(submitted.as_mut().poll(&mut cx).is_pending());
        // Freed, the lock goes to the submission first; this request queues
        // behind it, so the submission's next one, the save, waits for it.
        drop(held);
        let mut ours = Box::pin(store.write());
        assert!(ours.as_mut().poll(&mut cx).is_pending());
        assert!(submitted.as_mut().poll(&mut cx).is_pending());
        let Poll::Ready(held) = ours.as_mut().poll(&mut cx) else {
            panic!("the store is ours once the submission's anchor step is done");
        };
        // Dropped while the save waits.
        drop(submitted);
        assert!(!started.is_marked(), "nothing was saved");
        drop(held);
        for _ in 0..50 {
            if !store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .len(),
            1,
            "the guard released the notes"
        );
        assert!(store
            .read()
            .await
            .private_payment_journal(wallet_id)
            .expect("the journal")
            .is_empty());
    }

    /// A stale spend attempt's detached cleanup, delayed until after an
    /// account purge and rebind (or the wallet's replacement) let a newer
    /// attempt reserve the same notes and save its payment, releases nothing:
    /// the newer attempt's notes stay reserved, and its redrive record,
    /// association and journal record stay.
    #[tokio::test]
    async fn should_not_release_a_newer_attempt_s_notes_from_a_stale_cleanup() {
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let wallet_id = [0x6A; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let note = ShieldedNote {
            position: 0,
            cmx: [1; 32],
            nullifier: [0x5D; 32],
            block_height: 1,
            is_spent: false,
            value: 1_000_000_000_000,
            note_data: vec![],
        };
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 5,
        };
        for wallet_replaced in [false, true] {
            let dir = tempfile::tempdir().expect("a directory");
            let store = Arc::new(RwLock::new(
                FileBackedShieldedStore::open_path(dir.path().join("tree.sqlite"), 100)
                    .expect("the store"),
            ));
            store.write().await.save_note(id, &note).expect("a note");
            let (_, _, _, stale) =
                reserve_notes_covering(&sdk, &store, id, 1, 2, ShieldedFeeKind::Base)
                    .await
                    .expect("reserved");
            // Dropped while the store is busy: its cleanup waits on a task.
            let mut held = store.write().await;
            drop(stale);
            if wallet_replaced {
                held.purge_wallet(wallet_id).expect("the wallet's purge");
            } else {
                held.purge_subwallet(id).expect("the account's purge");
            }
            held.save_note(id, &note).expect("the note again");
            assert!(held
                .mark_pending_by(id, &note.nullifier, u64::MAX)
                .expect("reserved by the newer attempt"));
            held.arm_private_contact_payment(
                id,
                PendingRedrive {
                    activity_id: [0x79; 32],
                    anchor: [7; 32],
                    nullifiers: vec![note.nullifier],
                    st_bytes: vec![0xAB; 96],
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                },
                payment,
                &[[0xC4; 32]],
            )
            .expect("the newer payment saved");
            drop(held);
            // The stale cleanup gets the store now.
            for _ in 0..10 {
                tokio::task::yield_now().await;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
            let store = store.read().await;
            let context = format!("wallet replaced: {wallet_replaced}");
            assert!(
                store.get_unspent_notes(id).expect("notes").is_empty(),
                "{context}: the newer attempt's note stays reserved"
            );
            assert_eq!(
                store.pending_redrives(id).expect("records").len(),
                1,
                "{context}"
            );
            assert_eq!(
                store
                    .private_contact_redrives_for_wallet(wallet_id)
                    .expect("associations")
                    .len(),
                1,
                "{context}"
            );
            assert_eq!(
                store
                    .private_payment_journal(wallet_id)
                    .expect("the journal")
                    .len(),
                1,
                "{context}"
            );
        }
    }

    /// A private contact request payment's journal record outlives its notes'
    /// and redrive record whatever the spend's outcome, except a transition
    /// that provably never left the wallet. Settled unconfirmed (its
    /// broadcast may have been delivered and the result wait failed), the
    /// notes stay reserved and the record, association and journal record
    /// survive a reopen. Rejected after delivery, the notes and record are
    /// released, the journal record stays. Never delivered, the journal
    /// record is discharged too and the payment reads as not reaching the
    /// network.
    #[tokio::test]
    async fn should_keep_a_private_payment_s_journal_record_unless_it_never_left() {
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let action = SerializedAction {
            nullifier: [1; 32],
            rk: [2; 32],
            cmx: [3; 32],
            encrypted_note: vec![4; 216],
            cv_net: [5; 32],
            spend_auth_sig: [6; 64],
        };
        let entry = BinaryData::new(vec![9; 589]);
        let commitment = [9u8; 32];
        let transition = PrivateDocumentCreateFromShieldedPoolTransition::try_from_bundle(
            Identifier::from([0x44; 32]),
            "privateContactRequest".to_string(),
            vec![entry],
            vec![action.clone(), action],
            0,
            [7; 32],
            vec![8; 64],
            [9; 64],
            PlatformVersion::latest(),
        )
        .expect("a type 29 transition");
        let payment = PrivateContactPayment {
            sender: [0xB1; 32],
            recipient: [0xA1; 32],
            account_reference: 5,
        };
        let wallet_id = [0x67; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let failure = |outcome: &str| match outcome {
            "unconfirmed" => SpendFailure {
                error: PlatformWalletError::ShieldedSpendUnconfirmed {
                    operation: "private document create from shielded pool",
                    reason: "the result wait timed out".to_string(),
                },
                never_delivered: false,
            },
            "rejected after delivery" => SpendFailure {
                error: PlatformWalletError::ShieldedBroadcastFailed(
                    "rejected on its merits".to_string(),
                ),
                never_delivered: false,
            },
            _ => SpendFailure {
                error: PlatformWalletError::ShieldedBroadcastFailed(
                    "refused at the door".to_string(),
                ),
                never_delivered: true,
            },
        };
        for outcome in ["unconfirmed", "rejected after delivery", "never delivered"] {
            let dir = tempfile::tempdir().expect("a directory");
            let path = dir.path().join("tree.sqlite");
            let store = Arc::new(RwLock::new(
                FileBackedShieldedStore::open_path(&path, 100).expect("the store"),
            ));
            let note = ShieldedNote {
                position: 0,
                cmx: [1; 32],
                nullifier: [0x5F; 32],
                block_height: 1,
                is_spent: false,
                value: 1_000_000_000_000,
                note_data: vec![],
            };
            store.write().await.save_note(id, &note).expect("a note");
            let (selected, _, _, reservation) =
                reserve_notes_covering(&sdk, &store, id, 1, 2, ShieldedFeeKind::Base)
                    .await
                    .expect("reserved");
            let started = PaymentStarted::default();
            arm_redrive_before_broadcast(
                &store,
                id,
                Some([0x77; 32]),
                [7; 32],
                selected.iter().map(|note| note.nullifier).collect(),
                &transition,
                "private document create from shielded pool",
                Some(payment),
                Some(&started),
            )
            .await
            .expect("saved before the broadcast");
            assert_eq!(
                store
                    .read()
                    .await
                    .private_payment_journal(wallet_id)
                    .expect("journal")
                    .len(),
                1,
                "{outcome}: journaled before the broadcast"
            );

            let settled = settle_private_document_payment(
                &store,
                None,
                wallet_id,
                id,
                &None,
                reservation,
                &selected,
                Some(&[commitment]),
                &started,
                Err(failure(outcome)),
            )
            .await;
            assert!(settled.is_err(), "{outcome}: {settled:?}");
            let unspent = store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .len();
            drop(store);
            let reopened = FileBackedShieldedStore::open_path(&path, 100).expect("reopened");
            let journal = reopened
                .private_payment_journal(wallet_id)
                .expect("journal");
            let saved = reopened
                .private_contact_redrives_for_wallet(wallet_id)
                .expect("listed");
            match outcome {
                "unconfirmed" => {
                    assert_eq!(unspent, 0, "the notes stay reserved");
                    assert_eq!(saved.len(), 1, "the record and its association stay");
                    assert_eq!(journal.len(), 1, "the journal record stays");
                    assert!(started.may_have_reached_network());
                }
                "rejected after delivery" => {
                    assert_eq!(unspent, 1, "the notes are released");
                    assert!(saved.is_empty(), "the record goes with them");
                    assert_eq!(journal.len(), 1, "the journal record stays");
                    assert_eq!(journal[0].commitment, commitment);
                    assert_eq!(journal[0].payment, payment);
                    assert!(started.may_have_reached_network());
                }
                _ => {
                    assert_eq!(unspent, 1, "the notes are released");
                    assert!(saved.is_empty(), "the record goes with them");
                    assert!(journal.is_empty(), "the journal record is discharged");
                    assert!(!started.may_have_reached_network());
                }
            }
        }
    }

    /// A spend that failed before anything was broadcast, here a private document payment
    /// whose anchor fetch the mock refuses, and is dropped while its rollback waits for the
    /// store lock another task holds, still releases its notes: the guard owns them until the
    /// rollback has cleared them.
    #[tokio::test]
    async fn should_release_the_notes_of_a_spend_dropped_during_its_failure_rollback() {
        struct UnreachableProver;
        impl OrchardProver for UnreachableProver {
            fn proving_key(&self) -> &ProvingKey {
                panic!("nothing is proved")
            }
        }

        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_version(PlatformVersion::latest())
                .build()
                .expect("a mock SDK"),
        );
        let wallet_id = [0x65; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let note = ShieldedNote {
            position: 0,
            cmx: [1; 32],
            nullifier: [0x5E; 32],
            block_height: 1,
            is_spent: false,
            value: 1_000_000_000_000,
            note_data: vec![],
        };
        store.write().await.save_note(id, &note).expect("a note");
        let keys = OrchardKeySet::from_seed(&[0x42; 64], Network::Testnet, 0).expect("keys");
        let started = PaymentStarted::default();

        // The spend waits for the store behind this task; this task's next acquisition waits
        // behind the spend, so the lock passes to it right after the spend's reservation.
        let held = store.write().await;
        let mut spend = Box::pin(private_document_create_from_pool_with_metadata(
            &sdk,
            &store,
            None,
            wallet_id,
            &keys,
            0,
            Identifier::from([0x44; 32]),
            "privateContactRequest",
            vec![BinaryData::new(vec![1; 589])],
            None,
            &started,
            &UnreachableProver,
        ));
        assert!(poll!(&mut spend).is_pending(), "waits to reserve");
        let mut again = Box::pin(store.write());
        assert!(poll!(&mut again).is_pending(), "queued behind the spend");
        drop(held);
        // Reserved, refused by the anchor fetch, and waiting to roll back behind `again`.
        assert!(
            poll!(&mut spend).is_pending(),
            "the rollback waits for the store"
        );
        let Poll::Ready(contended) = poll!(&mut again) else {
            panic!("the lock passes on after the reservation");
        };
        assert!(
            contended.get_unspent_notes(id).expect("notes").is_empty(),
            "reserved while the rollback waits"
        );
        drop(spend);
        drop(contended);
        for _ in 0..100 {
            if !store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .is_empty()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .iter()
                .map(|note| note.nullifier)
                .collect::<Vec<_>>(),
            vec![note.nullifier],
            "the note is spendable again"
        );
        assert!(!started.is_marked());
    }

    /// A reservation dropped while the store is locked is released once the
    /// lock frees, on a spawned task; one handed over stays.
    #[tokio::test]
    async fn should_release_a_dropped_reservation_once_the_store_frees() {
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = SubwalletId::new([0x64; 32], 0);
        let note = ShieldedNote {
            position: 0,
            cmx: [1; 32],
            nullifier: [0x5B; 32],
            block_height: 1,
            is_spent: false,
            value: 1_000_000_000_000,
            note_data: vec![],
        };
        store.write().await.save_note(id, &note).expect("a note");
        let unspent = || async {
            store
                .read()
                .await
                .get_unspent_notes(id)
                .expect("notes")
                .len()
        };

        let (_, _, _, reservation) =
            reserve_notes_covering(&sdk, &store, id, 1, 2, ShieldedFeeKind::Base)
                .await
                .expect("reserved");
        assert_eq!(unspent().await, 0);
        let held = store.write().await;
        drop(reservation);
        drop(held);
        for _ in 0..100 {
            if unspent().await == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(unspent().await, 1, "released once the lock freed");

        let (_, _, _, mut kept) =
            reserve_notes_covering(&sdk, &store, id, 1, 2, ShieldedFeeKind::Base)
                .await
                .expect("reserved");
        kept.hand_over();
        drop(kept);
        assert_eq!(unspent().await, 0, "the spend owns it");
    }

    /// A spend dropped while it fetches the anchors, before its notes have an
    /// owner that frees them, leaves them spendable again: here a private
    /// document payment whose anchor fetch goes to a node that accepts the
    /// connection and never answers.
    #[tokio::test]
    async fn should_release_the_notes_of_a_spend_dropped_during_its_anchor_fetch() {
        struct NoKeys;
        impl ContextProvider for NoKeys {
            fn get_data_contract(
                &self,
                _id: &Identifier,
                _platform_version: &PlatformVersion,
            ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
                Ok(None)
            }
            fn get_token_configuration(
                &self,
                _token_id: &Identifier,
            ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
                Ok(None)
            }
            fn get_quorum_public_key(
                &self,
                _quorum_type: u32,
                _quorum_hash: [u8; 32],
                _core_chain_locked_height: u32,
            ) -> Result<[u8; 48], ContextProviderError> {
                Err(ContextProviderError::Generic("no quorum keys".to_string()))
            }
            fn get_platform_activation_height(
                &self,
            ) -> Result<CoreBlockHeight, ContextProviderError> {
                Ok(1)
            }
        }
        struct UnreachableProver;
        impl OrchardProver for UnreachableProver {
            fn proving_key(&self) -> &ProvingKey {
                panic!("nothing is proved")
            }
        }

        /// A loopback node that accepts connections and never answers. Dropped,
        /// even while a failed assertion unwinds, it stops accepting, closes
        /// what it accepted and joins its thread.
        struct SilentNode {
            address: std::net::SocketAddr,
            shutdown: Arc<std::sync::atomic::AtomicBool>,
            thread: Option<std::thread::JoinHandle<()>>,
        }
        impl SilentNode {
            fn start() -> Self {
                let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
                listener.set_nonblocking(true).expect("non-blocking accept");
                let address = listener.local_addr().expect("its address");
                let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let stop = Arc::clone(&shutdown);
                let thread = std::thread::spawn(move || {
                    let mut held = Vec::new();
                    while !stop.load(std::sync::atomic::Ordering::Acquire) {
                        match listener.accept() {
                            Ok((stream, _)) => held.push(stream),
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::sleep(std::time::Duration::from_millis(5));
                            }
                            Err(_) => break,
                        }
                    }
                    drop(held);
                });
                Self {
                    address,
                    shutdown,
                    thread: Some(thread),
                }
            }
        }
        impl Drop for SilentNode {
            fn drop(&mut self) {
                self.shutdown
                    .store(true, std::sync::atomic::Ordering::Release);
                if let Some(thread) = self.thread.take() {
                    let _ = thread.join();
                }
            }
        }

        let node = SilentNode::start();
        let address = node.address;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new(
                AddressList::from_str(&format!("http://{address}")).expect("an address"),
            )
            .with_network(Network::Testnet)
            .with_version(PlatformVersion::latest())
            .with_context_provider(NoKeys)
            .build()
            .expect("a network SDK"),
        );

        let wallet_id = [0x63; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let note = ShieldedNote {
            position: 0,
            cmx: [1; 32],
            nullifier: [0x5A; 32],
            block_height: 1,
            is_spent: false,
            value: 1_000_000_000_000,
            note_data: vec![],
        };
        store.write().await.save_note(id, &note).expect("a note");
        let keys = OrchardKeySet::from_seed(&[0x42; 64], Network::Testnet, 0).expect("keys");
        let started = PaymentStarted::default();

        let paid = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            private_document_create_from_pool_with_metadata(
                &sdk,
                &store,
                None,
                wallet_id,
                &keys,
                0,
                Identifier::from([0x44; 32]),
                "privateContactRequest",
                vec![BinaryData::new(vec![1; 589])],
                None,
                &started,
                &UnreachableProver,
            ),
        )
        .await;
        assert!(paid.is_err(), "dropped during the anchor fetch: {paid:?}");
        assert!(!started.is_marked());
        let unspent = store.read().await.get_unspent_notes(id).expect("notes");
        assert_eq!(
            unspent
                .iter()
                .map(|note| note.nullifier)
                .collect::<Vec<_>>(),
            vec![note.nullifier],
            "the note is spendable again"
        );
    }

    /// A private payment is marked started only once its record is saved:
    /// dropped while the save waits for the store, the spend has saved
    /// nothing and is not marked, so its contact request's reservation is
    /// released rather than kept for a payment that never left. Once the
    /// lock frees, a save marks it.
    #[tokio::test]
    async fn should_mark_a_payment_started_only_once_its_record_is_saved() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = SubwalletId::new([0x62; 32], 0);
        let transition = identity_shield_transition([0xBB; 32], 4);
        let started = PaymentStarted::default();

        let held = store.write().await;
        let save = arm_redrive_before_broadcast(
            &store,
            id,
            Some([0x78; 32]),
            [7; 32],
            vec![[2; 32]],
            &transition,
            "test spend",
            None,
            Some(&started),
        );
        let mut save = Box::pin(save);
        assert!(
            (&mut save).now_or_never().is_none(),
            "the save waits for the store"
        );
        drop(save);
        drop(held);
        assert!(!started.is_marked(), "dropped before its record was saved");
        assert!(store
            .read()
            .await
            .pending_redrives(id)
            .expect("redrives")
            .is_empty());

        arm_redrive_before_broadcast(
            &store,
            id,
            Some([0x78; 32]),
            [7; 32],
            vec![[2; 32]],
            &transition,
            "test spend",
            None,
            Some(&started),
        )
        .await
        .expect("saved");
        assert!(started.is_marked());
        assert_eq!(
            store
                .read()
                .await
                .pending_redrives(id)
                .expect("redrives")
                .len(),
            1
        );
    }

    /// A spend's record is saved before its broadcast, keyed by its
    /// activity entry; one with no entry to key it by is refused, nothing
    /// saved.
    #[tokio::test]
    async fn should_save_a_spends_record_before_its_broadcast() {
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let id = SubwalletId::new([0x61; 32], 0);
        let transition = identity_shield_transition([0xBA; 32], 3);

        let refused = arm_redrive_before_broadcast(
            &store,
            id,
            None,
            [7; 32],
            vec![[1; 32]],
            &transition,
            "test spend",
            None,
            None,
        )
        .await;
        assert!(
            matches!(refused, Err(PlatformWalletError::ShieldedBuildError(_))),
            "{refused:?}"
        );
        assert!(store
            .read()
            .await
            .pending_redrives(id)
            .expect("redrives")
            .is_empty());

        arm_redrive_before_broadcast(
            &store,
            id,
            Some([0x77; 32]),
            [7; 32],
            vec![[1; 32]],
            &transition,
            "test spend",
            None,
            None,
        )
        .await
        .expect("saved");
        let saved = store.read().await.pending_redrives(id).expect("redrives");
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].activity_id, [0x77; 32]);
        assert_eq!(saved[0].nullifiers, vec![[1; 32]]);
    }

    #[tokio::test]
    async fn ordinary_redrive_pass_leaves_identity_guard_untouched() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let store = Arc::new(RwLock::new(InMemoryShieldedStore::new()));
        let wallet_id = [0x54; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let anchor = [0x76; 32];
        let activity_id = [0x98; 32];
        store
            .write()
            .await
            .arm_redrive(
                id,
                PendingRedrive {
                    activity_id,
                    anchor,
                    nullifiers: vec![],
                    st_bytes: identity_shield_transition([0xBA; 32], 3)
                        .serialize_to_bytes()
                        .expect("serialize"),
                    attempts: 0,
                    identity_nonce_finalized: false,
                    identity_user_abandoned: false,
                },
            )
            .expect("arm identity guard");
        let recorded = [anchor].into_iter().collect();

        redrive_pending_spends(&sdk, &store, id, &recorded, &CancellationToken::new()).await;

        let retained = store.read().await.pending_redrives(id).expect("guard");
        assert_eq!(retained.len(), 1);
        assert_eq!(
            retained[0].attempts, 0,
            "the anchor-gated spend pass must not consume identity retry state"
        );
    }

    /// Decision paths that must NOT touch the network (the mock SDK has
    /// no broadcast expectation, so any attempt would error into the
    /// inconclusive arm and bump the counter): a pruned anchor belongs
    /// to the release backstop, an exhausted budget stays parked, and a
    /// corrupt stored transition is kept without a broadcast, as it may be
    /// the only evidence of a payment that can still execute.
    #[tokio::test]
    async fn should_skip_pruned_exhausted_and_undecodable_redrives_keeping_each() {
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
            identity_nonce_finalized: false,
            identity_user_abandoned: false,
        };
        {
            let mut guard = store.write().await;
            // Pruned anchor (10 not in recorded set) → untouched.
            guard.arm_redrive(id, rec(1, 10, 0)).unwrap();
            // Recorded anchor but attempts exhausted → untouched.
            guard
                .arm_redrive(id, rec(2, 11, MAX_REDRIVE_ATTEMPTS))
                .unwrap();
            // Recorded anchor, budget left, garbage bytes → kept, with no
            // broadcast.
            guard.arm_redrive(id, rec(3, 12, 0)).unwrap();
        }
        let recorded: std::collections::HashSet<[u8; 32]> =
            [[11u8; 32], [12u8; 32]].into_iter().collect();

        redrive_pending_spends(&sdk, &store, id, &recorded, &CancellationToken::new()).await;

        let left = store.read().await.pending_redrives(id).unwrap();
        let ids: Vec<[u8; 32]> = left.iter().map(|r| r.activity_id).collect();
        assert!(ids.contains(&[1u8; 32]), "pruned-anchor record left alone");
        assert!(ids.contains(&[2u8; 32]), "exhausted record left alone");
        assert!(
            ids.contains(&[3u8; 32]),
            "corrupt record kept without a broadcast attempt"
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

    /// A broadcast error carrying a consensus cause comes from an
    /// unauthenticated endpoint, with no proof behind it: the spend stays
    /// unconfirmed and its reservation stays.
    #[test]
    fn should_read_a_wait_stage_consensus_verdict_as_unconfirmed() {
        let cause = ConsensusError::BasicError(BasicError::ProtocolVersionParsingError(
            ProtocolVersionParsingError::new("bad version".to_string()),
        ));
        let err = classify_spend_wait_failure("unshield", &broadcast_err(Some(cause)));
        assert!(
            matches!(err, PlatformWalletError::ShieldedSpendUnconfirmed { .. }),
            "{err:?}"
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

    /// A consensus rejection surfacing from `broadcast()` stays definitive
    /// for the shields into the pool that classify with
    /// `broadcast_definitely_failed`, which move funds to the wallet's own
    /// notes. On a note spend's wait the same verdict reads as unconfirmed:
    /// no proof backs it.
    #[test]
    fn should_read_a_consensus_rejection_as_definitive_only_for_shields_into_the_pool() {
        assert!(broadcast_definitely_failed(&consensus_metadata_rejection()));
        let err = classify_spend_wait_failure("transfer", &consensus_metadata_rejection());
        assert!(
            matches!(err, PlatformWalletError::ShieldedSpendUnconfirmed { .. }),
            "{err:?}"
        );
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

    /// A nonce rejection on a note spend's wait, wrapped in the retry
    /// envelope, is an unauthenticated verdict too: unconfirmed, the notes
    /// kept, never a typed refusal a host could retry on.
    #[test]
    fn should_read_a_wrapped_nonce_rejection_on_the_wait_as_unconfirmed() {
        use dpp::address_funds::PlatformAddress;
        use dpp::consensus::state::address_funds::AddressInvalidNonceError;
        use dpp::consensus::state::state_error::StateError;

        let address = PlatformAddress::P2pkh([9u8; 20]);
        let cause = ConsensusError::StateError(StateError::AddressInvalidNonceError(
            AddressInvalidNonceError::new(address, 7, 8),
        ));
        let inner = dash_sdk::Error::Protocol(dpp::ProtocolError::ConsensusError(Box::new(cause)));
        let wrapped = dash_sdk::Error::NoAvailableAddressesToRetry(Box::new(inner));

        let err = classify_spend_wait_failure("withdraw", &wrapped);
        assert!(
            matches!(err, PlatformWalletError::ShieldedSpendUnconfirmed { .. }),
            "{err:?}"
        );
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
