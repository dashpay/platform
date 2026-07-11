use dpp::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::QuorumHash;
use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
use std::collections::BTreeMap;

use crate::platform_types::platform::Platform;

use crate::execution::platform_events::core_chain_lock::choose_quorum::ReversedQuorumHashBytes;

impl<C> Platform<C> {
    /// Based on DIP8 deterministically chooses a pseudorandom quorum from the list of quorums
    pub(super) fn choose_quorum_v0<'a>(
        llmq_quorum_type: QuorumType,
        quorums: &'a BTreeMap<QuorumHash, BlsPublicKey<Bls12381G2Impl>>,
        request_id: &[u8; 32],
    ) -> Option<(ReversedQuorumHashBytes, &'a BlsPublicKey<Bls12381G2Impl>)> {
        // Scoring system logic
        let mut scores: Vec<(
            ReversedQuorumHashBytes,
            &BlsPublicKey<Bls12381G2Impl>,
            [u8; 32],
        )> = Vec::new();

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
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::bls_signatures::SecretKey;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::QuorumHash;
    use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    fn test_choose_quorum_v0_empty_quorums_returns_none() {
        let quorums = BTreeMap::new();
        let request_id = [0u8; 32];

        let result = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_choose_quorum_thread_safe_v0_empty_quorums_returns_none() {
        let quorums: BTreeMap<QuorumHash, [u8; 48]> = BTreeMap::new();
        let request_id = [0u8; 32];

        let result = Platform::<MockCoreRPCLike>::choose_quorum_thread_safe_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_choose_quorum_thread_safe_v0_single_quorum() {
        let quorum_hash = QuorumHash::from_slice(
            hex::decode("000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223")
                .expect("valid hex")
                .as_slice(),
        )
        .expect("valid quorum hash");

        let key = [42u8; 48];
        let quorums = BTreeMap::from([(quorum_hash, key)]);
        let request_id = [1u8; 32];

        let result = Platform::<MockCoreRPCLike>::choose_quorum_thread_safe_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        );

        let (_, returned_key) = result.expect("expected quorum selection for single quorum");
        assert_eq!(*returned_key, key);
    }

    #[test]
    fn test_choose_quorum_thread_safe_v0_deterministic() {
        let quorum_hash1 = QuorumHash::from_slice(
            hex::decode("000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223")
                .expect("valid hex")
                .as_slice(),
        )
        .expect("valid quorum hash");
        let quorum_hash2 = QuorumHash::from_slice(
            hex::decode("000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071")
                .expect("valid hex")
                .as_slice(),
        )
        .expect("valid quorum hash");

        let key1 = [1u8; 48];
        let key2 = [2u8; 48];
        let quorums = BTreeMap::from([(quorum_hash1, key1), (quorum_hash2, key2)]);
        let request_id = [42u8; 32];

        // Call twice with same inputs - should be deterministic
        let result1 = Platform::<MockCoreRPCLike>::choose_quorum_thread_safe_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .expect("expected quorum selection on first call");
        let result2 = Platform::<MockCoreRPCLike>::choose_quorum_thread_safe_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .expect("expected quorum selection on second call");

        assert_eq!(result1.0, result2.0);
    }

    #[test]
    fn test_choose_quorum_v0_single_quorum() {
        let quorum_hash = QuorumHash::from_slice(
            hex::decode("000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223")
                .expect("valid hex")
                .as_slice(),
        )
        .expect("valid quorum hash");

        let mut rng = StdRng::seed_from_u64(42);
        let key = SecretKey::random(&mut rng).public_key();
        let quorums = BTreeMap::from([(quorum_hash, key)]);
        let request_id = [1u8; 32];

        let result = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        );

        assert!(
            result.is_some(),
            "expected quorum selection for single quorum"
        );
    }

    /// `choose_quorum_v0` (BLS variant) must be deterministic across calls:
    /// given the same quorums and request_id, it picks the same quorum.
    /// The `choose_quorum_thread_safe_v0` variant is already covered, but
    /// this test pins the contract for the BLS variant used in production.
    #[test]
    fn test_choose_quorum_v0_deterministic_with_multiple_quorums() {
        let quorum_hash1 = QuorumHash::from_slice(
            hex::decode("000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223")
                .expect("hex")
                .as_slice(),
        )
        .expect("valid hash");
        let quorum_hash2 = QuorumHash::from_slice(
            hex::decode("000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071")
                .expect("hex")
                .as_slice(),
        )
        .expect("valid hash");
        let quorum_hash3 = QuorumHash::from_slice(
            hex::decode("0000006faac9003919a6d5456a0a46ae10db517f572221279f0540b79fd9cf1b")
                .expect("hex")
                .as_slice(),
        )
        .expect("valid hash");

        let mut rng = StdRng::seed_from_u64(7);
        let quorums = BTreeMap::from([
            (quorum_hash1, SecretKey::random(&mut rng).public_key()),
            (quorum_hash2, SecretKey::random(&mut rng).public_key()),
            (quorum_hash3, SecretKey::random(&mut rng).public_key()),
        ]);

        let request_id = [13u8; 32];

        let first = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .expect("should pick a quorum");

        let second = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &quorums,
            &request_id,
        )
        .expect("should pick a quorum");

        assert_eq!(
            first.0, second.0,
            "choose_quorum_v0 must be deterministic across calls with identical inputs"
        );
    }

    /// The two `choose_quorum_*_v0` variants use different sort/pop orders
    /// (BLS `sort_by_key` + `remove(0)` vs thread-safe `sort_by_key` +
    /// `pop()`), so for the same inputs they MAY select different quorums.
    /// What must be guaranteed is that both are individually deterministic
    /// and that `thread_safe_v0` picks the MAX-scoring quorum while `_v0`
    /// picks the MIN-scoring quorum. We verify this invariant here.
    #[test]
    fn test_choose_quorum_v0_vs_thread_safe_v0_pick_extreme_scores() {
        // Build identical quorum maps for both variants using dummy 48-byte
        // "keys" (good enough for thread-safe, which just takes [u8; T]).
        let quorum_hash1 = QuorumHash::from_slice(
            hex::decode("000000dc07d722238a994116c3395c334211d9864ff5b37c3be51d5fdda66223")
                .expect("hex")
                .as_slice(),
        )
        .expect("valid hash");
        let quorum_hash2 = QuorumHash::from_slice(
            hex::decode("000000bd5639c21dd8abf60253c3fe0343d87a9762b5b8f57e2b4ea1523fd071")
                .expect("hex")
                .as_slice(),
        )
        .expect("valid hash");
        let quorum_hash3 = QuorumHash::from_slice(
            hex::decode("0000006faac9003919a6d5456a0a46ae10db517f572221279f0540b79fd9cf1b")
                .expect("hex")
                .as_slice(),
        )
        .expect("valid hash");

        let key1: [u8; 48] = [1u8; 48];
        let key2: [u8; 48] = [2u8; 48];
        let key3: [u8; 48] = [3u8; 48];

        let thread_safe_quorums = BTreeMap::from([
            (quorum_hash1, key1),
            (quorum_hash2, key2),
            (quorum_hash3, key3),
        ]);

        let request_id = [99u8; 32];

        // Run the thread_safe_v0 variant to obtain the MAX-scoring quorum.
        let max_pick = Platform::<MockCoreRPCLike>::choose_quorum_thread_safe_v0(
            QuorumType::Llmq50_60,
            &thread_safe_quorums,
            &request_id,
        )
        .expect("pick");

        // Build the equivalent BLS map.
        let mut rng = StdRng::seed_from_u64(123);
        let pk1 = SecretKey::random(&mut rng).public_key();
        let pk2 = SecretKey::random(&mut rng).public_key();
        let pk3 = SecretKey::random(&mut rng).public_key();
        let bls_quorums = BTreeMap::from([
            (quorum_hash1, pk1),
            (quorum_hash2, pk2),
            (quorum_hash3, pk3),
        ]);

        let min_pick = Platform::<MockCoreRPCLike>::choose_quorum_v0(
            QuorumType::Llmq50_60,
            &bls_quorums,
            &request_id,
        )
        .expect("pick");

        // With three entries and distinct hashes, the MIN-scoring and
        // MAX-scoring reversed quorum hashes must differ.
        assert_ne!(
            max_pick.0, min_pick.0,
            "with 3 distinct quorums, v0 and thread_safe_v0 must select different extremes"
        );
    }

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
        let mut rng = StdRng::seed_from_u64(345);
        let quorums = BTreeMap::from([
            (quorum_hash1, SecretKey::random(&mut rng).public_key()),
            (quorum_hash2, SecretKey::random(&mut rng).public_key()),
            (quorum_hash3, SecretKey::random(&mut rng).public_key()),
            (quorum_hash4, SecretKey::random(&mut rng).public_key()),
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
