//! Shielded transaction operations (5 transition types), multi-account.
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
//! The five transition types are:
//! - **Shield** (Type 15): transparent platform addresses → shielded pool
//! - **ShieldFromAssetLock** (Type 18): Core L1 asset lock → shielded pool
//! - **Unshield** (Type 17): shielded pool → transparent platform address
//! - **Transfer** (Type 16): shielded pool → shielded pool (private)
//! - **Withdraw** (Type 19): shielded pool → Core L1 address

use super::keys::OrchardKeySet;
use super::note_selection::select_notes_with_fee;
use super::store::{ShieldedNote, ShieldedStore, SubwalletId};
use crate::changeset::{PlatformWalletChangeSet, ShieldedChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

use std::collections::BTreeMap;
use std::sync::Arc;

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, OrchardAddress, PlatformAddress,
};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::shielded::builder::{
    build_shield_transition, build_shielded_transfer_transition,
    build_shielded_withdrawal_transition, build_unshield_transition, OrchardProver, SpendableNote,
};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::withdrawal::Pooling;
use grovedb_commitment_tree::{Anchor, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{info, trace, warn};

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

// -------------------------------------------------------------------------
// Shield: platform addresses -> shielded pool (Type 15)
// -------------------------------------------------------------------------

/// Shield credits from transparent platform addresses into the
/// shielded pool, with the resulting note assigned to `account`'s
/// default Orchard payment address derived from `keys`.
///
/// Thin wrapper over [`build_shield_st`] + broadcast — retained for
/// backward compatibility so existing callers
/// (`PlatformWallet::shielded_shield_from_account`) are unchanged.
#[allow(clippy::too_many_arguments)]
pub async fn shield<Sig: Signer<PlatformAddress>, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    keys: &OrchardKeySet,
    account: u32,
    inputs: BTreeMap<PlatformAddress, Credits>,
    amount: u64,
    signer: &Sig,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let (state_transition, claimed_inputs) =
        build_shield_st(sdk, keys, account, inputs, amount, signer, prover).await?;

    trace!("Shield credits: state transition built, broadcasting...");
    broadcast_shield_st(sdk, &state_transition, &claimed_inputs).await?;

    info!(account, credits = amount, "Shield broadcast succeeded");
    Ok(())
}

/// Build (fetch nonces + prove + sign) a Type-15 shield state transition
/// WITHOUT broadcasting it. Returns the signed transition plus the
/// claimed-inputs map (the latter enriches the broadcast-time
/// `AddressesNotEnoughFunds` diagnostic).
///
/// This is the capture seam: callers that need the serialized transition
/// (e.g. adversarial byte-mutation tests, custom broadcast policies) take
/// it here and broadcast separately. [`shield`] is the build-then-broadcast
/// wrapper.
#[allow(clippy::too_many_arguments)]
pub async fn build_shield_st<Sig: Signer<PlatformAddress>, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    keys: &OrchardKeySet,
    account: u32,
    inputs: BTreeMap<PlatformAddress, Credits>,
    amount: u64,
    signer: &Sig,
    prover: &P,
) -> Result<(StateTransition, BTreeMap<PlatformAddress, (u32, Credits)>), PlatformWalletError> {
    let recipient_addr = default_orchard_address(keys)?;

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

    let fetched = fetch_inputs_with_nonce(sdk, &inputs).await.map_err(|e| {
        // The hard balance check is the common pre-broadcast failure;
        // surface its structured (address, balance, required) info as a
        // diagnostic string rather than the opaque `{e}` form, matching
        // the richness of the broadcast-side handler below. The FFI
        // shape is unchanged (the host still receives a string body).
        if let Some(short) = address_not_enough_funds(&e) {
            PlatformWalletError::ShieldedBuildError(format!(
                "shield input {} has insufficient balance: requires {} credits, has {}",
                short.address().to_bech32m_string(sdk.network),
                short.required_balance(),
                short.balance(),
            ))
        } else {
            PlatformWalletError::ShieldedBuildError(format!("fetch input nonces: {e}"))
        }
    })?;

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
        sdk.version(),
    )
    .await
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

    Ok((state_transition, claimed_inputs))
}

/// Broadcast a built shield transition with the rich
/// `AddressesNotEnoughFunds` diagnostic. Waits for proven execution (not
/// just relay-ACK) so the host only sees success once Platform has
/// included the transition.
async fn broadcast_shield_st(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition: &StateTransition,
    claimed_inputs: &BTreeMap<PlatformAddress, (u32, Credits)>,
) -> Result<(), PlatformWalletError> {
    let network = sdk.network;
    state_transition
        .broadcast_and_wait::<StateTransitionProofResult>(sdk, None)
        .await
        .map_err(|e| {
            if let Some(rich) = addresses_not_enough_funds(&e) {
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
                PlatformWalletError::ShieldedBroadcastFailed(e.to_string())
            }
        })?;
    Ok(())
}

// -------------------------------------------------------------------------
// ShieldFromAssetLock: Core L1 asset lock -> shielded pool (Type 18)
// (orchestrated entry point lives in `wallet/shielded/fund_from_asset_lock.rs`)
// -------------------------------------------------------------------------

<<<<<<< HEAD
/// Shield credits from a Core L1 asset lock into the shielded
/// pool, with the resulting note assigned to `account`'s default
/// Orchard payment address derived from `keys`.
///
/// Thin wrapper over [`build_shield_from_asset_lock_st`] + broadcast.
pub async fn shield_from_asset_lock<P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    keys: &OrchardKeySet,
    account: u32,
    asset_lock_proof: AssetLockProof,
    private_key: &[u8],
    amount: u64,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let state_transition = build_shield_from_asset_lock_st(
        sdk,
        keys,
        account,
        asset_lock_proof,
        private_key,
        amount,
        prover,
    )?;

    trace!("Shield from asset lock: state transition built, broadcasting...");
    // Wait for proven execution rather than relay-ACK. This matters most
    // for Type 18: the asset-lock proof is single-use, so a false-
    // positive success on a transition Platform later rejects would
    // strand the user's L1 outpoint with no in-app signal. The proven
    // result is discarded; we only need the confirmation.
    broadcast_st(sdk, &state_transition).await?;

    info!(
        account,
        credits = amount,
        "Shield from asset lock broadcast succeeded"
    );
    Ok(())
}

/// Build a Type-18 shield-from-asset-lock state transition WITHOUT
/// broadcasting. The capture seam for the single-use asset-lock proof —
/// callers that need to control broadcast (e.g. the SH-035 replay test)
/// take the transition here. [`shield_from_asset_lock`] is the
/// build-then-broadcast wrapper.
pub fn build_shield_from_asset_lock_st<P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    keys: &OrchardKeySet,
    account: u32,
    asset_lock_proof: AssetLockProof,
    private_key: &[u8],
    amount: u64,
    prover: &P,
) -> Result<StateTransition, PlatformWalletError> {
    let recipient_addr = default_orchard_address(keys)?;

    info!(
        account,
        credits = amount,
        "Shield from asset lock: building state transition"
    );

    build_shield_from_asset_lock_transition(
        &recipient_addr,
        amount,
        asset_lock_proof,
        private_key,
        prover,
        [0u8; 36],
        sdk.version(),
    )
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))
}

=======
>>>>>>> feat/rs-platform-wallet-e2e
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
    let id = SubwalletId::new(wallet_id, account);

    let (selected_notes, total_input, exact_fee) =
        reserve_unspent_notes(sdk, store, id, amount, 1).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        "Unshield"
    );

    // From here on every error path must release the reservation
    // taken by `reserve_unspent_notes`.
    let result = async {
        let state_transition = build_unshield_st(
            sdk,
            store,
            keys,
            to_address,
            amount,
            exact_fee,
            &selected_notes,
            prover,
        )
        .await?;

        trace!("Unshield: state transition built, broadcasting...");
        broadcast_st(sdk, &state_transition).await
    }
    .await;

    match result {
        Ok(()) => {
            // Broadcast already succeeded; spent-state bookkeeping is
            // best-effort. Surfacing a local write failure as a send
            // failure here would invite duplicate retries — the next
            // nullifier sync reconciles any drift.
            //
            // No double-spend follows from this downgrade: the
            // authoritative no-reuse guarantee is the on-chain nullifier
            // set, not this local mark. Worst case, before the next
            // nullifier sync runs the note is re-selected and a second
            // spend is built + proven, then rejected at broadcast with a
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
        Err(e) => {
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
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let recipient_addr = payment_address_to_orchard(to_address)?;
    let id = SubwalletId::new(wallet_id, account);

    let (selected_notes, total_input, exact_fee) =
        reserve_unspent_notes(sdk, store, id, amount, 2).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        "Shielded transfer"
    );

    let result = async {
        let state_transition = build_transfer_st(
            sdk,
            store,
            keys,
            &recipient_addr,
            amount,
            exact_fee,
            &selected_notes,
            prover,
        )
        .await?;

        trace!("Shielded transfer: state transition built, broadcasting...");
        broadcast_st(sdk, &state_transition).await
    }
    .await;

    match result {
        Ok(()) => {
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
        Err(e) => {
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
    let id = SubwalletId::new(wallet_id, account);
    let output_script = CoreScript::from_bytes(to_address.script_pubkey().to_bytes());

    let (selected_notes, total_input, exact_fee) =
        reserve_unspent_notes(sdk, store, id, amount, 1).await?;

    info!(
        account,
        credits = amount,
        fee = exact_fee,
        inputs = selected_notes.len(),
        total_input,
        "Shielded withdrawal"
    );

    let result = async {
        let state_transition = build_withdraw_st(
            sdk,
            store,
            keys,
            output_script,
            amount,
            core_fee_per_byte,
            exact_fee,
            &selected_notes,
            prover,
        )
        .await?;

        trace!("Shielded withdrawal: state transition built, broadcasting...");
        broadcast_st(sdk, &state_transition).await
    }
    .await;

    match result {
        Ok(()) => {
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
        Err(e) => {
            cancel_pending(store, id, &selected_notes).await;
            Err(e)
        }
    }
}

// -------------------------------------------------------------------------
// Build seams (no broadcast)
// -------------------------------------------------------------------------

/// Build (extract witnesses + prove + sign) a Type-17 unshield state
/// transition WITHOUT broadcasting. `selected_notes` are the already-
/// reserved spend inputs and `exact_fee` the fee folded into the spend.
///
/// The capture seam for unshield: callers that need the serialized
/// transition take it here. The combined [`unshield`] wrapper handles
/// reservation + finalize/cancel around this build.
#[allow(clippy::too_many_arguments)]
pub async fn build_unshield_st<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    keys: &OrchardKeySet,
    to_address: &PlatformAddress,
    amount: u64,
    exact_fee: u64,
    selected_notes: &[ShieldedNote],
    prover: &P,
) -> Result<StateTransition, PlatformWalletError> {
    let change_addr = default_orchard_address(keys)?;
    let (spends, anchor) = extract_spends_and_anchor(store, selected_notes).await?;
    build_unshield_transition(
        spends,
        *to_address,
        amount,
        &change_addr,
        &keys.full_viewing_key,
        &keys.spend_auth_key,
        anchor,
        prover,
        [0u8; 36],
        Some(exact_fee),
        sdk.version(),
    )
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))
}

/// Build a Type-16 shielded-transfer state transition WITHOUT
/// broadcasting. Capture seam paralleling [`build_unshield_st`].
#[allow(clippy::too_many_arguments)]
pub async fn build_transfer_st<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    keys: &OrchardKeySet,
    recipient_addr: &OrchardAddress,
    amount: u64,
    exact_fee: u64,
    selected_notes: &[ShieldedNote],
    prover: &P,
) -> Result<StateTransition, PlatformWalletError> {
    let change_addr = default_orchard_address(keys)?;
    let (spends, anchor) = extract_spends_and_anchor(store, selected_notes).await?;
    build_shielded_transfer_transition(
        spends,
        recipient_addr,
        amount,
        &change_addr,
        &keys.full_viewing_key,
        &keys.spend_auth_key,
        anchor,
        prover,
        [0u8; 36],
        Some(exact_fee),
        sdk.version(),
    )
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))
}

/// Build a Type-19 shielded-withdrawal state transition WITHOUT
/// broadcasting. Capture seam paralleling [`build_unshield_st`].
#[allow(clippy::too_many_arguments)]
pub async fn build_withdraw_st<S: ShieldedStore, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    keys: &OrchardKeySet,
    output_script: CoreScript,
    amount: u64,
    core_fee_per_byte: u32,
    exact_fee: u64,
    selected_notes: &[ShieldedNote],
    prover: &P,
) -> Result<StateTransition, PlatformWalletError> {
    let change_addr = default_orchard_address(keys)?;
    let (spends, anchor) = extract_spends_and_anchor(store, selected_notes).await?;
    build_shielded_withdrawal_transition(
        spends,
        amount,
        output_script,
        core_fee_per_byte,
        Pooling::Standard,
        &change_addr,
        &keys.full_viewing_key,
        &keys.spend_auth_key,
        anchor,
        prover,
        [0u8; 36],
        Some(exact_fee),
        sdk.version(),
    )
    .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))
}

/// Broadcast a built shielded spend transition and wait for proven
/// execution. Shared by the unshield/transfer/withdraw/asset-lock
/// wrappers; maps the broadcast error to `ShieldedBroadcastFailed`.
pub async fn broadcast_st(
    sdk: &Arc<dash_sdk::Sdk>,
    state_transition: &StateTransition,
) -> Result<(), PlatformWalletError> {
    state_transition
        .broadcast_and_wait::<StateTransitionProofResult>(sdk, None)
        .await
        .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;
    Ok(())
}

// -------------------------------------------------------------------------
// Internal helpers (free fns)
// -------------------------------------------------------------------------

/// Convert `keys`'s default `PaymentAddress` to an `OrchardAddress`.
fn default_orchard_address(keys: &OrchardKeySet) -> Result<OrchardAddress, PlatformWalletError> {
    payment_address_to_orchard(&keys.default_address)
}

/// Extract `SpendableNote` structs with Merkle witnesses and the
/// tree anchor.
///
/// The anchor is derived from the witness paths themselves (via
/// `MerklePath::root(cmx)`) rather than from `store.tree_anchor()`.
/// The store's witness call is `witness_at_checkpoint_depth(0)`
/// (root of the most recent checkpoint) while `tree_anchor()` is
/// `root_at_checkpoint_depth(None)` (latest tree state) — any
/// commitments appended after the last checkpoint move the latter
/// ahead of the former, and the resulting `AnchorMismatch` from
/// the Orchard spend builder is what you'd see at proof time.
/// Using the witness's own computed root keeps the anchor
/// consistent with the authentication paths the proof actually
/// verifies.
async fn extract_spends_and_anchor<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    notes: &[ShieldedNote],
) -> Result<(Vec<SpendableNote>, Anchor), PlatformWalletError> {
    use grovedb_commitment_tree::ExtractedNoteCommitment;

    let store = store.read().await;

    let mut spends = Vec::with_capacity(notes.len());
    let mut anchor: Option<Anchor> = None;
    for note in notes {
        let orchard_note = deserialize_note(&note.note_data).ok_or_else(|| {
            PlatformWalletError::ShieldedBuildError(format!(
                "Failed to deserialize note at position {}",
                note.position
            ))
        })?;

        let merkle_path = store
            .witness(note.position)
            .map_err(|e| PlatformWalletError::ShieldedMerkleWitnessUnavailable(e.to_string()))?
            .ok_or_else(|| {
                PlatformWalletError::ShieldedMerkleWitnessUnavailable(format!(
                    "no witness available for note at position {} (not marked, or pruned past this position)",
                    note.position
                ))
            })?;

        // Compute the anchor this witness was generated against.
        // All selected notes must share the same anchor — if not,
        // the store handed us witnesses from different
        // checkpoints, which the spend builder would reject
        // downstream with `AnchorMismatch`. Surface the mismatch
        // here so the host doesn't pay the ~30 s proof cost
        // first.
        let cmx = ExtractedNoteCommitment::from_bytes(&note.cmx)
            .into_option()
            .ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "invalid stored cmx for note at position {}",
                    note.position
                ))
            })?;
        let witness_anchor = merkle_path.root(cmx);
        match &anchor {
            None => anchor = Some(witness_anchor),
            Some(prev) if prev.to_bytes() != witness_anchor.to_bytes() => {
                return Err(PlatformWalletError::ShieldedBuildError(format!(
                    "witness anchor mismatch across selected notes (position {})",
                    note.position
                )));
            }
            _ => {}
        }

        spends.push(SpendableNote {
            note: orchard_note,
            merkle_path,
        });
    }

    let anchor = anchor.ok_or_else(|| {
        PlatformWalletError::ShieldedBuildError(
            "no spendable notes selected — anchor undefined".to_string(),
        )
    })?;

    Ok((spends, anchor))
}

/// Mark the selected notes as spent for `id`. Also queues a
/// shielded changeset on the persister so the spent flag reaches
/// durable storage immediately rather than waiting for the next
/// nullifier-sync pass to rediscover the spend. Also drops any
/// matching pending reservation so the confirmed-spent state
/// and the in-flight-spend state can't disagree.
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
) -> Result<(Vec<ShieldedNote>, u64, u64), PlatformWalletError> {
    let mut store = store.write().await;
    let unspent = store
        .get_unspent_notes(id)
        .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    let (selected, total_input, exact_fee) =
        select_notes_with_fee(&unspent, amount, outputs, sdk.version())?.into_owned();
    for note in &selected {
        store
            .mark_pending(id, &note.nullifier)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
    }
    Ok((selected, total_input, exact_fee))
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
                "cancel_pending: clear_pending failed; the next nullifier sync will reconcile"
            );
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

// -------------------------------------------------------------------------
// Test-only spend-assembly seams (`test-utils` feature)
// -------------------------------------------------------------------------

/// Test-only re-exports of the spend-assembly internals the adversarial
/// e2e cases drive directly. Gated behind `test-utils` (pulled in by
/// `e2e`), NEVER in production builds — these bypass the wallet's spend
/// guards (reservation, balance, fee) by design so a test can build a
/// transition against a CHOSEN note (double-spend, replay,
/// intra-bundle-dup) and reach Drive.
#[cfg(feature = "test-utils")]
pub mod test_utils {
    use super::*;

    /// Reserve+select unspent notes (the production reservation path).
    /// Exposed so a test can observe / drive the reservation contract.
    pub async fn reserve_unspent_notes_for_test<S: ShieldedStore>(
        sdk: &Arc<dash_sdk::Sdk>,
        store: &Arc<RwLock<S>>,
        id: SubwalletId,
        amount: u64,
        outputs: usize,
    ) -> Result<(Vec<ShieldedNote>, u64, u64), PlatformWalletError> {
        super::reserve_unspent_notes(sdk, store, id, amount, outputs).await
    }

    /// Extract `SpendableNote`s + the tree anchor for a chosen note set,
    /// WITHOUT reserving. The skip-reservation seam: a test passes an
    /// already-spent or duplicated note to build a transition the wallet
    /// would never assemble, then broadcasts it to prove the BACKEND
    /// rejects (double-spend SH-020, replay SH-021, intra-bundle-dup
    /// SH-033).
    pub async fn extract_spends_and_anchor_for_test<S: ShieldedStore>(
        store: &Arc<RwLock<S>>,
        notes: &[ShieldedNote],
    ) -> Result<(Vec<SpendableNote>, Anchor), PlatformWalletError> {
        super::extract_spends_and_anchor(store, notes).await
    }

    /// All unspent notes for `id`, so a test can capture a note to build
    /// a second (double-spend / replay) transition against.
    pub async fn unspent_notes_for_test<S: ShieldedStore>(
        store: &Arc<RwLock<S>>,
        id: SubwalletId,
    ) -> Result<Vec<ShieldedNote>, PlatformWalletError> {
        let store = store.read().await;
        store
            .get_unspent_notes(id)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))
    }

    /// Derive the one-time asset-lock private key (32 secret bytes) from
    /// `(seed, path)`, where `path` is the `DerivationPath` the asset-lock
    /// builder returned alongside the proof.
    ///
    /// `shield_from_asset_lock` takes the key as `&[u8]`; the builder
    /// returns only the proof + path, so this mirrors the production
    /// seed → master xpriv → `derive_priv` derivation (see
    /// `core/broadcast.rs`) to materialize the key test-side for SH-018 /
    /// SH-035. Test-only — never materialize spend keys in production.
    pub fn derive_asset_lock_private_key(
        seed: &[u8],
        network: dashcore::Network,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<[u8; 32], PlatformWalletError> {
        use key_wallet::dashcore::secp256k1::Secp256k1;
        use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;

        let root_priv = RootExtendedPrivKey::new_master(seed).map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "derive_asset_lock_private_key: invalid seed: {e}"
            ))
        })?;
        let master = root_priv.to_extended_priv_key(network);
        let secp = Secp256k1::new();
        let derived = master.derive_priv(&secp, path).map_err(|e| {
            PlatformWalletError::ShieldedBuildError(format!(
                "derive_asset_lock_private_key: derive_priv: {e}"
            ))
        })?;
        Ok(derived.private_key.secret_bytes())
    }
}
