//! Authorization evidence for reserving a wallet coin before Core finality.

use dashcore::crypto::ecdsa::{EcdsaPublicKey, EcdsaSignature};
use dashcore::ecdsa::Signature;
use dashcore::hashes::{hash160, Hash};
use dashcore::script::Instruction;
use dashcore::sighash::{EcdsaSighashType, SighashCache};
use dashcore::{Transaction, TxOut};
use thiserror::Error;

/// A pending spend whose authorization cannot be decided by the wallet signer contract.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpendAuthorizationError {
    /// The funding script requires a different authorization verifier.
    #[error("pending spend uses an unsupported funding script")]
    UnsupportedScript,
    /// The signature does not commit to the complete transaction.
    #[error("pending spend uses unsupported signature hash type {0:#x}")]
    UnsupportedSighash(u8),
}

/// Checks P2PKH/SIGHASH_ALL authorization to reserve one input's funding coin.
///
/// The caller must obtain `prevout` from the funding transaction identified by
/// this input, and bind its outpoint to the wallet coin being reserved. A valid
/// signature proves authorization only: other inputs, amounts and consensus
/// rules are not validated, and this result supplies no finality evidence.
/// Unsupported forms return an error so an existing pending hold is not
/// silently released. Malformed or invalid supported authorizations return false.
pub fn verify_input_authorization(
    transaction: &Transaction,
    input_index: usize,
    prevout: &TxOut,
) -> Result<bool, SpendAuthorizationError> {
    let Some(input) = transaction.input.get(input_index) else {
        return Ok(false);
    };
    if transaction
        .input
        .iter()
        .filter(|candidate| candidate.previous_output == input.previous_output)
        .count()
        != 1
    {
        return Ok(false);
    }
    if !prevout.script_pubkey.is_p2pkh() {
        return Err(SpendAuthorizationError::UnsupportedScript);
    }
    let mut instructions = input.script_sig.instructions();
    let (Some(Ok(Instruction::PushBytes(signature))), Some(Ok(Instruction::PushBytes(public_key)))) =
        (instructions.next(), instructions.next())
    else {
        return Ok(false);
    };
    if instructions.next().is_some() {
        return Ok(false);
    }
    let public_key_bytes = public_key.as_bytes();
    if hash160::Hash::hash(public_key_bytes)
        .as_byte_array()
        .as_slice()
        != &prevout.script_pubkey.as_bytes()[3..23]
    {
        return Ok(false);
    }
    let Ok(public_key) = EcdsaPublicKey::from_bytes(public_key_bytes) else {
        return Ok(false);
    };
    let Some((&mode, _)) = signature.as_bytes().split_last() else {
        return Ok(false);
    };
    let Ok(mut signature) = Signature::from_slice(signature.as_bytes()) else {
        return Ok(false);
    };
    // Low-S is a relay policy, not evidence that the owner did not authorize
    // a spend. The crypto verifier requires normalized S.
    signature.signature.normalize_s();
    let signature = EcdsaSignature::from(signature.signature);
    let Ok(hash) = SighashCache::new(transaction).legacy_signature_hash(
        input_index,
        &prevout.script_pubkey,
        u32::from(mode),
    ) else {
        return Ok(false);
    };
    if public_key.verify(&hash.to_byte_array(), signature).is_err() {
        return Ok(false);
    }
    // Other modes can authorize only part of a transaction; SINGLE can even
    // produce a constant digest. They cannot authorize this durable reservation.
    if u32::from(mode) != EcdsaSighashType::All.to_u32() {
        return Err(SpendAuthorizationError::UnsupportedSighash(mode));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::blockdata::transaction::special_transaction::{
        asset_lock::AssetLockPayload, TransactionPayload,
    };
    use dashcore::crypto::ecdsa::EcdsaSecretKey;
    use dashcore::script::{Builder, PushBytesBuf};
    use dashcore::{OutPoint, ScriptBuf, TxIn, Txid, Witness};

    fn signed(uncompressed: bool) -> (Transaction, TxOut) {
        let key = EcdsaSecretKey::from_bytes(&[7; 32]).unwrap();
        let public_key = if uncompressed {
            key.public_key().to_uncompressed().to_vec()
        } else {
            key.public_key().to_compressed().to_vec()
        };
        let prevout = TxOut {
            value: 100_000,
            script_pubkey: ScriptBuf::new_p2pkh(&hash160::Hash::hash(&public_key).into()),
        };
        let mut tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([3; 32]),
                    vout: 2,
                },
                ..Default::default()
            }],
            output: vec![TxOut {
                value: 90_000,
                script_pubkey: prevout.script_pubkey.clone(),
            }],
            special_transaction_payload: None,
        };
        sign(&mut tx, &prevout, &key, &public_key, 1);
        (tx, prevout)
    }

    fn sign(
        tx: &mut Transaction,
        prevout: &TxOut,
        key: &EcdsaSecretKey,
        public_key: &[u8],
        mode: u8,
    ) {
        let hash = SighashCache::new(&*tx)
            .legacy_signature_hash(0, &prevout.script_pubkey, u32::from(mode))
            .unwrap();
        let mut signature = key.sign(&hash.to_byte_array()).with_der(|der| der.to_vec());
        signature.push(mode);
        tx.input[0].script_sig = Builder::new()
            .push_slice(PushBytesBuf::try_from(signature).unwrap())
            .push_slice(PushBytesBuf::try_from(public_key.to_vec()).unwrap())
            .into_script();
    }

    #[test]
    fn should_accept_compressed_and_uncompressed_owner_signatures() {
        for uncompressed in [false, true] {
            let (tx, prevout) = signed(uncompressed);
            assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(true));
        }
    }

    #[test]
    fn should_reject_unsigned_tampered_and_wrong_funding_claims() {
        let (tx, prevout) = signed(false);
        let mut unsigned = tx.clone();
        unsigned.input[0].script_sig = ScriptBuf::new();
        assert_eq!(
            verify_input_authorization(&unsigned, 0, &prevout),
            Ok(false)
        );
        let mut changed = tx.clone();
        changed.output[0].value -= 1;
        assert_eq!(verify_input_authorization(&changed, 0, &prevout), Ok(false));
        changed = tx.clone();
        changed.input[0].previous_output.vout += 1;
        assert_eq!(verify_input_authorization(&changed, 0, &prevout), Ok(false));
        let (_, different_encoding) = signed(true);
        assert_eq!(
            verify_input_authorization(&tx, 0, &different_encoding),
            Ok(false)
        );
        assert_eq!(verify_input_authorization(&tx, 1, &prevout), Ok(false));
        changed = tx.clone();
        changed.input.push(tx.input[0].clone());
        assert_eq!(verify_input_authorization(&changed, 0, &prevout), Ok(false));
    }

    #[test]
    fn should_reject_extra_script_instructions_and_malformed_signature() {
        let (mut tx, prevout) = signed(false);
        let mut bytes = tx.input[0].script_sig.as_bytes().to_vec();
        bytes.push(0);
        tx.input[0].script_sig = ScriptBuf::from_bytes(bytes);
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(false));
        tx.input[0].script_sig = ScriptBuf::from_bytes(vec![1, 1, 1, 2]);
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(false));
    }

    #[test]
    fn should_bind_asset_lock_payload_to_authorization() {
        let (mut tx, prevout) = signed(false);
        let key = EcdsaSecretKey::from_bytes(&[7; 32]).unwrap();
        tx.version = 3;
        tx.special_transaction_payload = Some(TransactionPayload::AssetLockPayloadType(
            AssetLockPayload::new(vec![prevout.clone()]),
        ));
        sign(
            &mut tx,
            &prevout,
            &key,
            &key.public_key().to_compressed(),
            1,
        );
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(true));
        if let Some(TransactionPayload::AssetLockPayloadType(payload)) =
            &mut tx.special_transaction_payload
        {
            payload.credit_outputs[0].value -= 1;
        }
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(false));
    }

    #[test]
    fn should_not_turn_an_unauthenticated_hash_flag_into_an_unsupported_error() {
        let (mut tx, prevout) = signed(false);
        let mut bytes = tx.input[0].script_sig.as_bytes().to_vec();
        let signature_len = usize::from(bytes[0]);
        bytes[signature_len] = 0x82;
        tx.input[0].script_sig = ScriptBuf::from_bytes(bytes);
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(false));
    }

    #[test]
    fn should_preserve_authorization_with_uncommitted_witness() {
        let (mut tx, prevout) = signed(false);
        let original_txid = tx.txid();
        tx.input[0].witness = Witness::from_slice(&[b"peer-attached witness"]);
        assert_eq!(tx.txid(), original_txid);
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(true));
    }

    #[test]
    fn should_accept_high_s_owner_authorization() {
        let (mut tx, prevout) = signed(false);
        let key = EcdsaSecretKey::from_bytes(&[7; 32]).unwrap();
        let hash = SighashCache::new(&tx)
            .legacy_signature_hash(0, &prevout.script_pubkey, 1)
            .unwrap();
        let low = key.sign(&hash.to_byte_array());
        let mut compact = low.to_bytes();
        // secp256k1 group order: (r, n-s) is the high-S form of (r, s).
        let order = hex::decode("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
            .unwrap();
        let mut borrow = false;
        for i in (0..32).rev() {
            let (difference, first_borrow) = order[i].overflowing_sub(compact[32 + i]);
            let (difference, second_borrow) = difference.overflowing_sub(u8::from(borrow));
            compact[32 + i] = difference;
            borrow = first_borrow || second_borrow;
        }
        let high = EcdsaSignature::from_bytes(&compact).unwrap();
        assert!(key
            .public_key()
            .verify(&hash.to_byte_array(), high)
            .is_err());
        let mut signature = high.with_der(|der| der.to_vec());
        signature.push(1);
        tx.input[0].script_sig = Builder::new()
            .push_slice(PushBytesBuf::try_from(signature).unwrap())
            .push_slice(key.public_key().to_compressed())
            .into_script();
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(true));
        tx.output[0].value -= 1;
        assert_eq!(verify_input_authorization(&tx, 0, &prevout), Ok(false));
    }

    #[test]
    fn should_preserve_unsupported_authorization_as_an_error() {
        let (mut tx, prevout) = signed(false);
        let key = EcdsaSecretKey::from_bytes(&[7; 32]).unwrap();
        for mode in [2, 3, 0x81, 0x82, 0x83] {
            sign(
                &mut tx,
                &prevout,
                &key,
                &key.public_key().to_compressed(),
                mode,
            );
            assert_eq!(
                verify_input_authorization(&tx, 0, &prevout),
                Err(SpendAuthorizationError::UnsupportedSighash(mode))
            );
        }
        let unknown = TxOut {
            value: prevout.value,
            script_pubkey: ScriptBuf::new(),
        };
        assert_eq!(
            verify_input_authorization(&tx, 0, &unknown),
            Err(SpendAuthorizationError::UnsupportedScript)
        );
    }
}
