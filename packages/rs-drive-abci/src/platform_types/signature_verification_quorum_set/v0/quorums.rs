use derive_more::{Deref, DerefMut, From};
use dpp::bls_signatures;
pub use dpp::bls_signatures::PublicKey as ThresholdBlsPublicKey;
use dpp::bls_signatures::{Bls12381G2Impl, SignatureSchemes};
use dpp::dashcore::bls_sig_utils::BLSSignature;
use dpp::dashcore::{QuorumHash, Txid};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::fmt::Debug;

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
    pub public_key: ThresholdBlsPublicKey<Bls12381G2Impl>,
}

impl Debug for VerificationQuorum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerificationQuorum")
            .field("index", &self.index)
            .field("public_key", &self.public_key.to_string())
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
            bls_signatures::SecretKey::<Bls12381G2Impl>::from_be_bytes(&self.private_key)
                .into_option()
                .ok_or(Error::BLSError(
                    dpp::bls_signatures::BlsError::DeserializationError(
                        "Could not deserialize private key".to_string(),
                    ),
                ))?;

        let signature = private_key
            .sign(
                SignatureSchemes::Basic,
                message_digest.as_byte_array().as_slice(),
            )
            .map_err(Error::BLSError)?;

        Ok(BLSSignature::from(signature.as_raw_value().to_compressed()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey as BlsPrivateKey};
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore_rpc::json::QuorumType;

    /// Helper: generate a deterministic BLS public key from a seed byte.
    fn make_public_key(seed: u8) -> ThresholdBlsPublicKey<Bls12381G2Impl> {
        let mut key_bytes = [0u8; 32];
        key_bytes[0] = seed;
        key_bytes[31] = 1; // ensure nonzero
        let sk = BlsPrivateKey::<Bls12381G2Impl>::from_be_bytes(&key_bytes)
            .expect("expected a valid secret key from test bytes");
        sk.public_key()
    }

    fn make_verification_quorum(seed: u8, index: Option<u32>) -> VerificationQuorum {
        VerificationQuorum {
            index,
            public_key: make_public_key(seed),
        }
    }

    fn make_classic_config() -> QuorumConfig {
        QuorumConfig {
            quorum_type: QuorumType::Llmq100_67,
            active_signers: 24,
            rotation: false,
            window: 24,
        }
    }

    fn make_rotating_config(active_signers: u16) -> QuorumConfig {
        QuorumConfig {
            quorum_type: QuorumType::Llmq60_75,
            active_signers,
            rotation: true,
            window: 24,
        }
    }

    // ---- Quorums default and construction ----

    #[test]
    fn quorums_default_is_empty() {
        let q: Quorums<VerificationQuorum> = Quorums::default();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn quorums_from_iter_collects_entries() {
        let hash1 = QuorumHash::from_byte_array([1u8; 32]);
        let hash2 = QuorumHash::from_byte_array([2u8; 32]);
        let q: Quorums<VerificationQuorum> = vec![
            (hash1, make_verification_quorum(10, None)),
            (hash2, make_verification_quorum(20, None)),
        ]
        .into_iter()
        .collect();
        assert_eq!(q.len(), 2);
        assert!(q.contains_key(&hash1));
        assert!(q.contains_key(&hash2));
    }

    #[test]
    fn quorums_into_iter_yields_all_entries() {
        let hash1 = QuorumHash::from_byte_array([3u8; 32]);
        let hash2 = QuorumHash::from_byte_array([4u8; 32]);
        let q: Quorums<VerificationQuorum> = vec![
            (hash1, make_verification_quorum(30, None)),
            (hash2, make_verification_quorum(40, None)),
        ]
        .into_iter()
        .collect();
        let entries: Vec<_> = q.into_iter().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn quorums_from_btreemap() {
        let mut map = BTreeMap::new();
        map.insert(
            QuorumHash::from_byte_array([5u8; 32]),
            make_verification_quorum(50, None),
        );
        let q: Quorums<VerificationQuorum> = Quorums::from(map);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn quorums_deref_and_deref_mut() {
        let hash = QuorumHash::from_byte_array([6u8; 32]);
        let mut q: Quorums<VerificationQuorum> = Quorums::default();
        // DerefMut: insert via BTreeMap method
        q.insert(hash, make_verification_quorum(60, None));
        assert_eq!(q.len(), 1);
        // Deref: get via BTreeMap method
        assert!(q.get(&hash).is_some());
    }

    // ---- choose_quorum: classic (DIP8) ----

    #[test]
    fn choose_classic_quorum_empty_returns_none() {
        let q: Quorums<VerificationQuorum> = Quorums::default();
        let config = make_classic_config();
        let request_id = [0u8; 32];
        assert!(q.choose_quorum(&config, &request_id).is_none());
    }

    #[test]
    fn choose_classic_quorum_single_returns_that_quorum() {
        let hash = QuorumHash::from_byte_array([7u8; 32]);
        let q: Quorums<VerificationQuorum> = vec![(hash, make_verification_quorum(70, None))]
            .into_iter()
            .collect();
        let config = make_classic_config();
        let request_id = [0u8; 32];
        let result = q.choose_quorum(&config, &request_id);
        assert!(result.is_some());
        let (chosen_hash, _) = result.unwrap();
        assert_eq!(chosen_hash, hash);
    }

    #[test]
    fn choose_classic_quorum_deterministic() {
        let hash1 = QuorumHash::from_byte_array([8u8; 32]);
        let hash2 = QuorumHash::from_byte_array([9u8; 32]);
        let q: Quorums<VerificationQuorum> = vec![
            (hash1, make_verification_quorum(80, None)),
            (hash2, make_verification_quorum(90, None)),
        ]
        .into_iter()
        .collect();
        let config = make_classic_config();
        let request_id = [42u8; 32];

        let result1 = q.choose_quorum(&config, &request_id);
        let result2 = q.choose_quorum(&config, &request_id);
        assert_eq!(result1.unwrap().0, result2.unwrap().0);
    }

    #[test]
    fn choose_classic_quorum_different_request_ids_may_differ() {
        let hash1 = QuorumHash::from_byte_array([10u8; 32]);
        let hash2 = QuorumHash::from_byte_array([11u8; 32]);
        let hash3 = QuorumHash::from_byte_array([12u8; 32]);
        let q: Quorums<VerificationQuorum> = vec![
            (hash1, make_verification_quorum(1, None)),
            (hash2, make_verification_quorum(2, None)),
            (hash3, make_verification_quorum(3, None)),
        ]
        .into_iter()
        .collect();
        let config = make_classic_config();

        // Try many request IDs; at least two distinct choices should appear
        let mut chosen = std::collections::HashSet::new();
        for i in 0u8..=255 {
            let mut rid = [0u8; 32];
            rid[0] = i;
            if let Some((h, _)) = q.choose_quorum(&config, &rid) {
                chosen.insert(h);
            }
        }
        assert!(
            chosen.len() > 1,
            "classic quorum selection should distribute across quorums"
        );
    }

    // ---- choose_quorum: rotating (DIP24) ----

    #[test]
    fn choose_rotating_quorum_empty_returns_none() {
        let q: Quorums<VerificationQuorum> = Quorums::default();
        let config = make_rotating_config(32);
        let request_id = [0u8; 32];
        assert!(q.choose_quorum(&config, &request_id).is_none());
    }

    #[test]
    fn choose_rotating_quorum_finds_matching_index() {
        // active_signers = 32, so n = 5 (since 2^5 = 32), mask = 31
        // We need to control request_id so the computed signer index matches an existing quorum.
        let config = make_rotating_config(32);

        // Build quorums with indices 0..31
        let quorums: Quorums<VerificationQuorum> = (0u32..32)
            .map(|i| {
                let mut hash_bytes = [0u8; 32];
                hash_bytes[0] = i as u8;
                (
                    QuorumHash::from_byte_array(hash_bytes),
                    make_verification_quorum(i as u8, Some(i)),
                )
            })
            .collect();

        let request_id = [0u8; 32];
        let result = quorums.choose_quorum(&config, &request_id);
        assert!(
            result.is_some(),
            "rotating quorum should find a matching index"
        );
        let (_, chosen_quorum) = result.unwrap();
        assert!(chosen_quorum.index.is_some());
    }

    #[test]
    fn choose_rotating_quorum_no_matching_index_returns_none() {
        // Create a quorum with an index that will likely not match the computed signer
        let config = make_rotating_config(32);
        // Only one quorum with index 999 (out of range for mask = 31)
        let q: Quorums<VerificationQuorum> = vec![(
            QuorumHash::from_byte_array([1u8; 32]),
            make_verification_quorum(1, Some(999)),
        )]
        .into_iter()
        .collect();

        let request_id = [0u8; 32];
        let result = q.choose_quorum(&config, &request_id);
        assert!(
            result.is_none(),
            "no quorum should match index 999 when mask is 31"
        );
    }

    #[test]
    fn choose_quorum_routes_by_config_rotation_flag() {
        let hash = QuorumHash::from_byte_array([20u8; 32]);
        let quorum = make_verification_quorum(20, Some(0));
        let q: Quorums<VerificationQuorum> = vec![(hash, quorum)].into_iter().collect();

        let request_id = [0u8; 32];

        // Non-rotating config should use classic selection
        let classic_config = make_classic_config();
        let classic_result = q.choose_quorum(&classic_config, &request_id);
        assert!(classic_result.is_some());

        // Rotating config may or may not find a match depending on the computed signer
        let rotating_config = make_rotating_config(1);
        let _rotating_result = q.choose_quorum(&rotating_config, &request_id);
        // We just verify it does not panic; result depends on signer calculation
    }

    // ---- Quorum trait implementations ----

    #[test]
    fn verification_quorum_index_trait() {
        let vq_none = make_verification_quorum(1, None);
        assert_eq!(Quorum::index(&vq_none), None);

        let vq_some = make_verification_quorum(2, Some(42));
        assert_eq!(Quorum::index(&vq_some), Some(42));
    }

    #[test]
    fn signing_quorum_index_trait() {
        let sq = SigningQuorum {
            index: Some(7),
            private_key: [0u8; 32],
        };
        assert_eq!(Quorum::index(&sq), Some(7));

        let sq_none = SigningQuorum {
            index: None,
            private_key: [0u8; 32],
        };
        assert_eq!(Quorum::index(&sq_none), None);
    }

    // ---- Debug implementations ----

    #[test]
    fn verification_quorum_debug_format() {
        let vq = make_verification_quorum(1, Some(5));
        let debug_str = format!("{:?}", vq);
        assert!(debug_str.contains("VerificationQuorum"));
        assert!(debug_str.contains("index"));
        assert!(debug_str.contains("public_key"));
    }

    #[test]
    fn quorums_debug_format() {
        let hash = QuorumHash::from_byte_array([1u8; 32]);
        let q: Quorums<VerificationQuorum> = vec![(hash, make_verification_quorum(1, None))]
            .into_iter()
            .collect();
        let debug_str = format!("{:?}", q);
        // Should use debug_map format with quorum hash strings as keys
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn signing_quorum_debug_format() {
        let sq = SigningQuorum {
            index: Some(3),
            private_key: [0u8; 32],
        };
        let debug_str = format!("{:?}", sq);
        assert!(debug_str.contains("SigningQuorum"));
        assert!(debug_str.contains("index"));
    }
}
