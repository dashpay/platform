//! Shielded transaction operations (5 transition types).
//!
//! Each operation follows a common pattern:
//! 1. Select spendable notes (if spending from the shielded pool)
//! 2. Get Merkle witnesses from the commitment tree
//! 3. Build Orchard bundle via DPP builder functions
//! 4. Broadcast the resulting state transition via SDK
//! 5. Mark spent notes (if any) in the store
//!
//! The five transition types are:
//! - **Shield** (Type 15): transparent platform addresses -> shielded pool
//! - **ShieldFromAssetLock** (Type 18): Core L1 asset lock -> shielded pool
//! - **Unshield** (Type 17): shielded pool -> transparent platform address
//! - **Transfer** (Type 16): shielded pool -> shielded pool (private)
//! - **Withdraw** (Type 19): shielded pool -> Core L1 address
//!
//! # Store requirements
//!
//! Spending operations (unshield, transfer, withdraw) require the store to
//! provide Merkle witness paths. The `ShieldedStore` trait needs a `witness()`
//! method for this -- see the TODO in `extract_spends_and_anchor()`.

use super::note_selection::select_notes_with_fee;
use super::store::{ShieldedNote, ShieldedStore};
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

/// Try to extract a structured `AddressesNotEnoughFundsError` from a
/// broadcast error so the shield path can format a diagnostic that
/// includes Platform's actual per-input view (nonce + balance) rather
/// than just the stringified message.
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
/// matching what the wallet UI shows so the same string can be used
/// to grep logs for a specific address.
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

    /// Shield funds from transparent platform addresses into the shielded pool.
    ///
    /// This is an output-only operation -- no notes are spent. Funds are deducted
    /// from the transparent input addresses and a new shielded note is created for
    /// this wallet's default payment address.
    ///
    /// # Parameters
    ///
    /// - `inputs` - Map of platform addresses to credits to spend from each
    /// - `amount` - Total amount to shield (in credits)
    /// - `signer` - Signs the transparent input witnesses (ECDSA)
    /// - `prover` - Orchard prover for Halo 2 proof generation
    pub async fn shield<Sig: Signer<PlatformAddress>, P: OrchardProver>(
        &self,
        inputs: BTreeMap<PlatformAddress, Credits>,
        amount: u64,
        signer: &Sig,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let recipient_addr = self.default_orchard_address()?;

        // Fetch the current address nonces from Platform. Each
        // input address has a per-address nonce that the next
        // state transition must use as `last_used + 1`.
        // `AddressInfo::fetch_many` returns the last-used nonce
        // (and current balance) per address; we increment it.
        // Without this the broadcast was rejected by drive-abci
        // because every shield transition tried to use nonce 0.
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
            // Surface a per-input diagnostic so the host can see what
            // we're claiming vs what Platform actually reports —
            // mismatches are the typical root cause of
            // `AddressesNotEnoughFundsError` on shield broadcast.
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
            // `AddressNonce` is `u32`; `info.nonce + 1` would panic in
            // debug and wrap in release once an address reaches the
            // ceiling. drive-abci treats `u32::MAX` as exhausted, so a
            // wrap submits nonce 0 and gets rejected as a replay
            // *after* the wallet has already spent ~30 s building the
            // Halo 2 proof. Bail loudly here instead.
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

        info!("Shield credits: {} credits, building proof...", amount,);

        // Snapshot what we're claiming so the diagnostic can show
        // local-claim vs platform-view side by side when broadcast
        // fails with `AddressesNotEnoughFundsError`. The map is
        // moved into the builder below so we have to clone here.
        let claimed_inputs = inputs_with_nonce.clone();

        // Build the state transition using the DPP builder.
        // `build_shield_transition` is async (cascade from the dpp
        // `Signer` trait being made async upstream); await before
        // mapping the error.
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

        // Broadcast
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

        info!("Shield credits broadcast succeeded: {} credits", amount);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // ShieldFromAssetLock: Core L1 -> shielded pool (Type 18)
    // -------------------------------------------------------------------------

    /// Shield funds from a Core L1 asset lock directly into the shielded pool.
    ///
    /// The asset lock proof proves ownership of L1 funds. The ECDSA signature
    /// from the private key binds those funds to the Orchard bundle.
    ///
    /// # Parameters
    ///
    /// - `asset_lock_proof` - Proof that funds are locked on the Core chain
    /// - `private_key` - Private key for the asset lock (signs the transition)
    /// - `amount` - Amount to shield (in credits)
    /// - `prover` - Orchard prover for Halo 2 proof generation
    pub async fn shield_from_asset_lock<P: OrchardProver>(
        &self,
        asset_lock_proof: AssetLockProof,
        private_key: &[u8],
        amount: u64,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let recipient_addr = self.default_orchard_address()?;

        info!(
            "Shield from asset lock: building state transition for {} credits",
            amount,
        );

        let state_transition = build_shield_from_asset_lock_transition(
            &recipient_addr,
            amount,
            asset_lock_proof,
            private_key,
            prover,
            [0u8; 36], // empty memo
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shield from asset lock: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        info!(
            "Shield from asset lock broadcast succeeded: {} credits",
            amount,
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Unshield: shielded pool -> platform address (Type 17)
    // -------------------------------------------------------------------------

    /// Unshield funds from the shielded pool to a transparent platform address.
    ///
    /// Selects notes to cover the requested amount plus fee, builds the Orchard
    /// bundle with spend proofs, and broadcasts the state transition.
    ///
    /// # Parameters
    ///
    /// - `to_address` - Platform address to receive the unshielded funds
    /// - `amount` - Amount to unshield (in credits)
    /// - `prover` - Orchard prover for Halo 2 proof generation
    pub async fn unshield<P: OrchardProver>(
        &self,
        to_address: &PlatformAddress,
        amount: u64,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let change_addr = self.default_orchard_address()?;

        // Select notes with fee convergence (min 1 action for unshield change output)
        let (selected_notes, total_input, exact_fee) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes()
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            select_notes_with_fee(&unspent, amount, 1, self.sdk.version())?.into_owned()
        };

        info!(
            "Unshield: {} credits, fee {} credits, spending {} input note(s), total {} credits",
            amount,
            exact_fee,
            selected_notes.len(),
            total_input,
        );

        // Build SpendableNote structs with Merkle witnesses
        let (spends, anchor) = self.extract_spends_and_anchor(&selected_notes).await?;

        let state_transition = build_unshield_transition(
            spends,
            *to_address,
            amount,
            &change_addr,
            &self.keys.full_viewing_key,
            &self.keys.spend_auth_key,
            anchor,
            prover,
            [0u8; 36], // empty memo
            Some(exact_fee),
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Unshield: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        // Mark spent notes in store
        self.mark_notes_spent(&selected_notes).await?;

        info!("Unshield broadcast succeeded: {} credits", amount);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Transfer: shielded pool -> shielded pool (Type 16)
    // -------------------------------------------------------------------------

    /// Transfer funds privately within the shielded pool.
    ///
    /// Both input and output are shielded -- an observer learns nothing about
    /// the sender, recipient, or amount.
    ///
    /// # Parameters
    ///
    /// - `to_address` - Recipient's Orchard payment address
    /// - `amount` - Amount to transfer (in credits)
    /// - `prover` - Orchard prover for Halo 2 proof generation
    pub async fn transfer<P: OrchardProver>(
        &self,
        to_address: &PaymentAddress,
        amount: u64,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let recipient_addr = payment_address_to_orchard(to_address)?;
        let change_addr = self.default_orchard_address()?;

        // Select notes with fee convergence (min 2 actions: recipient + change)
        let (selected_notes, total_input, exact_fee) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes()
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            select_notes_with_fee(&unspent, amount, 2, self.sdk.version())?.into_owned()
        };

        info!(
            "Shielded transfer: {} credits, fee {} credits, spending {} input note(s), total {} credits",
            amount,
            exact_fee,
            selected_notes.len(),
            total_input,
        );

        let (spends, anchor) = self.extract_spends_and_anchor(&selected_notes).await?;

        let state_transition = build_shielded_transfer_transition(
            spends,
            &recipient_addr,
            amount,
            &change_addr,
            &self.keys.full_viewing_key,
            &self.keys.spend_auth_key,
            anchor,
            prover,
            [0u8; 36], // empty memo
            Some(exact_fee),
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shielded transfer: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        self.mark_notes_spent(&selected_notes).await?;

        info!("Shielded transfer broadcast succeeded: {} credits", amount);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Withdraw: shielded pool -> Core L1 address (Type 19)
    // -------------------------------------------------------------------------

    /// Withdraw funds from the shielded pool to a Core L1 address.
    ///
    /// Spends shielded notes and creates a withdrawal to the specified Core
    /// chain address. The withdrawal uses standard pooling by default.
    ///
    /// # Parameters
    ///
    /// - `to_address` - Core chain address to receive the withdrawal
    /// - `amount` - Amount to withdraw (in credits)
    /// - `core_fee_per_byte` - Core chain fee rate (duffs per byte)
    /// - `prover` - Orchard prover for Halo 2 proof generation
    pub async fn withdraw<P: OrchardProver>(
        &self,
        to_address: &dashcore::Address,
        amount: u64,
        core_fee_per_byte: u32,
        prover: &P,
    ) -> Result<(), PlatformWalletError> {
        let change_addr = self.default_orchard_address()?;
        let output_script = CoreScript::from_bytes(to_address.script_pubkey().to_bytes());

        // Select notes with fee convergence (min 1 action for change output)
        let (selected_notes, total_input, exact_fee) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes()
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            select_notes_with_fee(&unspent, amount, 1, self.sdk.version())?.into_owned()
        };

        info!(
            "Shielded withdrawal: {} credits, fee {} credits, spending {} input note(s), total {} credits",
            amount,
            exact_fee,
            selected_notes.len(),
            total_input,
        );

        let (spends, anchor) = self.extract_spends_and_anchor(&selected_notes).await?;

        let state_transition = build_shielded_withdrawal_transition(
            spends,
            amount,
            output_script,
            core_fee_per_byte,
            Pooling::Standard,
            &change_addr,
            &self.keys.full_viewing_key,
            &self.keys.spend_auth_key,
            anchor,
            prover,
            [0u8; 36], // empty memo
            Some(exact_fee),
            self.sdk.version(),
        )
        .map_err(|e| PlatformWalletError::ShieldedBuildError(e.to_string()))?;

        trace!("Shielded withdrawal: state transition built, broadcasting...");
        state_transition
            .broadcast(&self.sdk, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedBroadcastFailed(e.to_string()))?;

        self.mark_notes_spent(&selected_notes).await?;

        info!(
            "Shielded withdrawal broadcast succeeded: {} credits",
            amount
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    /// Convert this wallet's default PaymentAddress to an OrchardAddress.
    fn default_orchard_address(&self) -> Result<OrchardAddress, PlatformWalletError> {
        payment_address_to_orchard(&self.keys.default_address)
    }

    /// Extract SpendableNote structs with Merkle witnesses and the tree anchor.
    ///
    /// Reads the commitment tree from the store, computes a Merkle path for each
    /// selected note, and returns them alongside the current tree anchor.
    ///
    /// # Note
    ///
    /// This method requires the `ShieldedStore` to support `witness()` for
    /// generating Merkle paths. If the store trait does not yet include this
    /// method, it needs to be added. The spec defines:
    /// ```ignore
    /// fn witness(&self, position: u64) -> Result<MerklePath, Self::Error>;
    /// ```
    /// Until that method is added, this will not compile.
    #[allow(clippy::never_loop, unused_mut)]
    async fn extract_spends_and_anchor(
        &self,
        notes: &[ShieldedNote],
    ) -> Result<(Vec<SpendableNote>, Anchor), PlatformWalletError> {
        let store = self.store.read().await;

        let mut spends = Vec::with_capacity(notes.len());
        for note in notes {
            // Deserialize the stored note back to an Orchard Note
            let orchard_note = deserialize_note(&note.note_data).ok_or_else(|| {
                PlatformWalletError::ShieldedBuildError(format!(
                    "Failed to deserialize note at position {}",
                    note.position
                ))
            })?;

            // Get Merkle witness for this note position.
            // The ShieldedStore trait returns Vec<u8> to avoid coupling the trait
            // to the MerklePath type. Production implementations should store the
            // witness bytes from ClientPersistentCommitmentTree::witness().
            //
            // TODO: MerklePath doesn't implement serde traits, so we can't
            // deserialize from bytes generically. The real fix is to either:
            // (a) Make ShieldedStore return MerklePath directly (couples to orchard), or
            // (b) Add a witness_for_spend() method that returns SpendableNote directly.
            // For now, spending operations require a store that provides valid witnesses.
            let _witness_bytes = store.witness(note.position).map_err(|e| {
                PlatformWalletError::ShieldedMerkleWitnessUnavailable(e.to_string())
            })?;

            // TODO: Convert witness bytes to MerklePath and build SpendableNote.
            // MerklePath doesn't implement serde, so this requires either:
            // (a) coupling ShieldedStore to MerklePath type, or
            // (b) a higher-level method that returns SpendableNote directly.
            // For now, spending operations are not yet functional.
            let _note = orchard_note;
            return Err(PlatformWalletError::ShieldedBuildError(
                "Spending operations require a ShieldedStore that provides MerklePath witnesses. Not yet implemented.".to_string(),
            ));
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

    /// Mark selected notes as spent in the store.
    async fn mark_notes_spent(&self, notes: &[ShieldedNote]) -> Result<(), PlatformWalletError> {
        let mut store = self.store.write().await;

        for note in notes {
            store
                .mark_spent(&note.nullifier)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        }

        Ok(())
    }
}

/// Helper trait extension for note selection results that need to own the data.
///
/// When note selection is performed inside a store lock scope, we need to
/// clone the results so they can outlive the lock.
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

/// Convert a PaymentAddress to an OrchardAddress for the DPP builder functions.
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
/// Format: `recipient(43) || value(8 LE) || rho(32) || rseed(32)`
///
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
