//! Shielded transaction operations (5 transition types), multi-account.
//!
//! Each operation now takes `account: u32` and routes through the
//! corresponding `OrchardKeySet` / `SubwalletId`. Spends never
//! cross account boundaries — note selection reads only the
//! given account's unspent notes.
//!
//! The five transition types are:
//! - **Shield** (Type 15): transparent platform addresses → shielded pool
//! - **ShieldFromAssetLock** (Type 18): Core L1 asset lock → shielded pool
//! - **Unshield** (Type 17): shielded pool → transparent platform address
//! - **Transfer** (Type 16): shielded pool → shielded pool (private)
//! - **Withdraw** (Type 19): shielded pool → Core L1 address

use super::note_selection::select_notes_with_fee;
use super::store::{ShieldedNote, ShieldedStore, SubwalletId};
use super::ShieldedWallet;
use crate::error::PlatformWalletError;

use std::collections::BTreeMap;

use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, OrchardAddress, PlatformAddress,
};
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::identity::signer::Signer;
use dpp::prelude::AssetLockProof;
use dpp::shielded::builder::{
    build_shield_from_asset_lock_transition, build_shield_transition,
    build_shielded_transfer_transition, build_shielded_withdrawal_transition,
    build_unshield_transition, OrchardProver, SpendableNote,
};
use dpp::withdrawal::Pooling;
use grovedb_commitment_tree::{Anchor, PaymentAddress};
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

impl<S: ShieldedStore> ShieldedWallet<S> {
    // -------------------------------------------------------------------------
    // Shield: platform addresses -> shielded pool (Type 15)
    // -------------------------------------------------------------------------

    /// Shield credits from transparent platform addresses into the
    /// shielded pool, with the resulting note assigned to `account`'s
    /// default Orchard payment address.
    pub async fn shield<Sig: Signer<PlatformAddress>, P: OrchardProver>(
        &self,
        account: u32,
        inputs: BTreeMap<PlatformAddress, Credits>,
        amount: u64,
        signer: &Sig,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let recipient_addr = self.default_orchard_address(account)?;

        // Fetch the current address nonces from Platform. Each
        // input address has a per-address nonce that the next
        // state transition must use as `last_used + 1`.
        use dash_sdk::platform::FetchMany;
        use dash_sdk::query_types::AddressInfo;
        use std::collections::BTreeSet;

        let address_set: BTreeSet<PlatformAddress> = inputs.keys().copied().collect();
        let infos = AddressInfo::fetch_many(&self.sdk, address_set)
            .await
            .map_err(|e| {
                PlatformWalletError::ShieldedBuildError(format!("fetch input nonces: {e}"))
            })?;

        let mut inputs_with_nonce: BTreeMap<PlatformAddress, (u32, Credits)> = BTreeMap::new();
        for (addr, credits) in inputs {
            let info = infos
                .get(&addr)
                .and_then(|opt| opt.as_ref())
                .ok_or_else(|| {
                    PlatformWalletError::ShieldedBuildError(format!(
                        "input address not found on platform: {:?}",
                        addr
                    ))
                })?;
            if info.balance < credits {
                warn!(
                    address = ?addr,
                    claimed_credits = credits,
                    platform_balance = info.balance,
                    platform_nonce = info.nonce,
                    "Shield input claims more credits than Platform reports — broadcast will likely fail"
                );
            } else {
                info!(
                    address = ?addr,
                    claimed_credits = credits,
                    platform_balance = info.balance,
                    platform_nonce = info.nonce,
                    "Shield input"
                );
            }
            // `AddressNonce` is `u32`; `info.nonce + 1` would
            // wrap silently in release once an address reaches
            // u32::MAX. drive-abci treats wrap-to-0 as a replay
            // and rejects it after the wallet has spent ~30 s on
            // a Halo 2 proof. Bail loudly here instead.
            let next_nonce = info.nonce.checked_add(1).ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "input address nonce exhausted on platform: {:?}",
                    addr
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
            self.sdk.version(),
        )
        .await
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shield credits: state transition built, broadcasting...");
        let network = self.sdk.network;
        state_transition
            .broadcast(&self.sdk, None)
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
    // ShieldFromAssetLock: Core L1 -> shielded pool (Type 18)
    // -------------------------------------------------------------------------

    /// Shield funds from a Core L1 asset lock directly into
    /// `account`'s shielded pool entry.
    pub async fn shield_from_asset_lock<P: OrchardProver>(
        &self,
        account: u32,
        asset_lock_proof: AssetLockProof,
        private_key: &[u8],
        amount: u64,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let recipient_addr = self.default_orchard_address(account)?;

        info!(
            account,
            credits = amount,
            "Shield from asset lock: building state transition"
        );

        let state_transition = build_shield_from_asset_lock_transition(
            &recipient_addr,
            amount,
            asset_lock_proof,
            private_key,
            prover,
            [0u8; 36],
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shield from asset lock: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        info!(
            account,
            credits = amount,
            "Shield from asset lock broadcast succeeded"
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Unshield: shielded pool -> platform address (Type 17)
    // -------------------------------------------------------------------------

    /// Unshield funds from `account`'s shielded notes to a
    /// transparent platform address.
    pub async fn unshield<P: OrchardProver>(
        &self,
        account: u32,
        to_address: &PlatformAddress,
        amount: u64,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let keys = self.keys_for(account)?;
        let change_addr = self.default_orchard_address(account)?;
        let id = self.subwallet_id(account);

        let (selected_notes, total_input, exact_fee) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes(id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            select_notes_with_fee(&unspent, amount, 1, self.sdk.version())?.into_owned()
        };

        info!(
            account,
            credits = amount,
            fee = exact_fee,
            inputs = selected_notes.len(),
            total_input,
            "Unshield"
        );

        let (spends, anchor) = self.extract_spends_and_anchor(&selected_notes).await?;

        let state_transition = build_unshield_transition(
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
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Unshield: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        self.mark_notes_spent(id, &selected_notes).await?;

        info!(account, credits = amount, "Unshield broadcast succeeded");
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Transfer: shielded pool -> shielded pool (Type 16)
    // -------------------------------------------------------------------------

    /// Transfer funds privately from `account`'s shielded notes
    /// to another Orchard payment address.
    pub async fn transfer<P: OrchardProver>(
        &self,
        account: u32,
        to_address: &PaymentAddress,
        amount: u64,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let keys = self.keys_for(account)?;
        let recipient_addr = payment_address_to_orchard(to_address)?;
        let change_addr = self.default_orchard_address(account)?;
        let id = self.subwallet_id(account);

        let (selected_notes, total_input, exact_fee) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes(id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            select_notes_with_fee(&unspent, amount, 2, self.sdk.version())?.into_owned()
        };

        info!(
            account,
            credits = amount,
            fee = exact_fee,
            inputs = selected_notes.len(),
            total_input,
            "Shielded transfer"
        );

        let (spends, anchor) = self.extract_spends_and_anchor(&selected_notes).await?;

        let state_transition = build_shielded_transfer_transition(
            spends,
            &recipient_addr,
            amount,
            &change_addr,
            &keys.full_viewing_key,
            &keys.spend_auth_key,
            anchor,
            prover,
            [0u8; 36],
            Some(exact_fee),
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shielded transfer: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        self.mark_notes_spent(id, &selected_notes).await?;

        info!(
            account,
            credits = amount,
            "Shielded transfer broadcast succeeded"
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Withdraw: shielded pool -> Core L1 address (Type 19)
    // -------------------------------------------------------------------------

    /// Withdraw funds from `account`'s shielded notes to a Core L1 address.
    pub async fn withdraw<P: OrchardProver>(
        &self,
        account: u32,
        to_address: &dashcore::Address,
        amount: u64,
        core_fee_per_byte: u32,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let keys = self.keys_for(account)?;
        let change_addr = self.default_orchard_address(account)?;
        let id = self.subwallet_id(account);
        let output_script = CoreScript::from_bytes(to_address.script_pubkey().to_bytes());

        let (selected_notes, total_input, exact_fee) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes(id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            select_notes_with_fee(&unspent, amount, 1, self.sdk.version())?.into_owned()
        };

        info!(
            account,
            credits = amount,
            fee = exact_fee,
            inputs = selected_notes.len(),
            total_input,
            "Shielded withdrawal"
        );

        let (spends, anchor) = self.extract_spends_and_anchor(&selected_notes).await?;

        let state_transition = build_shielded_withdrawal_transition(
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
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shielded withdrawal: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        self.mark_notes_spent(id, &selected_notes).await?;

        info!(
            account,
            credits = amount,
            "Shielded withdrawal broadcast succeeded"
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    /// Convert `account`'s default `PaymentAddress` to an `OrchardAddress`.
    fn default_orchard_address(&self, account: u32) -> Result<OrchardAddress, PlatformWalletError> {
        let keys = self.keys_for(account)?;
        payment_address_to_orchard(&keys.default_address)
    }

    /// Extract `SpendableNote` structs with Merkle witnesses and the
    /// tree anchor. The tree is shared per-network; only note
    /// selection is per-subwallet (already done by the caller).
    async fn extract_spends_and_anchor(
        &self,
        notes: &[ShieldedNote],
    ) -> Result<(Vec<SpendableNote>, Anchor), PlatformWalletError> {
        let store = self.store.read().await;

        let mut spends = Vec::with_capacity(notes.len());
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

            spends.push(SpendableNote {
                note: orchard_note,
                merkle_path,
            });
        }

        let anchor_bytes = store
            .tree_anchor()
            .map_err(|e| PlatformWalletError::ShieldedMerkleWitnessUnavailable(e.to_string()))?;
        let anchor = Anchor::from_bytes(anchor_bytes)
            .into_option()
            .ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(
                    "Invalid anchor bytes from commitment tree".to_string(),
                )
            })?;

        Ok((spends, anchor))
    }

    /// Mark the selected notes as spent for `id`. Also queues a
    /// shielded changeset on the persister so the spent flag
    /// reaches durable storage immediately rather than waiting for
    /// the next nullifier-sync pass to rediscover the spend.
    async fn mark_notes_spent(
        &self,
        id: SubwalletId,
        notes: &[ShieldedNote],
    ) -> Result<(), PlatformWalletError> {
        let mut changeset = crate::changeset::ShieldedChangeSet::default();
        {
            let mut store = self.store.write().await;
            for note in notes {
                if store
                    .mark_spent(id, &note.nullifier)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
                {
                    changeset.record_nullifier_spent(id, note.nullifier);
                }
            }
        }
        self.queue_shielded_changeset(changeset);
        Ok(())
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
