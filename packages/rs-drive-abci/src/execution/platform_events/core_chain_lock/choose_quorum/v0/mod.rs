use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::{ChainLock, QuorumHash};
use std::collections::BTreeMap;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::execution::platform_events::core_chain_lock::choose_quorum::ReversedQuorumHashBytes;

impl<C> Platform<C> {
    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    pub(super) fn choose_quorum_v0<'a>(
        llmq_quorum_type: QuorumType,
        quorums: &'a BTreeMap<QuorumHash, BlsPublicKey>,
        request_id: &[u8; 32],
    ) -> Option<(ReversedQuorumHashBytes, &'a BlsPublicKey)> {
        // Scoring system logic
        let mut scores: Vec<(ReversedQuorumHashBytes, &BlsPublicKey, [u8; 32])> = Vec::new();

        for (quorum_hash, public_key) in quorums {
            let mut quorum_hash_bytes = quorum_hash.to_byte_array().to_vec();

            // Only the quorum hash needs reversal.
            quorum_hash_bytes.reverse();

            let mut hasher = sha256d::Hash::engine();

            // Serialize and hash the LLMQ type
            hasher.input(&[llmq_quorum_type as u8]);

            // Serialize and add the quorum hash
            hasher.input(quorum_hash_bytes.as_slice());

            // Serialize and add the selection hash from the chain lock
            hasher.input(request_id.as_slice());

            // Finalize the hash
            let hash_result = sha256d::Hash::from_engine(hasher);
            scores.push((quorum_hash_bytes, public_key, hash_result.into()));
        }

        if scores.is_empty() {
            None
        } else {
            scores.sort_by_key(|k| k.2);

            let (quorum_hash, key, _) = scores.remove(0);

            Some((quorum_hash, key))
        }
    }

    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    pub(super) fn choose_quorum_thread_safe_v0<'a, const T: usize>(
        llmq_quorum_type: QuorumType,
        quorums: &'a BTreeMap<QuorumHash, [u8; T]>,
        request_id: &[u8; 32],
    ) -> Option<(ReversedQuorumHashBytes, &'a [u8; T])> {
        // Scoring system logic
        let mut scores: Vec<(ReversedQuorumHashBytes, &[u8; T], [u8; 32])> = Vec::new();

        for (quorum_hash, key) in quorums {
            let mut quorum_hash_bytes = quorum_hash.to_byte_array().to_vec();

            // Only the quorum hash needs reversal.
            quorum_hash_bytes.reverse();

            let mut hasher = sha256d::Hash::engine();

            // Serialize and hash the LLMQ type
            hasher.input(&[llmq_quorum_type as u8]);

            // Serialize and add the quorum hash
            hasher.input(quorum_hash_bytes.as_slice());

            // Serialize and add the selection hash from the chain lock
            hasher.input(request_id.as_slice());

            // Finalize the hash
            let hash_result = sha256d::Hash::from_engine(hasher);
            scores.push((quorum_hash_bytes, key, hash_result.into()));
        }

        scores.sort_by_key(|k| k.2);
        scores.pop().map(|(hash, key, _)| (hash, key))
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::platform::Platform;
    use crate::rpc::core::{CoreRPCLike, MockCoreRPCLike};
    use dashcore_rpc::dashcore_rpc_json::QuorumType;
    use dpp::bls_signatures::PublicKey as BlsPublicKey;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::QuorumHash;
    use std::collections::BTreeMap;

    #[test]
    fn test_choose_quorum() {
        // Active quorums:
        let quorum_hash1 = QuorumHash::from_slice(
            hex::decode("000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223")
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let quorum_hash2 = QuorumHash::from_slice(
            hex::decode("000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071")
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let quorum_hash3 = QuorumHash::from_slice(
            hex::decode("0000006faac9003919a6d5456a0a46ae10db517f572221279f0540b79fd9cf1b")
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let quorum_hash4 = QuorumHash::from_slice(
            hex::decode("0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e")
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let quorums = BTreeMap::from([
            (quorum_hash1, BlsPublicKey::generate()),
            (quorum_hash2, BlsPublicKey::generate()),
            (quorum_hash3, BlsPublicKey::generate()),
            (quorum_hash4, BlsPublicKey::generate()),
        ]);

        //
        // ###############
        // llmqType[1] requestID[bdcf9fb3ef01209a09db19170a1950775afb5f824c5f0662b9cdae2bf3bb36d5] -> 0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e
        // llmqType[4] requestID[bdcf9fb3ef01209a09db19170a1950775afb5f824c5f0662b9cdae2bf3bb36d5] -> 000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071
        // llmqType[5] requestID[bdcf9fb3ef01209a09db19170a1950775afb5f824c5f0662b9cdae2bf3bb36d5] -> 000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223
        // llmqType[100] requestID[bdcf9fb3ef01209a09db19170a1950775afb5f824c5f0662b9cdae2bf3bb36d5] -> 0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e

        let mut request_id: [u8; 32] =
            hex::decode("bdcf9fb3ef01209a09db19170a1950775afb5f824c5f0662b9cdae2bf3bb36d5")
                .unwrap()
                .try_into()
                .unwrap();

        request_id.reverse();

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq100_67,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq60_75,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::LlmqTest,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e"
        );

        // ###############
        // llmqType[1] requestID[b06aa45eb35423f988e36c022967b4c02bb719b037717df13fa57c0f503d8a20] -> 0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e
        // llmqType[4] requestID[b06aa45eb35423f988e36c022967b4c02bb719b037717df13fa57c0f503d8a20] -> 000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071
        // llmqType[5] requestID[b06aa45eb35423f988e36c022967b4c02bb719b037717df13fa57c0f503d8a20] -> 000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071
        // llmqType[100] requestID[b06aa45eb35423f988e36c022967b4c02bb719b037717df13fa57c0f503d8a20] -> 0000006faac9003919a6d5456a0a46ae10db517f572221279f0540b79fd9cf1b

        let mut request_id: [u8; 32] =
            hex::decode("b06aa45eb35423f988e36c022967b4c02bb719b037717df13fa57c0f503d8a20")
                .unwrap()
                .try_into()
                .unwrap();

        request_id.reverse();

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq100_67,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq60_75,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::LlmqTest,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "0000006faac9003919a6d5456a0a46ae10db517f572221279f0540b79fd9cf1b"
        );

        // ###############
        // llmqType[1] requestID[2fc41ef02a3216e4311805a9a11405a41a8d7a9f179526b4f6f2866bff009a10] -> 000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071
        // llmqType[4] requestID[2fc41ef02a3216e4311805a9a11405a41a8d7a9f179526b4f6f2866bff009a10] -> 0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e
        // llmqType[5] requestID[2fc41ef02a3216e4311805a9a11405a41a8d7a9f179526b4f6f2866bff009a10] -> 000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071
        // llmqType[100] requestID[2fc41ef02a3216e4311805a9a11405a41a8d7a9f179526b4f6f2866bff009a10] -> 000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223

        let mut request_id: [u8; 32] =
            hex::decode("2fc41ef02a3216e4311805a9a11405a41a8d7a9f179526b4f6f2866bff009a10")
                .unwrap()
                .try_into()
                .unwrap();

        request_id.reverse();

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq100_67,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "0000000e6d15a11825211c943c4a995c44ebb2b0834b7848c2e080b48ca0148e"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq60_75,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071"
        );

        let mut quorum = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::LlmqTest,
            &quorums,
            &request_id,
        )
        .unwrap()
        .0;

        quorum.reverse();

        assert_eq!(
            hex::encode(quorum),
            "000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223"
        );
    }
}
