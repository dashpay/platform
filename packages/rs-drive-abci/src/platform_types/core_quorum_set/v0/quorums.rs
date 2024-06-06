use dashcore_rpc::json::QuorumType;
use derive_more::{Deref, DerefMut, From};
use dpp::bls_signatures::PrivateKey;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::{QuorumHash, QuorumSigningRequestId, Transaction, TxIn, Txid, VarInt};
use dpp::{bls_signatures, ProtocolError};
use std::collections::BTreeMap;
use std::convert::TryInto;

pub use dpp::bls_signatures::PublicKey as ThresholdBlsPublicKey;

/// Reversed quorum hash bytes. Used for signature verification as part of the signature payload
pub type ReversedQuorumHashBytes = Vec<u8>;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::core_quorum_set::QuorumConfig;
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};

/// Quorum per hash
#[derive(Debug, Clone, Deref, DerefMut, From)]
pub struct Quorums<Q>(BTreeMap<QuorumHash, Q>);

impl<Q> Default for Quorums<Q> {
    fn default() -> Self {
        Quorums::<Q>(BTreeMap::new())
    }
}

impl<Q: Quorum> FromIterator<(QuorumHash, Q)> for Quorums<Q> {
    fn from_iter<T: IntoIterator<Item = (QuorumHash, Q)>>(iter: T) -> Self {
        Quorums::<Q>(BTreeMap::from_iter(iter))
    }
}

impl<Q> IntoIterator for Quorums<Q> {
    type Item = (QuorumHash, Q);
    type IntoIter = std::collections::btree_map::IntoIter<QuorumHash, Q>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<Q: Quorum> Quorums<Q> {
    /// Choose pseudorandom DIP8 or DIP24 quorum based on quorum config
    /// and request_id
    pub fn choose_quorum(
        &self,
        quorum_config: &QuorumConfig,
        request_id: &[u8; 32],
    ) -> Option<(QuorumHash, &Q)> {
        if quorum_config.rotation {
            self.choose_rotating_quorum(quorum_config, request_id)
        } else {
            self.choose_classic_quorum(quorum_config, request_id)
        }
    }

    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    fn choose_classic_quorum(
        &self,
        quorum_config: &QuorumConfig,
        request_id: &[u8; 32],
    ) -> Option<(QuorumHash, &Q)> {
        // Scoring system logic
        let mut scores: Vec<(&QuorumHash, &Q, [u8; 32])> = Vec::new();

        for (quorum_hash, quorum) in self.0.iter() {
            let mut quorum_hash_bytes = quorum_hash.to_byte_array().to_vec();

            // Only the quorum hash needs reversal.
            quorum_hash_bytes.reverse();

            let mut hasher = sha256d::Hash::engine();

            // Serialize and hash the LLMQ type
            hasher.input(&[quorum_config.quorum_type as u8]);

            // Serialize and add the quorum hash
            hasher.input(quorum_hash_bytes.as_slice());

            // Serialize and add the selection hash from the chain lock
            hasher.input(request_id.as_slice());

            // Finalize the hash
            let hash_result = sha256d::Hash::from_engine(hasher);
            scores.push((quorum_hash, quorum, hash_result.into()));
        }

        if scores.is_empty() {
            return None;
        }

        scores.sort_by_key(|k| k.2);

        let (quorum_hash, quorum, _) = scores.remove(0);

        Some((*quorum_hash, quorum))
    }

    /// Based on DIP24 deterministically chooses a pseudorandom quorum from the list of quorums
    fn choose_rotating_quorum(
        &self,
        quorum_config: &QuorumConfig,
        request_id: &[u8; 32],
    ) -> Option<(QuorumHash, &Q)> {
        let n = quorum_config.active_signers as u64;

        let b = u64::from_le_bytes(request_id[24..32].try_into().unwrap());

        // Take last n bits of b
        let mask = (1u64 << n) - 1;
        let signer = mask & (b >> (64 - n));

        self.0
            .iter()
            .find(|(hash, quorum)| quorum.index() == Some(signer as u32))
            .map(|(hash, quorum)| (*hash, quorum))
    }
}

/// Quorum trait for Quorums collection
pub trait Quorum {
    /// Index is present only for rotated quorums (DIP24)
    fn index(&self) -> Option<u32>;
}

/// Quorum for signature verification
#[derive(Debug, Clone)]
pub struct VerificationQuorum {
    /// Index is present only for rotated quorums (DIP24)
    pub index: Option<u32>,

    /// Quorum threshold public key is used to verify
    /// signatures produced by corresponding quorum
    pub public_key: ThresholdBlsPublicKey,
}

impl Quorum for VerificationQuorum {
    fn index(&self) -> Option<u32> {
        self.index
    }
}

/// Quorum for signature verification
#[derive(Debug, Clone)]
pub struct SigningQuorum {
    /// Index is present only for rotated quorums (DIP24)
    pub index: Option<u32>,

    /// Quorum private key for signing
    pub private_key: [u8; 32],
}

impl Quorum for SigningQuorum {
    fn index(&self) -> Option<u32> {
        self.index
    }
}

impl SigningQuorum {
    /// Signs a transition for instant lock
    pub fn sign_for_instant_lock(
        &self,
        quorum_config: &QuorumConfig,
        quorum_hash: &QuorumHash,
        request_id: &[u8; 32],
        transaction_id: &Txid,
    ) -> Result<BLSSignature, Error> {
        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), txId).
        // llmqType and quorumHash must be taken from the quorum selected in 1.
        let mut engine = sha256d::Hash::engine();

        let mut reversed_quorum_hash = quorum_hash.to_byte_array().to_vec();
        reversed_quorum_hash.reverse();

        engine.input(&[quorum_config.quorum_type as u8]);
        engine.input(reversed_quorum_hash.as_slice());
        engine.input(request_id);
        engine.input(transaction_id.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        let private_key =
            PrivateKey::from_bytes(&self.private_key, false).map_err(Error::BLSError)?;

        let g2element = private_key.sign(message_digest.as_ref());
        let g2element_bytes = *g2element.to_bytes();

        Ok(BLSSignature::from(g2element_bytes))
    }
}

//
// const IS_LOCK_REQUEST_ID_PREFIX: &str = "islock";
//
// // TODO: Must be in dashcore lib
//
// pub struct InstantLockQuorumSigner {
//     quorum_hash: QuorumHash,
//     quorum_type: QuorumType,
//     quorum_private_key: PrivateKey,
// }
//
// impl InstantLockQuorumSigner {
//     pub fn new(
//         quorum_hash: QuorumHash,
//         quorum_type: QuorumType,
//         quorum_private_key: PrivateKey,
//     ) -> Self {
//         Self {
//             quorum_hash,
//             quorum_type,
//             quorum_private_key,
//         }
//     }
//
//     pub fn sign(
//         &self,
//         request_id: &QuorumSigningRequestId,
//         transaction: &Transaction,
//     ) -> Result<BLSSignature, ProtocolError> {
//         // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), txId).
//         // llmqType and quorumHash must be taken from the quorum selected in 1.
//         let mut engine = sha256d::Hash::engine();
//
//         let mut reversed_quorum_hash = self.quorum_hash.to_byte_array().to_vec();
//         reversed_quorum_hash.reverse();
//
//         engine.input(&[self.quorum_type as u8]);
//         engine.input(reversed_quorum_hash.as_slice());
//         engine.input(request_id.as_byte_array());
//         engine.input(transaction.txid().as_byte_array());
//
//         let message_digest = sha256d::Hash::from_engine(engine);
//
//         let g2element = self.quorum_private_key.sign(message_digest.as_ref());
//         let g2element_bytes = *g2element.to_bytes();
//
//         Ok(BLSSignature::from(g2element_bytes))
//     }
// }
//
// fn request_id_for_tx(tx: &Transaction) -> Result<QuorumSigningRequestId, ProtocolError> {
//     let mut engine = QuorumSigningRequestId::engine();
//
//     // Prefix
//     let prefix_len = VarInt(IS_LOCK_REQUEST_ID_PREFIX.len() as u64);
//     prefix_len.consensus_encode(&mut engine).map_err(|e| {
//         ProtocolError::CorruptedSerialization(format!("can't serialize varInt for request_id: {e}"))
//     })?;
//
//     engine.input(IS_LOCK_REQUEST_ID_PREFIX.as_bytes());
//
//     // Inputs
//     let inputs_len = VarInt(tx.input.len() as u64);
//     inputs_len.consensus_encode(&mut engine).map_err(|e| {
//         ProtocolError::CorruptedSerialization(format!("can't serialize varInt for request_id: {e}"))
//     })?;
//
//     for TxIn {
//         previous_output, ..
//     } in &tx.input
//     {
//         engine.input(&previous_output.txid[..]);
//         engine.input(&previous_output.vout.to_le_bytes());
//     }
//
//     Ok(QuorumSigningRequestId::from_engine(engine))
// }
