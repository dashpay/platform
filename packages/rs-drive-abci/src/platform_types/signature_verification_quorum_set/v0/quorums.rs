use derive_more::{Deref, DerefMut, From};
use dpp::bls_signatures::PrivateKey;
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::{QuorumHash, Txid};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::Debug;

pub use dpp::bls_signatures::PublicKey as ThresholdBlsPublicKey;

use crate::error::Error;
use crate::platform_types::signature_verification_quorum_set::QuorumConfig;
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};

/// Quorum per hash
#[derive(Clone, Deref, DerefMut, From)]
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
        let active_signers = quorum_config.active_signers as u32;

        // binary (base-2) logarithm from active_signers
        let n = 31 - active_signers.leading_zeros();

        // Extract last 64 bits of request_id
        let b = u64::from_le_bytes(
            request_id[24..32]
                .try_into()
                .expect("request_id is [u8; 32]"),
        );

        // Take last n bits of b
        let mask = (1u64 << n) - 1;
        let signer = (mask & (b >> (64 - n - 1))) as u32;

        self.0
            .iter()
            .find(|(_, quorum)| quorum.index() == Some(signer))
            .map(|(quorum_hash, quorum)| (*quorum_hash, quorum))
    }
}

impl<Q: Debug> Debug for Quorums<Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .iter()
                    .map(|(quorum_hash, quorum)| (quorum_hash.to_string(), quorum)),
            )
            .finish()
    }
}

/// Quorum trait for Quorums collection
pub trait Quorum {
    /// Index is present only for rotated quorums (DIP24)
    fn index(&self) -> Option<u32>;
}

/// Quorum for signature verification
#[derive(Clone)]
pub struct VerificationQuorum {
    /// Index is present only for rotated quorums (DIP24)
    pub index: Option<u32>,

    /// Quorum threshold public key is used to verify
    /// signatures produced by corresponding quorum
    pub public_key: ThresholdBlsPublicKey,
}

impl Debug for VerificationQuorum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerificationQuorum")
            .field("index", &self.index)
            .field(
                "public_key",
                &hex::encode(*self.public_key.to_bytes()).to_string(),
            )
            .finish()
    }
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
