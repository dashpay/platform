use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_current_quorums_info_request::GetCurrentQuorumsInfoRequestV0;
use dapi_grpc::platform::v0::get_current_quorums_info_response::{
    GetCurrentQuorumsInfoResponseV0, ValidatorSetV0, ValidatorV0,
};
use dpp::dashcore::hashes::Hash;

impl<C> Platform<C> {
    pub(super) fn query_current_quorums_info_v0(
        &self,
        GetCurrentQuorumsInfoRequestV0 {}: GetCurrentQuorumsInfoRequestV0,
        platform_state: &PlatformState,
    ) -> Result<QueryValidationResult<GetCurrentQuorumsInfoResponseV0>, Error> {
        // Get all validator sets sorted by core height
        let validator_sets_sorted =
            platform_state.validator_sets_sorted_by_core_height_by_most_recent();

        // Collect the validator sets in the desired format
        let validator_sets: Vec<ValidatorSetV0> = validator_sets_sorted
            .iter()
            .map(|validator_set| {
                // Map each ProTxHash to a ValidatorV0 object
                let members: Vec<ValidatorV0> = validator_set
                    .members()
                    .iter()
                    .map(|(pro_tx_hash, validator)| ValidatorV0 {
                        pro_tx_hash: pro_tx_hash.as_byte_array().to_vec(),
                        node_ip: validator.node_ip.clone(),
                        is_banned: validator.is_banned,
                    })
                    .collect();

                // Construct a ValidatorSetV0 object
                ValidatorSetV0 {
                    quorum_hash: validator_set.quorum_hash().as_byte_array().to_vec(),
                    core_height: validator_set.core_height(),
                    members,
                    threshold_public_key: validator_set
                        .threshold_public_key()
                        .0
                        .to_compressed()
                        .to_vec(),
                }
            })
            .collect();

        // Get the current proposer index and current quorum index, if applicable
        let current_quorum_index = platform_state.current_validator_set_quorum_hash();
        let last_committed_block_proposer_pro_tx_hash =
            platform_state.last_committed_block_proposer_pro_tx_hash();

        // Construct the response
        let response = GetCurrentQuorumsInfoResponseV0 {
            quorum_hashes: validator_sets_sorted
                .iter()
                .map(|validator_set| validator_set.quorum_hash().as_byte_array().to_vec())
                .collect(),
            validator_sets,
            last_block_proposer: last_committed_block_proposer_pro_tx_hash.to_vec(),
            current_quorum_hash: current_quorum_index.as_byte_array().to_vec(),
            metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
        };

        // Return the response wrapped in a QueryValidationResult
        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mimic::test_quorum::TestQuorumInfo;
    use crate::platform_types::validator_set::ValidatorSet;
    use crate::query::tests::setup_platform;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Network;
    use dpp::dashcore::{ProTxHash, QuorumHash};
    use indexmap::IndexMap;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::sync::Arc;

    /// Build a `ValidatorSet::V0` for a given quorum hash, core height, and
    /// set of pro_tx_hashes. Uses the `TestQuorumInfo` helper so the BLS keys
    /// are well-formed.
    fn make_validator_set(
        quorum_hash: QuorumHash,
        core_height: u32,
        pro_tx_hashes: Vec<ProTxHash>,
        seed: u64,
    ) -> ValidatorSet {
        let mut rng = StdRng::seed_from_u64(seed);
        let info = TestQuorumInfo::from_quorum_hash_and_pro_tx_hashes(
            core_height,
            quorum_hash,
            None,
            pro_tx_hashes,
            &mut rng,
        );
        ValidatorSet::V0(info.into())
    }

    /// Store a map of validator sets, a current quorum hash, and an optional
    /// last-committed-block proposer into the platform state.
    fn install_validator_sets(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        sets: IndexMap<QuorumHash, ValidatorSet>,
        current_quorum_hash: QuorumHash,
        proposer_pro_tx_hash: Option<[u8; 32]>,
    ) {
        let mut state = (**platform.state.load()).clone();
        state.set_validator_sets(sets);
        state.set_current_validator_set_quorum_hash(current_quorum_hash);

        if let Some(proposer) = proposer_pro_tx_hash {
            state.set_last_committed_block_info(Some(
                ExtendedBlockInfoV0 {
                    basic_info: dpp::block::block_info::BlockInfo::default(),
                    app_hash: [0u8; 32],
                    quorum_hash: current_quorum_hash.as_byte_array().to_owned(),
                    block_id_hash: [0u8; 32],
                    proposer_pro_tx_hash: proposer,
                    signature: [0u8; 96],
                    round: 0,
                }
                .into(),
            ));
        }

        platform.state.store(Arc::new(state));
    }

    #[test]
    fn test_query_current_quorums_info_empty_state() {
        // On an initialized platform with no validator sets, the response
        // should contain empty quorum_hashes and validator_sets, with the
        // default all-zeroes current_quorum_hash.
        let (platform, state, _version) = setup_platform(None, Network::Testnet, None);

        let request = GetCurrentQuorumsInfoRequestV0 {};

        let result = platform
            .query_current_quorums_info_v0(request, &state)
            .expect("expected query to succeed");

        let data = result.into_data().expect("expected data");

        assert!(data.quorum_hashes.is_empty());
        assert!(data.validator_sets.is_empty());
        assert_eq!(data.current_quorum_hash.len(), 32);
        // all-zeros default quorum hash
        assert!(data.current_quorum_hash.iter().all(|b| *b == 0));
        // default proposer pro_tx_hash is all zero bytes at genesis
        assert_eq!(data.last_block_proposer.len(), 32);
        assert!(data.last_block_proposer.iter().all(|b| *b == 0));
        assert!(data.metadata.is_some());
    }

    #[test]
    fn test_query_current_quorums_info_single_set_mapping() {
        // With a single populated validator set we exercise the `.map` branch
        // that copies members, threshold public key, quorum hash, and core
        // height into the response.
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);

        let quorum_hash = QuorumHash::from_byte_array([7u8; 32]);
        let pro_tx_hashes = vec![
            ProTxHash::from_byte_array([1u8; 32]),
            ProTxHash::from_byte_array([2u8; 32]),
            ProTxHash::from_byte_array([3u8; 32]),
        ];
        let validator_set = make_validator_set(quorum_hash, 1000, pro_tx_hashes.clone(), 42);

        let mut sets = IndexMap::new();
        sets.insert(quorum_hash, validator_set);
        install_validator_sets(&platform, sets, quorum_hash, Some([9u8; 32]));

        // Re-load state now that we stored new one.
        let state = platform.state.load_full();

        let result = platform
            .query_current_quorums_info_v0(GetCurrentQuorumsInfoRequestV0 {}, &state)
            .expect("expected query to succeed");

        let data = result.into_data().expect("expected data");

        assert_eq!(data.quorum_hashes.len(), 1);
        assert_eq!(data.quorum_hashes[0], quorum_hash.as_byte_array().to_vec());
        assert_eq!(data.validator_sets.len(), 1);
        let vs = &data.validator_sets[0];
        assert_eq!(vs.quorum_hash, quorum_hash.as_byte_array().to_vec());
        assert_eq!(vs.core_height, 1000);
        assert_eq!(vs.members.len(), 3);
        // threshold public key is 48 bytes compressed BLS key
        assert_eq!(vs.threshold_public_key.len(), 48);
        // current_quorum_hash reflects what we set
        assert_eq!(
            data.current_quorum_hash,
            quorum_hash.as_byte_array().to_vec()
        );
        // last_block_proposer reflects the extended block info
        assert_eq!(data.last_block_proposer, vec![9u8; 32]);
    }

    #[test]
    fn test_query_current_quorums_info_multiple_sets_sorted_descending() {
        // When more than one validator set is present, the response must order
        // them by core_height descending. This covers the `sort_by_key` path.
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);

        let qh1 = QuorumHash::from_byte_array([1u8; 32]);
        let qh2 = QuorumHash::from_byte_array([2u8; 32]);
        let qh3 = QuorumHash::from_byte_array([3u8; 32]);

        let set_low = make_validator_set(
            qh1,
            100,
            vec![
                ProTxHash::from_byte_array([10u8; 32]),
                ProTxHash::from_byte_array([11u8; 32]),
            ],
            1,
        );
        let set_high = make_validator_set(
            qh2,
            500,
            vec![
                ProTxHash::from_byte_array([20u8; 32]),
                ProTxHash::from_byte_array([21u8; 32]),
            ],
            2,
        );
        let set_mid = make_validator_set(
            qh3,
            300,
            vec![
                ProTxHash::from_byte_array([30u8; 32]),
                ProTxHash::from_byte_array([31u8; 32]),
            ],
            3,
        );

        // Intentionally insert out of order. Response must sort by core_height desc.
        let mut sets = IndexMap::new();
        sets.insert(qh1, set_low);
        sets.insert(qh2, set_high);
        sets.insert(qh3, set_mid);

        install_validator_sets(&platform, sets, qh2, None);

        let state = platform.state.load_full();

        let data = platform
            .query_current_quorums_info_v0(GetCurrentQuorumsInfoRequestV0 {}, &state)
            .expect("expected query to succeed")
            .into_data()
            .expect("expected data");

        assert_eq!(data.validator_sets.len(), 3);
        // Descending by core_height
        let heights: Vec<u32> = data.validator_sets.iter().map(|v| v.core_height).collect();
        assert_eq!(heights, vec![500, 300, 100]);
        let returned_hashes: Vec<Vec<u8>> = data.quorum_hashes.to_vec();
        assert_eq!(returned_hashes.len(), 3);
        assert_eq!(returned_hashes[0], qh2.as_byte_array().to_vec());
        assert_eq!(returned_hashes[1], qh3.as_byte_array().to_vec());
        assert_eq!(returned_hashes[2], qh1.as_byte_array().to_vec());
        assert_eq!(data.current_quorum_hash, qh2.as_byte_array().to_vec());
    }

    #[test]
    fn test_query_current_quorums_info_banned_validator_propagates() {
        // Validators with `is_banned = true` should still appear in the
        // response but carry the flag through.
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);

        let quorum_hash = QuorumHash::from_byte_array([5u8; 32]);
        let pro_tx_hashes = vec![
            ProTxHash::from_byte_array([11u8; 32]),
            ProTxHash::from_byte_array([12u8; 32]),
        ];
        let mut validator_set = make_validator_set(quorum_hash, 42, pro_tx_hashes, 7);

        // Mutate one member to be banned. The `ValidatorSet` enum currently
        // has only one variant but we still destructure it rather than assume.
        let ValidatorSet::V0(v0) = &mut validator_set;
        let first_key = *v0.members.keys().next().expect("at least one member");
        v0.members.get_mut(&first_key).unwrap().is_banned = true;

        let mut sets = IndexMap::new();
        sets.insert(quorum_hash, validator_set);
        install_validator_sets(&platform, sets, quorum_hash, None);

        let state = platform.state.load_full();

        let data = platform
            .query_current_quorums_info_v0(GetCurrentQuorumsInfoRequestV0 {}, &state)
            .expect("expected query to succeed")
            .into_data()
            .expect("expected data");

        assert_eq!(data.validator_sets.len(), 1);
        let vs = &data.validator_sets[0];
        let banned_count = vs.members.iter().filter(|m| m.is_banned).count();
        assert_eq!(
            banned_count, 1,
            "one banned validator should be reported as banned"
        );
        let not_banned = vs.members.iter().filter(|m| !m.is_banned).count();
        assert_eq!(not_banned, 1);
        // node_ip is a non-empty string constructed by TestQuorumInfo
        for m in &vs.members {
            assert!(!m.node_ip.is_empty());
            assert_eq!(m.pro_tx_hash.len(), 32);
        }
    }

    #[test]
    fn test_query_current_quorums_info_current_quorum_hash_not_in_sets() {
        // The response returns `current_quorum_hash` even if no set in the map
        // actually matches it. This covers a defensive read path.
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);

        let present_hash = QuorumHash::from_byte_array([8u8; 32]);
        let unrelated_current = QuorumHash::from_byte_array([99u8; 32]);
        let set = make_validator_set(
            present_hash,
            250,
            vec![
                ProTxHash::from_byte_array([44u8; 32]),
                ProTxHash::from_byte_array([45u8; 32]),
            ],
            99,
        );

        let mut sets = IndexMap::new();
        sets.insert(present_hash, set);
        install_validator_sets(&platform, sets, unrelated_current, None);

        let state = platform.state.load_full();

        let data = platform
            .query_current_quorums_info_v0(GetCurrentQuorumsInfoRequestV0 {}, &state)
            .expect("expected query to succeed")
            .into_data()
            .expect("expected data");

        assert_eq!(data.validator_sets.len(), 1);
        assert_eq!(
            data.current_quorum_hash,
            unrelated_current.as_byte_array().to_vec()
        );
        // The set's quorum hash is NOT the current one.
        assert_ne!(data.validator_sets[0].quorum_hash, data.current_quorum_hash);
    }

    #[test]
    fn test_query_current_quorums_info_last_block_proposer_reflected() {
        // Verify the proposer pro-tx-hash from last_committed_block_info is
        // returned verbatim as `last_block_proposer`.
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);

        let quorum_hash = QuorumHash::from_byte_array([6u8; 32]);
        let set = make_validator_set(
            quorum_hash,
            10,
            vec![
                ProTxHash::from_byte_array([77u8; 32]),
                ProTxHash::from_byte_array([78u8; 32]),
            ],
            11,
        );
        let mut sets = IndexMap::new();
        sets.insert(quorum_hash, set);

        let proposer = [123u8; 32];
        install_validator_sets(&platform, sets, quorum_hash, Some(proposer));

        let state = platform.state.load_full();

        let data = platform
            .query_current_quorums_info_v0(GetCurrentQuorumsInfoRequestV0 {}, &state)
            .expect("expected query to succeed")
            .into_data()
            .expect("expected data");

        assert_eq!(data.last_block_proposer, proposer.to_vec());
    }
}
