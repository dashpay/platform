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
use super::update_service::{
    display_hex, fetch_registration_payload, require_standard_payout_script,
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
    if params.new_operator_key_index.is_none() && params.new_voting_key_index.is_none() {
        return Err(PlatformWalletError::InvalidParameter(
            "nothing to rotate: neither a new operator key nor a new voting key was chosen"
                .to_string(),
        ));
    }

    let summaries = spv
        .masternode_list_summaries()
        .await
        .ok_or(PlatformWalletError::MasternodeListUnavailable)?;
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

    // The owner key is immutable — set at registration, never rotatable —
    // so the ProRegTx's keyIDOwner is the reliable authority to verify the
    // supplied secret against (the masternode list does not carry it).
    let registration = fetch_registration_payload(wallet, &params.pro_tx_hash).await?;
    verify_owner_secret(&registration.owner_key_hash, &owner)?;

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
            ensure_operator_key_unused(&summaries, &bytes, legacy.as_ref())?;
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

    let placeholder = ProviderUpdateRegistrarPayload::new(
        Txid::from_byte_array(params.pro_tx_hash),
        0, // provider_mode — 0 is the only defined mode
        BLSPublicKey::from(operator_public_key),
        PubkeyHash::from_byte_array(voting_key_hash),
        script_payout,
        dashcore::hash_types::InputsHash::all_zeros(),
        Vec::new(),
    );

    build_sign_update_registrar(wallet.core(), placeholder, owner, signer).await
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
