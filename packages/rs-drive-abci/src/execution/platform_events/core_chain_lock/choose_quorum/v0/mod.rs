

use std::collections::BTreeMap;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use sha2::Sha256;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::{ChainLock, QuorumHash};
use dpp::dashcore::hashes::{Hash, HashEngine, sha256d};
use dpp::platform_value::Bytes32;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    pub(super) fn choose_quorum_v0<'a>(&self, llmq_quorum_type: QuorumType, quorums: &'a BTreeMap<QuorumHash, BlsPublicKey>, request_id: &[u8;32], _platform_version: &PlatformVersion) -> Option<(&'a QuorumHash, &'a BlsPublicKey)> {
        // Scoring system logic
        let mut scores: Vec<(&QuorumHash, &BlsPublicKey, [u8;32])> = Vec::new();

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
            scores.push((quorum_hash, public_key,  hash_result.into()));
        }

        scores.sort_by_key(|k| k.2);
        scores.first().map(|&(hash, key, _)| (hash, key))
    }

}
