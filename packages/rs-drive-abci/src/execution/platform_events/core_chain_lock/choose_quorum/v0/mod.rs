use crate::error::Error;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::{ChainLock, QuorumHash};
use dpp::platform_value::Bytes32;
use sha2::Sha256;
use std::collections::BTreeMap;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    pub(super) fn choose_quorum_v0<'a>(
        llmq_quorum_type: QuorumType,
        quorums: &'a BTreeMap<QuorumHash, BlsPublicKey>,
        request_id: &[u8; 32],
        _platform_version: &PlatformVersion,
    ) -> Option<(&'a QuorumHash, &'a BlsPublicKey)> {
        // Scoring system logic
        let mut scores: Vec<(&QuorumHash, &BlsPublicKey, [u8; 32])> = Vec::new();

        for (quorum_hash, public_key) in quorums {
            let mut hasher = sha256d::Hash::engine();

            // Serialize and hash the LLMQ type
            hasher.input(&[llmq_quorum_type as u8]);

            // Serialize and add the quorum hash
            hasher.input(quorum_hash.as_byte_array());

            // Serialize and add the selection hash from the chain lock
            hasher.input(request_id.as_slice());

            // Finalize the hash
            let hash_result = sha256d::Hash::from_engine(hasher);
            scores.push((quorum_hash, public_key, hash_result.into()));
        }

        scores.sort_by_key(|k| k.2);
        scores.first().map(|&(hash, key, _)| (hash, key))
    }

    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    pub(super) fn choose_quorum_thread_safe_v0<'a, const T: usize>(
        llmq_quorum_type: QuorumType,
        quorums: &'a BTreeMap<QuorumHash, [u8; T]>,
        request_id: &[u8; 32],
        _platform_version: &PlatformVersion,
    ) -> Option<(&'a QuorumHash, &'a [u8; T])> {
        // Scoring system logic
        let mut scores: Vec<(&QuorumHash, &[u8; T], [u8; 32])> = Vec::new();

        for (quorum_hash, key) in quorums {
            let mut hasher = sha256d::Hash::engine();

            // Serialize and hash the LLMQ type
            hasher.input(&[llmq_quorum_type as u8]);

            // Serialize and add the quorum hash
            hasher.input(quorum_hash.as_byte_array());

            // Serialize and add the selection hash from the chain lock
            hasher.input(request_id.as_slice());

            // Finalize the hash
            let hash_result = sha256d::Hash::from_engine(hasher);
            scores.push((quorum_hash, key, hash_result.into()));
        }

        scores.sort_by_key(|k| k.2);
        scores.first().map(|&(hash, key, _)| (hash, key))
    }
}
