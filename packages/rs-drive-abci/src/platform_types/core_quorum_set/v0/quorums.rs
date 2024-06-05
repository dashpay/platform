use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::json::QuorumType;
use derive_more::{Deref, DerefMut, From};
use dpp::dashcore::QuorumHash;
use std::collections::BTreeMap;
use std::convert::TryInto;

pub use dpp::bls_signatures::PublicKey as ThresholdBlsPublicKey;

/// Reversed quorum hash bytes. Used for signature verification as part of the signature payload
pub type ReversedQuorumHashBytes = Vec<u8>;

/// Quorum verification data for signature verification
pub type QuorumVerificationData<'p> = (ReversedQuorumHashBytes, &'p ThresholdBlsPublicKey);

use dpp::dashcore::hashes::{sha256d, HashEngine};

type QuorumsInnerV0 = BTreeMap<QuorumHash, Quorum>;

/// Quorum per hash
#[derive(Debug, Clone, Deref, DerefMut, From, Default)]
pub struct Quorums(QuorumsInnerV0);

impl FromIterator<(QuorumHash, ThresholdBlsPublicKey)> for Quorums {
    fn from_iter<T: IntoIterator<Item = (QuorumHash, ThresholdBlsPublicKey)>>(iter: T) -> Self {
        let mut quorums = Quorums::default();

        for (hash, public_key) in iter {
            quorums.0.insert(
                hash,
                Quorum {
                    index: None,
                    public_key,
                },
            );
        }

        quorums
    }
}

impl FromIterator<(QuorumHash, ThresholdBlsPublicKey, Option<u32>)> for Quorums {
    fn from_iter<T: IntoIterator<Item = (QuorumHash, ThresholdBlsPublicKey, Option<u32>)>>(
        iter: T,
    ) -> Self {
        let mut quorums = Quorums::default();

        for (hash, public_key, index) in iter {
            quorums.0.insert(hash, Quorum { index, public_key });
        }

        quorums
    }
}

impl FromIterator<(QuorumHash, Quorum)> for Quorums {
    fn from_iter<T: IntoIterator<Item = (QuorumHash, Quorum)>>(iter: T) -> Self {
        let mut quorums = Quorums::default();

        for (hash, quorum) in iter {
            quorums.0.insert(hash, quorum);
        }

        quorums
    }
}

impl IntoIterator for Quorums {
    type Item = (QuorumHash, Quorum);
    type IntoIter = std::collections::btree_map::IntoIter<QuorumHash, Quorum>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Quorums {
    pub(super) fn find_classic_quorum_verification_data(
        &self,
        quorum_type: QuorumType,
        request_id: &[u8; 32],
    ) -> Option<QuorumVerificationData> {
        // Scoring system logic
        let mut scores: Vec<(ReversedQuorumHashBytes, &ThresholdBlsPublicKey, [u8; 32])> =
            Vec::new();

        for (quorum_hash, quorum) in self.0.iter() {
            let mut quorum_hash_bytes = quorum_hash.to_byte_array().to_vec();

            // Only the quorum hash needs reversal.
            quorum_hash_bytes.reverse();

            let mut hasher = sha256d::Hash::engine();

            // Serialize and hash the LLMQ type
            hasher.input(&[quorum_type as u8]);

            // Serialize and add the quorum hash
            hasher.input(quorum_hash_bytes.as_slice());

            // Serialize and add the selection hash from the chain lock
            hasher.input(request_id.as_slice());

            // Finalize the hash
            let hash_result = sha256d::Hash::from_engine(hasher);
            scores.push((quorum_hash_bytes, &quorum.public_key, hash_result.into()));
        }

        if scores.is_empty() {
            return None;
        }

        scores.sort_by_key(|k| k.2);

        let (quorum_reversed_hash, public_key, _) = scores.remove(0);

        Some((quorum_reversed_hash, public_key))
    }

    pub(super) fn find_rotating_quorum_verification_data(
        &self,
        quorums_signing_active_count: u16,
        request_id: &[u8; 32],
    ) -> Option<QuorumVerificationData> {
        let n = quorums_signing_active_count as u64;

        let b = u64::from_le_bytes(request_id[24..32].try_into().unwrap());

        // Take last n bits of b
        let mask = (1u64 << n) - 1;
        let signer = mask & (b >> (64 - n));

        self.0
            .iter()
            .find(|(_, quorum)| quorum.index == Some(signer as u32))
            .map(|(hash, quorum)| {
                let mut quorum_reversed_hash = hash.to_byte_array().to_vec();

                // Only the quorum hash needs reversal.
                quorum_reversed_hash.reverse();

                (quorum_reversed_hash, &quorum.public_key)
            })
    }
}

/// Quorum for signature verification
#[derive(Debug, Clone)]
pub struct Quorum {
    /// Index is present only for rotated quorums (DIP24)
    pub index: Option<u32>,

    /// Quorum threshold public key is used to verify
    /// signatures produced by corresponding quorum
    pub public_key: ThresholdBlsPublicKey,
}
