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

use super::keys::OrchardKeySet;
use super::note_selection::{
    select_notes_for_denomination, select_notes_with_fee, ShieldedFeeKind,
};
use super::store::{ShieldedNote, ShieldedStore, SubwalletId};
use crate::changeset::{PlatformWalletChangeSet, ShieldedChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;

use std::collections::BTreeMap;
use std::sync::Arc;

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
use dpp::withdrawal::Pooling;
use grovedb_commitment_tree::{Anchor, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{info, trace, warn};

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
pub async fn shield<Sig: Signer<PlatformAddress>, P: OrchardProver>(
    sdk: &Arc<dash_sdk::Sdk>,
    keys: &OrchardKeySet,
    account: u32,
    inputs: BTreeMap<PlatformAddress, Credits>,
    amount: u64,
    signer: &Sig,
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let recipient_addr = default_orchard_address(keys)?;

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
    // Wait for proven execution (not just relay-ACK) so the host only
    // sees success once Platform has actually included the transition —
    // matching the spend-side flows (unshield/transfer/withdraw). A
    // DAPI-level ACK alone could otherwise mask a later Platform
    // rejection. The proven result is discarded; we only need the
    // confirmation.
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
    let change_addr = default_orchard_address(keys)?;
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

    // From here on every error path must release the reservation
    // taken by `reserve_unspent_notes`.
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(store, &selected_notes).await?;

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

        trace!("Unshield: state transition built, broadcasting...");
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;
        Ok::<(), PlatformWalletError>(())
    }
    .await;

    match result {
        Ok(()) => {
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
    memo: [u8; 36],
    prover: &P,
) -> Result<(), PlatformWalletError> {
    let recipient_addr = payment_address_to_orchard(to_address)?;
    let change_addr = default_orchard_address(keys)?;
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

    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(store, &selected_notes).await?;

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

        trace!("Shielded transfer: state transition built, broadcasting...");
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;
        Ok::<(), PlatformWalletError>(())
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
    let change_addr = default_orchard_address(keys)?;
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

    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(store, &selected_notes).await?;

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

        trace!("Shielded withdrawal: state transition built, broadcasting...");
        state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;
        Ok::<(), PlatformWalletError>(())
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
    let change_addr = default_orchard_address(keys)?;
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

    // From here on every error path must release the reservation taken above.
    let result = async {
        let (spends, anchor) = extract_spends_and_anchor(store, &selected_notes).await?;

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

        // Broadcast (relay-ACK only). A failure here is definitive: the tx was NOT accepted, so the
        // spend never happened — map to `ShieldedBroadcastFailed` and let the outer match release
        // the reservation, exactly as before.
        st.broadcast(sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        // Wait for proven execution. Classify the failure:
        //   - `StateTransitionBroadcastError` = Platform DEFINITIVELY reported the transition's own
        //     execution error (it ran and was rejected on its merits). The identity does not exist;
        //     keep today's behavior (release the reservation via `ShieldedBroadcastFailed`).
        //   - any other error (DriveProofError / Proof / InvalidProvedResponse / TimeoutReached /
        //     DapiClientError / …) = AMBIGUOUS: the broadcast was accepted and the transition may
        //     have executed even though we couldn't fetch/verify its result proof (this is exactly
        //     the #3859 result-proof incident). Fall back to fetching the identity by its
        //     pre-derived id before deciding it doesn't exist.
        let proof_result = match st
            .wait_for_response::<StateTransitionProofResult>(sdk, None)
            .await
        {
            Ok(result) => result,
            Err(dash_sdk::Error::StateTransitionBroadcastError(e)) => {
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
        // very hazard this variant exists to prevent.
        Err(e @ PlatformWalletError::ShieldedBroadcastUnconfirmed { .. }) => Err(e),
        Err(e) => {
            cancel_pending(store, id, &selected_notes).await;
            Err(e)
        }
    }
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
