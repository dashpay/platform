use dpp::bls::PublicKey as BlsPublicKey;
use dpp::dashcore::{ProTxHash, QuorumHash};
use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

/// The public key shares of a validator quorum's members, by member.
pub(crate) type QuorumKeyShares = BTreeMap<ProTxHash, BlsPublicKey>;

/// How many quorums' key shares are kept. A validator quorum type has about 24 quorums active
/// at a time; twice that keeps the shares of every quorum a block executed again can still send.
const MAX_KEPT_QUORUMS: usize = 48;

/// Validator quorum members' key shares that recovered their quorum's public key, kept for the
/// life of the process.
///
/// A quorum's key shares are fixed by its mined commitment, so a share once kept is the answer
/// for good: a quorum's entry only grows. A block executed again (FinalizeBlock executes the
/// committed block again when it was not the last one processed) then sends Tenderdash the keys
/// it sent the first time, even if Core cannot answer that time. Only the oldest quorums leave,
/// beyond [`MAX_KEPT_QUORUMS`].
#[derive(Debug, Default)]
pub(crate) struct QuorumKeyShareCache {
    quorums: Mutex<IndexMap<QuorumHash, Arc<QuorumKeyShares>>>,
}

impl QuorumKeyShareCache {
    /// The key shares kept for `quorum_hash`.
    pub(crate) fn get(&self, quorum_hash: &QuorumHash) -> Option<Arc<QuorumKeyShares>> {
        // Every write leaves the map whole, so a lock poisoned by a panic elsewhere still guards
        // a usable map.
        self.quorums
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(quorum_hash)
            .cloned()
    }

    /// Adds the shares of `key_shares` not yet kept for `quorum_hash`, never replacing a kept
    /// one, and returns everything kept for the quorum.
    pub(crate) fn keep(
        &self,
        quorum_hash: QuorumHash,
        key_shares: QuorumKeyShares,
    ) -> Arc<QuorumKeyShares> {
        let mut quorums = self.quorums.lock().unwrap_or_else(PoisonError::into_inner);
        let kept = match quorums.get(&quorum_hash) {
            Some(kept) => {
                let mut merged = QuorumKeyShares::clone(kept);
                for (pro_tx_hash, key) in key_shares {
                    merged.entry(pro_tx_hash).or_insert(key);
                }
                Arc::new(merged)
            }
            None => Arc::new(key_shares),
        };
        // An existing quorum keeps its place in the order.
        quorums.insert(quorum_hash, Arc::clone(&kept));
        while quorums.len() > MAX_KEPT_QUORUMS {
            quorums.shift_remove_index(0);
        }
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::bls::SecretKey;
    use dpp::dashcore::hashes::Hash;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn quorum_hash(index: u32) -> QuorumHash {
        let mut bytes = [0u8; 32];
        bytes[..4].copy_from_slice(&index.to_be_bytes());
        QuorumHash::from_byte_array(bytes)
    }

    fn key_shares(seed: u64) -> QuorumKeyShares {
        let mut rng = StdRng::seed_from_u64(seed);
        BTreeMap::from([(member(1), SecretKey::random(&mut rng).public_key())])
    }

    fn member(index: u8) -> ProTxHash {
        ProTxHash::from_byte_array([index; 32])
    }

    /// A quorum's shares are fixed by its commitment, so a share once kept is what every later
    /// execution sends, and a later answer only adds members.
    #[test]
    fn should_only_add_shares_to_a_kept_quorum() {
        let cache = QuorumKeyShareCache::default();
        let first = key_shares(1);
        let mut later = BTreeMap::from([(member(2), key_shares(3)[&member(1)])]);
        later.insert(member(1), key_shares(2)[&member(1)]);

        cache.keep(quorum_hash(1), first.clone());
        let kept = cache.keep(quorum_hash(1), later.clone());

        assert_eq!(kept.get(&member(1)), first.get(&member(1)));
        assert_eq!(kept.get(&member(2)), later.get(&member(2)));
        assert_eq!(cache.get(&quorum_hash(1)), Some(kept));
    }

    /// The cache must not grow for the life of the process: quorums expire daily.
    #[test]
    fn should_drop_the_oldest_quorums_beyond_the_limit() {
        let cache = QuorumKeyShareCache::default();
        for index in 0..=MAX_KEPT_QUORUMS as u32 {
            cache.keep(quorum_hash(index), key_shares(index as u64));
        }

        assert!(cache.get(&quorum_hash(0)).is_none());
        assert!(cache.get(&quorum_hash(1)).is_some());
        assert!(cache.get(&quorum_hash(MAX_KEPT_QUORUMS as u32)).is_some());
    }
}
