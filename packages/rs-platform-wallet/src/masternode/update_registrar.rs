//! ProUpRegTx (provider update registrar) orchestration.
//!
//! Rotates a masternode's operator and/or voting key to fresh wallet keys,
//! authorized by the immutable owner key. The payload commits to the
//! funding inputs (`inputs_hash`) and carries a 65-byte compact recoverable
//! ECDSA signature by the owner key over `base_payload_hash()` — Core's
//! `CHashSigner` convention, pinned by the real testnet vector in
//! dashcore's `provider_update_registrar` tests — so the build order is the
//! same as the update-service path: select and reserve inputs → write
//! `inputs_hash` → compact-sign the payload → ECDSA-sign the inputs →
//! broadcast, riding key-wallet's payload-finalizer seam.
//!
//! Consensus consequence callers must plan for: when the operator key
//! changes, Core RESETS the entry's service fields and PoSe-bans the node
//! until the NEW operator broadcasts a ProUpServTx. Rotating the operator
//! key is therefore a two-stage flow; stage two is the explicit-values
//! update-service in this crate's sibling module. A voting-only (or
//! payout-only) update has no such reset.

use dashcore::blockdata::script::ScriptBuf;
use dashcore::blockdata::transaction::special_transaction::provider_update_registrar::ProviderUpdateRegistrarPayload;
use dashcore::blockdata::transaction::special_transaction::{
    SpecialTransactionBasePayloadEncodable, TransactionPayload,
};
use dashcore::bls_sig_utils::BLSPublicKey;
use dashcore::hashes::{hash160, Hash};
use dashcore::secp256k1::{Message, Secp256k1, SecretKey};
use dashcore::{Address as DashAddress, Network, PubkeyHash, Txid};
use key_wallet::wallet::managed_wallet_info::transaction_builder::{
    BuilderError, TransactionBuilder, TransactionSigner,
};
use zeroize::Zeroizing;

use super::list::MasternodeListSummary;
use super::locator::p2pkh_script_hash;
use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderRegistrationPayload;

use super::update_service::{
    display_hex, fetch_registration_transaction, fetch_transaction_checked,
    registration_payload_from_fetched, require_standard_payout_script,
};
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;
use crate::spv::SpvRuntime;
use crate::wallet::core::{CoreWallet, SignedCoreTransaction, SEND_FUNDING_SOURCES};
use crate::wallet::platform_wallet::PlatformWallet;
use crate::wallet::provider_key_at_index::ProviderKeyKind;

/// What a registrar update lets the caller change. `None` keeps the
/// entry's current value (the payload always carries a full field set, so
/// "keep" means "copy from the live list entry").
#[derive(Debug, Clone)]
pub struct MasternodeUpdateRegistrarParams {
    /// ProRegTx hash of the masternode to update, in WIRE order.
    pub pro_tx_hash: [u8; 32],
    /// Wallet `ProviderOperatorKeys` index for the NEW operator key, or
    /// `None` to keep the current operator key. Changing the operator key
    /// PoSe-bans the node with its service fields reset until a
    /// ProUpServTx from the new key reactivates it.
    pub new_operator_key_index: Option<u32>,
    /// Wallet `ProviderVotingKeys` index for the NEW voting key, or `None`
    /// to keep the current voting key.
    pub new_voting_key_index: Option<u32>,
    /// Owner payout address. Always required: the payload REPLACES the
    /// payout script on-chain, so the caller must confirm it explicitly —
    /// an empty script is refused outright.
    pub payout_address: String,
}

/// The owner's secp256k1 secret plus whether its public key is the
/// compressed form — the compact-signature header byte encodes it, and a
/// wrong flag makes recovery resolve to a different key id.
pub struct OwnerSecret {
    pub secret: Zeroizing<[u8; 32]>,
    pub compressed: bool,
}

/// Build, owner-sign, fund, input-sign, and broadcast a ProUpRegTx — the
/// transaction Core produces for `protx update_registrar`.
pub async fn execute_masternode_update_registrar<S: TransactionSigner + ?Sized + Sync>(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
    params: MasternodeUpdateRegistrarParams,
    owner: OwnerSecret,
    signer: &S,
) -> Result<Txid, PlatformWalletError> {
    let signed = prepare_masternode_update_registrar(wallet, spv, params, owner, signer).await?;
    wallet.core().broadcast_finalized_transaction(&signed).await
}

/// Everything [`execute_masternode_update_registrar`] does except the
/// broadcast, for hosts that show the transaction first. The returned
/// transaction holds its funding inputs reserved; broadcast or abandon it.
pub async fn prepare_masternode_update_registrar<S: TransactionSigner + ?Sized + Sync>(
    wallet: &PlatformWallet,
    spv: &SpvRuntime,
    params: MasternodeUpdateRegistrarParams,
    owner: OwnerSecret,
    signer: &S,
) -> Result<SignedCoreTransaction, PlatformWalletError> {
    let summaries = spv
        .masternode_list_summaries()
        .await
        .ok_or(PlatformWalletError::MasternodeListUnavailable)?;

    // The owner key is immutable — set at registration, never rotatable —
    // so the ProRegTx's keyIDOwner is the authority the supplied secret is
    // verified against (the masternode list does not carry it). The full
    // transaction is fetched because an internal collateral is one of its
    // own outputs.
    let registration_tx = fetch_registration_transaction(wallet, &params.pro_tx_hash).await?;
    let registration =
        registration_payload_from_fetched(&params.pro_tx_hash, registration_tx.clone())?;
    let collateral_script =
        resolve_collateral_script(wallet, &registration_tx, &registration).await?;

    let (placeholder, owner) = assemble_update_registrar_placeholder_from_async(
        wallet.clone(),
        summaries,
        registration,
        collateral_script,
        params,
        owner,
    )
    .await?;

    build_sign_update_registrar(wallet.core(), placeholder, owner, signer).await
}

/// The assembly step as the async orchestrator must run it: on the
/// blocking pool. The assembly derives wallet keys, and
/// [`PlatformWallet::derive_provider_key_at_index`] takes the
/// wallet-manager lock via tokio's `blocking_read`, which panics when
/// called on an async worker thread — exactly where the FFI's
/// `block_on_worker` polls this future. The owner secret rides through and
/// comes back so the caller can keep signing with it.
async fn assemble_update_registrar_placeholder_from_async(
    wallet: PlatformWallet,
    summaries: Vec<MasternodeListSummary>,
    registration: ProviderRegistrationPayload,
    collateral_script: ScriptBuf,
    params: MasternodeUpdateRegistrarParams,
    owner: OwnerSecret,
) -> Result<(ProviderUpdateRegistrarPayload, OwnerSecret), PlatformWalletError> {
    tokio::task::spawn_blocking(move || {
        let placeholder = assemble_update_registrar_placeholder(
            &wallet,
            &summaries,
            &registration,
            &collateral_script,
            &params,
            &owner,
        )?;
        Ok((placeholder, owner))
    })
    .await
    .map_err(|join_error| {
        PlatformWalletError::KeyDerivation(format!(
            "the registrar assembly task did not complete: {join_error}"
        ))
    })?
}

/// Everything between the network reads and the funding build: resolve the
/// live entry, verify the owner secret, resolve every payload field, run
/// the consensus preflights, and assemble the placeholder payload. Split
/// out so the whole wiring is testable without SPV or DAPI — the public
/// orchestrator adds only the three fetches around it.
pub(crate) fn assemble_update_registrar_placeholder(
    wallet: &PlatformWallet,
    summaries: &[MasternodeListSummary],
    registration: &ProviderRegistrationPayload,
    collateral_script: &ScriptBuf,
    params: &MasternodeUpdateRegistrarParams,
    owner: &OwnerSecret,
) -> Result<ProviderUpdateRegistrarPayload, PlatformWalletError> {
    if params.new_operator_key_index.is_none() && params.new_voting_key_index.is_none() {
        return Err(PlatformWalletError::InvalidParameter(
            "nothing to rotate: neither a new operator key nor a new voting key was chosen"
                .to_string(),
        ));
    }

    let entry = summaries
        .iter()
        .find(|entry| entry.pro_tx_hash == params.pro_tx_hash)
        .ok_or_else(|| {
            PlatformWalletError::InvalidParameter(format!(
                "masternode {} is not in the masternode list",
                display_hex(&params.pro_tx_hash)
            ))
        })?;

    // Rotating the operator key erases the entry's service values, and
    // stage two re-asserts a single address — which would downgrade a v3
    // extended-net-info entry's endpoint map. Fail closed, exactly like the
    // unban path. A voting-only update touches no service state.
    if params.new_operator_key_index.is_some() && entry.has_extended_net_info {
        return Err(PlatformWalletError::InvalidParameter(
            "this masternode advertises v3 extended network info; rotating its operator key \
             would require re-asserting a single service address and discard its endpoint \
             map, so it cannot be rotated from this wallet yet"
                .to_string(),
        ));
    }

    verify_owner_secret(&registration.owner_key_hash, owner)?;

    let script_payout = resolve_owner_payout_script(&params.payout_address, wallet.network())?;

    // Resolve the payload's full field set: chosen fresh wallet keys where
    // the caller rotates, the live entry's values where it keeps.
    let operator_public_key = match params.new_operator_key_index {
        Some(index) => {
            let derived = wallet.derive_provider_key_at_index(
                ProviderKeyKind::Operator,
                index,
                None,
                false,
            )?;
            let bytes: [u8; 48] = derived
                .public_key_bytes
                .as_slice()
                .try_into()
                .map_err(|_| {
                    PlatformWalletError::KeyDerivation(
                        "derived operator public key is not 48 bytes".to_string(),
                    )
                })?;
            let legacy: Option<[u8; 48]> = derived
                .legacy_public_key_bytes
                .as_deref()
                .and_then(|b| b.try_into().ok());
            ensure_operator_key_unused(summaries, &bytes, legacy.as_ref())?;
            bytes
        }
        None => normalize_operator_key_to_basic(
            &entry.operator_public_key,
            entry.operator_key_is_legacy,
        )?,
    };
    let voting_key_hash = match params.new_voting_key_index {
        Some(index) => {
            let derived =
                wallet.derive_provider_key_at_index(ProviderKeyKind::Voting, index, None, false)?;
            hash160::Hash::hash(&derived.public_key_bytes).to_byte_array()
        }
        None => entry.voting_key_id,
    };

    // Consensus rejects a payout paid to the owner key or the payload's
    // (final) voting key (`bad-protx-payee-reuse`) — refuse before funding.
    ensure_payout_not_reusing_keys(
        &script_payout,
        &registration.owner_key_hash,
        &voting_key_hash,
    )?;
    ensure_collateral_not_reused(
        collateral_script,
        &registration.owner_key_hash,
        &voting_key_hash,
        &script_payout,
        entry.has_extended_net_info,
    )?;

    Ok(ProviderUpdateRegistrarPayload::new(
        Txid::from_byte_array(params.pro_tx_hash),
        0, // provider_mode — 0 is the only defined mode
        BLSPublicKey::from(operator_public_key),
        PubkeyHash::from_byte_array(voting_key_hash),
        script_payout,
        dashcore::hash_types::InputsHash::all_zeros(),
        Vec::new(),
    ))
}

/// The collateral UTXO's scriptPubKey. A null collateral txid means the
/// collateral is internal — an output of the ProRegTx itself; otherwise it
/// is an output of a separately fetched transaction, txid-bound before
/// anything is read from it. The script at that outpoint is immutable
/// history, and the entry's presence in the live list proves the outpoint
/// is unspent.
async fn resolve_collateral_script(
    wallet: &PlatformWallet,
    registration_tx: &dashcore::Transaction,
    registration: &ProviderRegistrationPayload,
) -> Result<ScriptBuf, PlatformWalletError> {
    let outpoint = registration.collateral_outpoint;
    if outpoint.txid == Txid::all_zeros() {
        collateral_output_script(registration_tx, outpoint.vout)
    } else {
        let external = fetch_transaction_checked(
            wallet,
            &outpoint.txid.to_byte_array(),
            "collateral transaction",
        )
        .await?;
        collateral_output_script(&external, outpoint.vout)
    }
}

/// Output `vout`'s scriptPubKey, refusing an out-of-range index.
pub(crate) fn collateral_output_script(
    transaction: &dashcore::Transaction,
    vout: u32,
) -> Result<ScriptBuf, PlatformWalletError> {
    transaction
        .output
        .get(vout as usize)
        .map(|output| output.script_pubkey.clone())
        .ok_or_else(|| {
            PlatformWalletError::InvalidIdentityData(format!(
                "the collateral outpoint index {vout} is out of range for its transaction"
            ))
        })
}

/// Core resolves the masternode's collateral UTXO and rejects a ProUpRegTx
/// whose final voting key (or the immutable owner key) is the collateral's
/// key destination (`bad-protx-collateral-reuse`) — the rule keeping the
/// collateral key off an online voting server. `ExtractDestination` maps
/// both a P2PKH collateral and a valid P2PK one to a `PKHash` — the latter
/// by hashing the public key in its original (compressed or uncompressed)
/// script serialization — so both forms are extracted here. Candidate
/// discovery joins wallet keys against DML voting fields only, so a key
/// whose address once funded this node's collateral looks unused there;
/// this is the check that stops it before funding. From v3 (ExtAddr)
/// entries Core also rejects a payout script equal to the collateral script
/// (`bad-protx-payee-reuse`); this payload is version 2, so that arm
/// applies exactly when the ENTRY is v3 — Core gates on `max(entry version,
/// payload version)`, and an entry advertises extended net info exactly
/// when it is v3+.
pub(crate) fn ensure_collateral_not_reused(
    collateral_script: &ScriptBuf,
    owner_key_hash: &PubkeyHash,
    final_voting_key_hash: &[u8; 20],
    script_payout: &ScriptBuf,
    entry_is_v3: bool,
) -> Result<(), PlatformWalletError> {
    let collateral_key = p2pkh_script_hash(collateral_script.as_bytes()).or_else(|| {
        collateral_script
            .p2pk_public_key()
            .map(|public_key| public_key.pubkey_hash().to_byte_array())
    });
    if let Some(collateral_key) = collateral_key {
        // The owner key is immutable, so its collision has no remedy the
        // caller can apply — unlike the voting key, which can be re-chosen.
        if collateral_key == owner_key_hash.to_byte_array() {
            return Err(PlatformWalletError::InvalidParameter(
                "the masternode's immutable owner key is its collateral address — consensus \
                 rejects reusing the collateral key (`bad-protx-collateral-reuse`), so this \
                 masternode cannot be updated with a ProUpRegTx"
                    .to_string(),
            ));
        }
        if collateral_key == *final_voting_key_hash {
            return Err(PlatformWalletError::InvalidParameter(
                "the chosen voting key is the masternode's collateral address — consensus \
                 rejects reusing the collateral key (`bad-protx-collateral-reuse`); pick a \
                 different voting key"
                    .to_string(),
            ));
        }
    }
    if entry_is_v3 && script_payout == collateral_script {
        return Err(PlatformWalletError::InvalidParameter(
            "the payout address is the masternode's collateral address — consensus rejects \
             paying the payout to the collateral (`bad-protx-payee-reuse`); pick a different \
             payout address"
                .to_string(),
        ));
    }
    Ok(())
}

/// Refuse an owner secret whose public key hash does not match the
/// ProRegTx's immutable `keyIDOwner`, before any signing or network work.
pub(crate) fn verify_owner_secret(
    expected_owner_key_hash: &PubkeyHash,
    owner: &OwnerSecret,
) -> Result<(), PlatformWalletError> {
    let secp = Secp256k1::new();
    let secret = SecretKey::from_byte_array(&owner.secret).map_err(|_| {
        PlatformWalletError::InvalidParameter(
            "the owner key is not a valid secp256k1 private key".to_string(),
        )
    })?;
    let public = secret.public_key(&secp);
    let serialized: Vec<u8> = if owner.compressed {
        public.serialize().to_vec()
    } else {
        public.serialize_uncompressed().to_vec()
    };
    let hash = hash160::Hash::hash(&serialized);
    if hash.to_byte_array() != expected_owner_key_hash.to_byte_array() {
        return Err(PlatformWalletError::InvalidParameter(
            "the owner key does not match this masternode's registered owner key".to_string(),
        ));
    }
    Ok(())
}

/// The payout rule for a registrar update: the payload replaces the owner
/// payout script on-chain, so the address is always required and confirmed
/// by the caller — never defaulted, never empty.
pub(crate) fn resolve_owner_payout_script(
    payout_address: &str,
    network: Network,
) -> Result<ScriptBuf, PlatformWalletError> {
    let trimmed = payout_address.trim();
    if trimmed.is_empty() {
        return Err(PlatformWalletError::InvalidParameter(
            "the payout address is required: the update replaces the payout script on-chain, \
             and an empty script would clear it"
                .to_string(),
        ));
    }
    let address = trimmed
        .parse::<DashAddress<dashcore::address::NetworkUnchecked>>()
        .map_err(|e| {
            PlatformWalletError::InvalidParameter(format!(
                "payout address is not a valid Dash address: {e}"
            ))
        })?
        .require_network(network)
        .map_err(|e| {
            PlatformWalletError::InvalidParameter(format!(
                "payout address is for another network: {e}"
            ))
        })?;
    let script = address.script_pubkey();
    require_standard_payout_script(&script)?;
    Ok(script)
}

/// Refuse a candidate operator key already registered to any masternode —
/// operator keys are consensus-unique across the whole list, so a duplicate
/// would make the ProUpRegTx invalid. The list may hold either
/// serialization of a key, so both forms are checked.
pub(crate) fn ensure_operator_key_unused(
    summaries: &[MasternodeListSummary],
    candidate: &[u8; 48],
    candidate_legacy: Option<&[u8; 48]>,
) -> Result<(), PlatformWalletError> {
    let clash = summaries.iter().find(|entry| {
        entry.operator_public_key == *candidate
            || candidate_legacy.is_some_and(|legacy| entry.operator_public_key == *legacy)
    });
    if let Some(entry) = clash {
        return Err(PlatformWalletError::InvalidParameter(format!(
            "the chosen operator key is already used by masternode {} — operator keys must \
             be unique; pick an unused key",
            display_hex(&entry.pro_tx_hash)
        )));
    }
    Ok(())
}

/// Consensus rejects a P2PKH payout paid to the owner key or the payload's
/// final voting key (`bad-protx-payee-reuse`). Both hashes are known before
/// funding — the immutable owner hash from the ProRegTx, the voting hash
/// from the payload being built — so a doomed transaction is refused here.
/// (P2SH payouts carry a script hash, not a key id, and cannot collide.)
pub(crate) fn ensure_payout_not_reusing_keys(
    script_payout: &ScriptBuf,
    owner_key_hash: &PubkeyHash,
    final_voting_key_hash: &[u8; 20],
) -> Result<(), PlatformWalletError> {
    let Some(payee) = p2pkh_script_hash(script_payout.as_bytes()) else {
        return Ok(());
    };
    if payee == owner_key_hash.to_byte_array() {
        return Err(PlatformWalletError::InvalidParameter(
            "the payout address is the masternode's owner address — consensus rejects paying \
             the payout to the owner key; pick a different payout address"
                .to_string(),
        ));
    }
    if payee == *final_voting_key_hash {
        return Err(PlatformWalletError::InvalidParameter(
            "the payout address is the masternode's voting address — consensus rejects paying \
             the payout to the voting key; pick a different payout address"
                .to_string(),
        ));
    }
    Ok(())
}

/// A kept (not rotated) operator key re-enters a version-2 payload, which
/// Core deserializes under the BASIC scheme — but a version-1 (pre-v19)
/// list entry carries the key in the LEGACY serialization. The two
/// serializations of one point differ only in flag bits (legacy bytes can
/// even parse "successfully" under the basic scheme as a different
/// reading), so the entry's version — not the bytes — decides: a legacy
/// key is parsed under Legacy and reserialized to basic; a basic key is
/// validated and passed through.
pub(crate) fn normalize_operator_key_to_basic(
    bytes: &[u8; 48],
    is_legacy: bool,
) -> Result<[u8; 48], PlatformWalletError> {
    use dashcore::blsful::{Bls12381G2Impl, PublicKey as BlsPubKey, SerializationFormat};
    let format = if is_legacy {
        SerializationFormat::Legacy
    } else {
        SerializationFormat::Modern
    };
    let key = BlsPubKey::<Bls12381G2Impl>::from_bytes_with_mode(bytes, format).map_err(|_| {
        PlatformWalletError::InvalidParameter(
            "the masternode's current operator key could not be parsed in the entry's BLS \
             serialization"
                .to_string(),
        )
    })?;
    if !is_legacy {
        return Ok(*bytes);
    }
    key.to_bytes().as_slice().try_into().map_err(|_| {
        PlatformWalletError::KeyDerivation("reserialized operator key is not 48 bytes".to_string())
    })
}

/// Compact recoverable ECDSA over `base_payload_hash`, in Core's
/// `CHashSigner` form: `[27 + recovery_id + (compressed ? 4 : 0)] ‖ r ‖ s`
/// (65 bytes) — the hash is signed directly, with no message prefix. The
/// real testnet ProUpRegTx vector's signature starts `0x1f` = 31 =
/// 27 + 0 + 4, confirming the convention.
pub(crate) fn owner_compact_signature(
    payload: &ProviderUpdateRegistrarPayload,
    owner: &OwnerSecret,
) -> Result<Vec<u8>, PlatformWalletError> {
    let secp = Secp256k1::new();
    let secret = SecretKey::from_byte_array(&owner.secret).map_err(|_| {
        PlatformWalletError::InvalidParameter(
            "the owner key is not a valid secp256k1 private key".to_string(),
        )
    })?;
    let digest = payload.base_payload_hash().to_byte_array();
    let message = Message::from_digest(digest);
    let recoverable = secp.sign_ecdsa_recoverable(&message, &secret);
    let (recovery_id, compact) = recoverable.serialize_compact();
    let mut signature = Vec::with_capacity(65);
    signature.push(27 + i32::from(recovery_id) as u8 + if owner.compressed { 4 } else { 0 });
    signature.extend_from_slice(&compact);
    Ok(signature)
}

/// Fund and finalize the ProUpRegTx: input selection reserves the funding
/// inputs, the payload finalizer writes `inputs_hash` and the owner's
/// compact signature, and only then are the inputs ECDSA-signed, since
/// their sighashes cover the finished payload. Stops at the signed
/// transaction; the caller broadcasts or abandons it.
pub(crate) async fn build_sign_update_registrar<B, S>(
    core: &CoreWallet<B>,
    placeholder: ProviderUpdateRegistrarPayload,
    owner: OwnerSecret,
    signer: &S,
) -> Result<SignedCoreTransaction, PlatformWalletError>
where
    B: TransactionBroadcaster + ?Sized,
    S: TransactionSigner + ?Sized + Sync,
{
    let builder = TransactionBuilder::new()
        .set_special_payload(TransactionPayload::ProviderUpdateRegistrarPayloadType(
            placeholder,
        ))
        .set_payload_finalizer(move |unsigned| {
            let Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(placeholder)) =
                &unsigned.special_transaction_payload
            else {
                return Err(BuilderError::InvalidData(
                    "the ProUpRegTx placeholder payload is missing from the assembled \
                     transaction"
                        .into(),
                ));
            };
            let mut finalized = placeholder.clone();
            finalized.inputs_hash = unsigned.hash_inputs();
            finalized.payload_sig = owner_compact_signature(&finalized, &owner)
                .map_err(|e| BuilderError::SigningFailed(e.to_string()))?;
            Ok(TransactionPayload::ProviderUpdateRegistrarPayloadType(
                finalized,
            ))
        });

    core.finalize_transaction(builder, &SEND_FUNDING_SOURCES, 0, signer)
        .await
}

#[cfg(test)]
mod tests {
    use super::super::list::test_support::masternode;
    use super::*;
    use crate::broadcaster::BroadcastError;
    use crate::test_support::funded_wallet_manager;
    use dashcore::hash_types::InputsHash;
    use dashcore::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
    use dashcore::Transaction;
    use key_wallet::account::StandardAccountType;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    /// A fixed valid secp256k1 scalar so the test owner keypair is
    /// deterministic.
    const OWNER_SECRET: [u8; 32] = [7u8; 32];

    fn owner() -> OwnerSecret {
        OwnerSecret {
            secret: Zeroizing::new(OWNER_SECRET),
            compressed: true,
        }
    }

    fn owner_key_hash() -> PubkeyHash {
        let secp = Secp256k1::new();
        let secret = SecretKey::from_byte_array(&OWNER_SECRET).expect("valid scalar");
        let public = secret.public_key(&secp);
        PubkeyHash::from_byte_array(hash160::Hash::hash(&public.serialize()).to_byte_array())
    }

    #[derive(Default)]
    struct RecordingBroadcaster {
        sent: Mutex<Vec<Transaction>>,
    }

    #[async_trait::async_trait]
    impl TransactionBroadcaster for RecordingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.sent
                .lock()
                .expect("broadcaster lock")
                .push(transaction.clone());
            Ok(transaction.txid())
        }
    }

    /// The payload-hash convention, pinned against the real testnet
    /// ProUpRegTx vector embedded in dashcore's own payload tests.
    #[test]
    fn base_payload_hash_matches_the_known_testnet_vector() {
        let operator = <[u8; 48]>::try_from(
            hex::decode(
                "139b654f0b1c031e1cf2b934c2d895178875cfe7c6a4f6758f02bc66eea7fc292d0040701acbe31f5e14a911cb061a2f",
            )
            .expect("hex")
            .as_slice(),
        )
        .expect("48 bytes");
        let voting = <[u8; 20]>::try_from(
            hex::decode("6cc4a7bb877a80c11ae06b988d98305773f93b98")
                .expect("hex")
                .as_slice(),
        )
        .expect("20 bytes");
        let payout_hash = <[u8; 20]>::try_from(
            hex::decode("56bcf3cac49235537d6ce0fb3214d8850a6db777")
                .expect("hex")
                .as_slice(),
        )
        .expect("20 bytes");

        let payload = ProviderUpdateRegistrarPayload {
            version: 1,
            pro_tx_hash: Txid::from_str(
                "3dbb7de94e219e8f7eaea4f3c01cf97d77372e10152734c1959f17302369aa49",
            )
            .expect("txid"),
            provider_mode: 0,
            operator_public_key: BLSPublicKey::from(operator),
            voting_key_hash: PubkeyHash::from_byte_array(voting),
            script_payout: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(payout_hash)),
            inputs_hash: InputsHash::from_str(
                "cf2b940faa8c46c7981f5bd082e5409bf08cffe3bccfa04093eb152f7a857f2d",
            )
            .expect("inputs hash"),
            payload_sig: Vec::new(),
        };
        assert_eq!(
            format!("{:x}", payload.base_payload_hash()),
            "85deffc85d2304f0305356e1dc8d02eecdb3220576abb370bc67be446c854296",
            "payload hash must match the vector dashcore pins"
        );
    }

    /// The owner signature is Core's CHashSigner form: 65 bytes, header
    /// 27 + recovery_id (+4 compressed), recovering to the owner key.
    #[test]
    fn owner_signature_is_compact_recoverable_over_the_payload_hash() {
        let payload = ProviderUpdateRegistrarPayload {
            version: 2,
            pro_tx_hash: Txid::all_zeros(),
            provider_mode: 0,
            operator_public_key: BLSPublicKey::from([4u8; 48]),
            voting_key_hash: PubkeyHash::from_byte_array([3u8; 20]),
            script_payout: ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([5u8; 20])),
            inputs_hash: InputsHash::all_zeros(),
            payload_sig: Vec::new(),
        };
        let signature = owner_compact_signature(&payload, &owner()).expect("signs");
        assert_eq!(signature.len(), 65);
        assert!(
            (31..=34).contains(&signature[0]),
            "compressed-key header byte, got {}",
            signature[0]
        );

        // Recover and compare against the owner key id — the check Core's
        // CheckHashSig performs on validation.
        let secp = Secp256k1::new();
        let recovery_id =
            RecoveryId::try_from(i32::from(signature[0] - 27 - 4)).expect("recovery id");
        let recoverable =
            RecoverableSignature::from_compact(&signature[1..], recovery_id).expect("compact body");
        let digest = Message::from_digest(payload.base_payload_hash().to_byte_array());
        let recovered = secp.recover_ecdsa(&digest, &recoverable).expect("recovers");
        assert_eq!(
            hash160::Hash::hash(&recovered.serialize()).to_byte_array(),
            owner_key_hash().to_byte_array(),
            "the signature must recover to the owner key id"
        );
    }

    #[test]
    fn owner_secret_is_verified_against_the_registered_key_id() {
        verify_owner_secret(&owner_key_hash(), &owner()).expect("matching owner accepted");

        let err = verify_owner_secret(&PubkeyHash::from_byte_array([9u8; 20]), &owner())
            .expect_err("a different owner key id must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let invalid = OwnerSecret {
            secret: Zeroizing::new([0u8; 32]),
            compressed: true,
        };
        let err = verify_owner_secret(&owner_key_hash(), &invalid)
            .expect_err("an invalid scalar must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    #[test]
    fn payout_address_is_required_and_network_checked() {
        let err = resolve_owner_payout_script("", Network::Testnet)
            .expect_err("an empty payout must be refused, never cleared");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let testnet = DashAddress::dummy(Network::Testnet, 3);
        let script = resolve_owner_payout_script(&testnet.to_string(), Network::Testnet)
            .expect("valid address accepted");
        assert_eq!(script, testnet.script_pubkey());

        let mainnet = DashAddress::dummy(Network::Mainnet, 3).to_string();
        let err = resolve_owner_payout_script(&mainnet, Network::Testnet)
            .expect_err("network mismatch must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    /// Operator keys are consensus-unique across the list — a candidate in
    /// use (under either serialization) must be refused before signing.
    #[test]
    fn used_operator_keys_are_refused() {
        let mut entry = masternode(0x11);
        entry.operator_public_key = [0xAA; 48];
        let summaries = vec![entry];

        let err = ensure_operator_key_unused(&summaries, &[0xAA; 48], None)
            .expect_err("modern-serialization clash refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let err = ensure_operator_key_unused(&summaries, &[0xBB; 48], Some(&[0xAA; 48]))
            .expect_err("legacy-serialization clash refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        ensure_operator_key_unused(&summaries, &[0xBB; 48], Some(&[0xCC; 48]))
            .expect("an unused key passes");
    }

    /// The full prepare wiring below the network fetches — entry lookup,
    /// owner verification, fresh-key derivation, uniqueness and reuse
    /// preflights, payload assembly — driven through the same seam the
    /// public orchestrator uses, then funded, signed and broadcast. The
    /// public function adds only the SPV summaries read, the txid-bound
    /// ProRegTx fetch and the collateral resolution around this.
    /// A registration payload for masternode `0x11` with an internal
    /// collateral and the test owner key — the ProRegTx side of the
    /// assembly fixtures.
    fn test_registration() -> ProviderRegistrationPayload {
        use dashcore::blockdata::transaction::special_transaction::provider_registration::ProviderMasternodeType;
        use dashcore::OutPoint;

        ProviderRegistrationPayload {
            version: ProviderRegistrationPayload::CURRENT_VERSION,
            masternode_type: ProviderMasternodeType::Regular,
            masternode_mode: 0,
            collateral_outpoint: OutPoint {
                txid: Txid::all_zeros(),
                vout: 0,
            },
            service_address: "10.0.0.17:9999".parse().expect("socket address"),
            owner_key_hash: owner_key_hash(),
            operator_public_key: BLSPublicKey::from([0x11; 48]),
            voting_key_hash: PubkeyHash::from_byte_array([0x11; 20]),
            operator_reward: 0,
            script_payout: ScriptBuf::new(),
            inputs_hash: InputsHash::all_zeros(),
            signature: vec![],
            platform_node_id: None,
            platform_p2p_port: None,
            platform_http_port: None,
        }
    }

    #[test]
    fn assembles_and_signs_through_the_prepare_wiring() {
        let wallet = crate::test_support::sync_test_platform_wallet();

        let derived_operator: [u8; 48] = wallet
            .derive_provider_key_at_index(ProviderKeyKind::Operator, 0, None, false)
            .expect("operator key 0")
            .public_key_bytes
            .as_slice()
            .try_into()
            .expect("48 bytes");
        let derived_voting = wallet
            .derive_provider_key_at_index(ProviderKeyKind::Voting, 0, None, false)
            .expect("voting key 0");
        let expected_voting_hash =
            hash160::Hash::hash(&derived_voting.public_key_bytes).to_byte_array();

        let summaries = vec![masternode(0x11), masternode(0x22)];
        // The mock-SDK fixture reports mainnet, so the payout address must
        // be a mainnet one — `wallet.network()` gates it.
        let payout_address = DashAddress::dummy(Network::Mainnet, 3);
        let params = MasternodeUpdateRegistrarParams {
            pro_tx_hash: [0x11; 32],
            new_operator_key_index: Some(0),
            new_voting_key_index: Some(0),
            payout_address: payout_address.to_string(),
        };
        let registration = test_registration();
        let collateral = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0xAB; 20]));

        let placeholder = assemble_update_registrar_placeholder(
            &wallet,
            &summaries,
            &registration,
            &collateral,
            &params,
            &owner(),
        )
        .expect("the wiring assembles the placeholder");

        assert_eq!(placeholder.pro_tx_hash, Txid::from_byte_array([0x11; 32]));
        assert_eq!(placeholder.provider_mode, 0);
        assert_eq!(
            placeholder.operator_public_key,
            BLSPublicKey::from(derived_operator),
            "the chosen wallet operator key lands in the payload"
        );
        assert_eq!(
            placeholder.voting_key_hash,
            PubkeyHash::from_byte_array(expected_voting_hash),
            "the chosen wallet voting key's hash160 lands in the payload"
        );
        assert_eq!(placeholder.script_payout, payout_address.script_pubkey());
        assert_eq!(placeholder.inputs_hash, InputsHash::all_zeros());
        assert!(placeholder.payload_sig.is_empty());

        // The collateral preflight is wired through: a collateral paid to
        // the chosen voting key's address refuses the whole assembly.
        let voting_collateral =
            ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(expected_voting_hash));
        let err = assemble_update_registrar_placeholder(
            &wallet,
            &summaries,
            &registration,
            &voting_collateral,
            &params,
            &owner(),
        )
        .expect_err("collateral reuse by the chosen voting key is refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // Fund, sign and broadcast the assembled placeholder end-to-end.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        runtime.block_on(async move {
            let (wallet_manager, wallet_id, generation, signer) =
                funded_wallet_manager(StandardAccountType::BIP44Account).await;
            let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
            let broadcaster = Arc::new(RecordingBroadcaster::default());
            let core = CoreWallet::new(
                sdk,
                wallet_manager,
                wallet_id,
                broadcaster.clone(),
                generation,
            );

            let prepared = build_sign_update_registrar(&core, placeholder, owner(), &signer)
                .await
                .expect("the assembled placeholder funds and signs");
            let txid = core
                .broadcast_finalized_transaction(&prepared)
                .await
                .expect("broadcasts");

            let sent = broadcaster.sent.lock().expect("broadcaster lock");
            assert_eq!(sent.len(), 1);
            let tx = &sent[0];
            assert_eq!(tx.txid(), txid);
            let Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(payload)) =
                &tx.special_transaction_payload
            else {
                panic!("the broadcast transaction must carry the ProUpRegTx payload");
            };
            assert_eq!(payload.pro_tx_hash, Txid::from_byte_array([0x11; 32]));
            assert_eq!(
                payload.operator_public_key,
                BLSPublicKey::from(derived_operator)
            );
            assert_eq!(
                payload.voting_key_hash,
                PubkeyHash::from_byte_array(expected_voting_hash)
            );
            assert_eq!(payload.inputs_hash, tx.hash_inputs());

            // The owner signature recovers to the owner key id over the
            // finished payload hash — Core's CheckHashSig check.
            let secp = Secp256k1::new();
            let recovery_id =
                RecoveryId::try_from(i32::from(payload.payload_sig[0] - 27 - 4)).expect("recid");
            let recoverable =
                RecoverableSignature::from_compact(&payload.payload_sig[1..], recovery_id)
                    .expect("compact body");
            let digest = Message::from_digest(payload.base_payload_hash().to_byte_array());
            let recovered = secp.recover_ecdsa(&digest, &recoverable).expect("recovers");
            assert_eq!(
                hash160::Hash::hash(&recovered.serialize()).to_byte_array(),
                owner_key_hash().to_byte_array()
            );
        });
    }

    /// Regression for the review's async-context panic: key derivation
    /// takes the wallet-manager lock via tokio's `blocking_read`, which
    /// panics on a runtime worker thread — where the FFI's
    /// `block_on_worker` polls the orchestrator. Driving the same
    /// assembly seam the orchestrator awaits from inside a multi-thread
    /// runtime must yield the payload, not abort.
    #[test]
    fn assembly_derives_keys_from_an_async_context_without_panicking() {
        // The fixture hands back an `Arc`; the orchestrator moves an owned
        // clone of the wallet handle into the assembly task.
        let wallet = PlatformWallet::clone(&crate::test_support::sync_test_platform_wallet());
        let summaries = vec![masternode(0x11)];
        let params = MasternodeUpdateRegistrarParams {
            pro_tx_hash: [0x11; 32],
            new_operator_key_index: Some(0),
            new_voting_key_index: Some(0),
            payout_address: DashAddress::dummy(Network::Mainnet, 3).to_string(),
        };
        let collateral = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0xAB; 20]));

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let (placeholder, _owner) = runtime
            .block_on(async move {
                // A spawned task (not just `block_on`) — the assembly must
                // survive polling on a worker in an async execution
                // context, exactly like the FFI drives it.
                tokio::spawn(assemble_update_registrar_placeholder_from_async(
                    wallet,
                    summaries,
                    test_registration(),
                    collateral,
                    params,
                    owner(),
                ))
                .await
                .expect("assembly task must not panic in an async context")
            })
            .expect("assembly succeeds");
        assert_eq!(placeholder.pro_tx_hash, Txid::from_byte_array([0x11; 32]));
    }

    #[tokio::test]
    async fn builds_signs_and_broadcasts_a_pro_up_reg_tx() {
        let (wallet_manager, wallet_id, generation, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let broadcaster = Arc::new(RecordingBroadcaster::default());
        let core = CoreWallet::new(
            sdk,
            wallet_manager,
            wallet_id,
            broadcaster.clone(),
            generation,
        );

        let placeholder = ProviderUpdateRegistrarPayload::new(
            Txid::from_byte_array([0x22; 32]),
            0,
            BLSPublicKey::from([4u8; 48]),
            PubkeyHash::from_byte_array([3u8; 20]),
            ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([5u8; 20])),
            InputsHash::all_zeros(),
            Vec::new(),
        );

        let prepared = build_sign_update_registrar(&core, placeholder, owner(), &signer)
            .await
            .expect("registrar update builds and signs");
        assert!(
            broadcaster
                .sent
                .lock()
                .expect("broadcaster lock")
                .is_empty(),
            "preparing must not broadcast"
        );

        let txid = core
            .broadcast_finalized_transaction(&prepared)
            .await
            .expect("prepared transaction broadcasts");

        let sent = broadcaster.sent.lock().expect("broadcaster lock");
        assert_eq!(sent.len(), 1);
        let tx = &sent[0];
        assert_eq!(tx.txid(), txid);
        assert_eq!(tx.version, 3);
        assert!(tx.input.iter().all(|input| !input.script_sig.is_empty()));

        let Some(TransactionPayload::ProviderUpdateRegistrarPayloadType(payload)) =
            &tx.special_transaction_payload
        else {
            panic!("the broadcast transaction must carry the ProUpRegTx payload");
        };
        assert_eq!(payload.inputs_hash, tx.hash_inputs());
        assert_eq!(
            payload.version,
            ProviderUpdateRegistrarPayload::CURRENT_VERSION
        );
        assert_eq!(payload.payload_sig.len(), 65);

        // The owner signature recovers over the finished payload hash.
        let secp = Secp256k1::new();
        let recovery_id =
            RecoveryId::try_from(i32::from(payload.payload_sig[0] - 27 - 4)).expect("recid");
        let recoverable =
            RecoverableSignature::from_compact(&payload.payload_sig[1..], recovery_id)
                .expect("compact body");
        let digest = Message::from_digest(payload.base_payload_hash().to_byte_array());
        let recovered = secp.recover_ecdsa(&digest, &recoverable).expect("recovers");
        assert_eq!(
            hash160::Hash::hash(&recovered.serialize()).to_byte_array(),
            owner_key_hash().to_byte_array()
        );
    }
}

#[cfg(test)]
mod review_tests {
    use super::super::locator::bls_public_keys;
    use super::*;
    use dashcore::secp256k1::PublicKey as SecpPublicKey;

    /// Consensus accepts only P2PKH / P2SH payouts (`bad-protx-payee`): a
    /// witness-program address must be refused before funding.
    #[test]
    fn witness_payout_addresses_are_refused() {
        let secp = Secp256k1::new();
        let secret = SecretKey::from_byte_array(&[7u8; 32]).expect("valid scalar");
        let public = dashcore::PublicKey::new(SecpPublicKey::from_secret_key(&secp, &secret));
        let witness = DashAddress::p2wpkh(&public, Network::Testnet).expect("p2wpkh address");

        let err = resolve_owner_payout_script(&witness.to_string(), Network::Testnet)
            .expect_err("a witness payout must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // The operator-payout resolver in the update-service path applies
        // the same restriction to a non-empty payout.
        let err = super::super::update_service::resolve_operator_payout_script(
            500,
            Some(&witness.to_string()),
            Network::Testnet,
        )
        .expect_err("a witness operator payout must be refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // P2SH remains accepted.
        let p2sh = DashAddress::p2sh(
            &ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([9; 20])),
            Network::Testnet,
        )
        .expect("p2sh address");
        resolve_owner_payout_script(&p2sh.to_string(), Network::Testnet)
            .expect("a P2SH payout is accepted");
    }

    /// Consensus rejects a payout paid to the owner or final voting key
    /// (`bad-protx-payee-reuse`).
    #[test]
    fn payouts_reusing_owner_or_voting_keys_are_refused() {
        let owner = PubkeyHash::from_byte_array([0x11; 20]);
        let voting = [0x22u8; 20];

        let owner_payout = ScriptBuf::new_p2pkh(&owner);
        let err = ensure_payout_not_reusing_keys(&owner_payout, &owner, &voting)
            .expect_err("owner-address payout refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let voting_payout = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(voting));
        let err = ensure_payout_not_reusing_keys(&voting_payout, &owner, &voting)
            .expect_err("voting-address payout refused — including a newly selected candidate");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let other = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x33; 20]));
        ensure_payout_not_reusing_keys(&other, &owner, &voting)
            .expect("an unrelated payout passes");

        // P2SH carries a script hash, not a key id — never a collision.
        let p2sh = ScriptBuf::new_p2sh(&dashcore::ScriptHash::from_byte_array([0x11; 20]));
        ensure_payout_not_reusing_keys(&p2sh, &owner, &voting)
            .expect("a P2SH payout cannot reuse a key id");
    }

    /// Consensus loads the collateral UTXO and rejects reusing its P2PKH
    /// destination as the owner or final voting key
    /// (`bad-protx-collateral-reuse`) — the DML never shows a collateral
    /// address, so candidate discovery alone cannot catch this. For a v3
    /// entry it also rejects a payout equal to the collateral script
    /// (`bad-protx-payee-reuse`).
    #[test]
    fn collateral_reuse_is_refused() {
        let owner = PubkeyHash::from_byte_array([0x11; 20]);
        let voting = [0x22u8; 20];
        let payout = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x33; 20]));

        let voting_collateral = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array(voting));
        let err = ensure_collateral_not_reused(&voting_collateral, &owner, &voting, &payout, false)
            .expect_err("a collateral at the final voting key's address is refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let owner_collateral = ScriptBuf::new_p2pkh(&owner);
        let err = ensure_collateral_not_reused(&owner_collateral, &owner, &voting, &payout, false)
            .expect_err("a collateral at the owner key's address is refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        let unrelated = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x44; 20]));
        ensure_collateral_not_reused(&unrelated, &owner, &voting, &payout, true)
            .expect("an unrelated collateral passes both gates");

        // The two collisions are distinct conditions: re-choosing the
        // voting key clears one, while the immutable owner key's collision
        // has no remedy — the messages must say so.
        let owner_message = ensure_collateral_not_reused(
            &ScriptBuf::new_p2pkh(&owner),
            &owner,
            &voting,
            &payout,
            false,
        )
        .expect_err("owner collision")
        .to_string();
        assert!(
            owner_message.contains("cannot be updated"),
            "the owner-key collision must not suggest re-choosing the voting key: \
             {owner_message}"
        );
        let voting_message =
            ensure_collateral_not_reused(&voting_collateral, &owner, &voting, &payout, false)
                .expect_err("voting collision")
                .to_string();
        assert!(
            voting_message.contains("pick a different voting key"),
            "the voting-key collision is remedied by re-choosing: {voting_message}"
        );

        // Payout == collateral: Core applies this arm only from v3
        // (ExtAddr) entries — a P2SH collateral carries no key id, so only
        // the gated script-equality check can fire.
        let p2sh = ScriptBuf::new_p2sh(&dashcore::ScriptHash::from_byte_array([0x55; 20]));
        ensure_collateral_not_reused(&p2sh, &owner, &voting, &p2sh, false)
            .expect("payout-collateral reuse is not checked below v3");
        let err = ensure_collateral_not_reused(&p2sh, &owner, &voting, &p2sh, true)
            .expect_err("a v3 entry's payout must not be the collateral script");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }

    /// Core's `ExtractDestination` also converts a valid P2PK collateral
    /// script to a `PKHash` — hashing the public key in its original
    /// (compressed or uncompressed) serialization — and `CheckProUpRegTx`
    /// compares that hash against the owner and final voting keys. A P2PK
    /// collateral paid to either key must be refused, in both
    /// serializations.
    #[test]
    fn p2pk_collateral_reuse_is_refused() {
        let secp = Secp256k1::new();
        let secret = SecretKey::from_byte_array(&[7u8; 32]).expect("valid scalar");
        let inner = SecpPublicKey::from_secret_key(&secp, &secret);
        let compressed = dashcore::PublicKey::new(inner);
        let uncompressed = dashcore::PublicKey::new_uncompressed(inner);
        let payout = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x33; 20]));
        let other_owner = PubkeyHash::from_byte_array([0x11; 20]);
        let other_voting = [0x22u8; 20];

        // Compressed P2PK collateral at the chosen voting key.
        let compressed_collateral = ScriptBuf::new_p2pk(&compressed);
        let voting = compressed.pubkey_hash().to_byte_array();
        let err = ensure_collateral_not_reused(
            &compressed_collateral,
            &other_owner,
            &voting,
            &payout,
            false,
        )
        .expect_err("a compressed-P2PK collateral at the voting key is refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // Uncompressed P2PK collateral at the owner key — hashed over the
        // key's own 65-byte serialization, which differs from the
        // compressed hash.
        let uncompressed_collateral = ScriptBuf::new_p2pk(&uncompressed);
        let owner = uncompressed.pubkey_hash();
        assert_ne!(
            owner.to_byte_array(),
            voting,
            "the two serializations must hash differently for this test to mean anything"
        );
        let err = ensure_collateral_not_reused(
            &uncompressed_collateral,
            &owner,
            &other_voting,
            &payout,
            false,
        )
        .expect_err("an uncompressed-P2PK collateral at the owner key is refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));

        // A P2PK collateral for an unrelated key passes.
        ensure_collateral_not_reused(
            &compressed_collateral,
            &other_owner,
            &other_voting,
            &payout,
            false,
        )
        .expect("an unrelated P2PK collateral passes");
    }

    /// The collateral script comes from an output index inside a fetched
    /// transaction — bounds-checked, never a panic on hostile data.
    #[test]
    fn collateral_output_script_is_bounds_checked() {
        let script = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([0x66; 20]));
        let transaction = dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![dashcore::TxOut {
                value: 1_000_000_000_000,
                script_pubkey: script.clone(),
            }],
            special_transaction_payload: None,
        };

        assert_eq!(
            collateral_output_script(&transaction, 0).expect("in-range output"),
            script
        );
        let err = collateral_output_script(&transaction, 1)
            .expect_err("an out-of-range outpoint index is refused");
        assert!(matches!(err, PlatformWalletError::InvalidIdentityData(_)));
    }

    /// A kept operator key from a version-1 (legacy-serialized) entry is
    /// reserialized to the basic scheme a v2 payload requires; a v2 entry's
    /// key passes through; garbage is refused. The entry version — not the
    /// bytes — picks the scheme: legacy bytes also "parse" under basic (the
    /// serializations differ only in flag bits), so sniffing is unsound.
    #[test]
    fn kept_operator_keys_are_normalized_to_basic() {
        let (basic, legacy) = bls_public_keys(&[7u8; 32]).expect("valid scalar");

        assert_eq!(
            normalize_operator_key_to_basic(&basic, false).expect("basic passes through"),
            basic
        );
        assert_eq!(
            normalize_operator_key_to_basic(&legacy, true).expect("legacy is reserialized"),
            basic,
            "the same G1 point re-emerges in basic serialization"
        );
        let err = normalize_operator_key_to_basic(&[0xFF; 48], false)
            .expect_err("invalid basic bytes are refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
        let err = normalize_operator_key_to_basic(&[0xFF; 48], true)
            .expect_err("invalid legacy bytes are refused");
        assert!(matches!(err, PlatformWalletError::InvalidParameter(_)));
    }
}
