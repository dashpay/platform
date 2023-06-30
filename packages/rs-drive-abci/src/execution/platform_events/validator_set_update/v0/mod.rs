use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// We need to validate against the platform state for rotation and not the block execution
    /// context state
    pub(in crate::execution) fn validator_set_update_v0(
        &self,
        platform_state: &platform_state::v0::PlatformState,
        block_execution_context: &mut block_execution_context::v0::BlockExecutionContext,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        let mut perform_rotation = false;

        if block_execution_context.block_state_info.height
            % self.config.validator_set_quorum_rotation_block_count as u64
            == 0
        {
            tracing::debug!(
                method = "validator_set_update_v0",
                "rotation: previous quorum finished members"
            );
            perform_rotation = true;
        }

        if let Some(current_quorum) = block_execution_context
            .block_platform_state
            .validator_sets
            .get(&platform_state.current_validator_set_quorum_hash)
        {
            // We need to perform a rotation if the quorum health is low
            if current_quorum.is_low_health(self.config.quorum_size) {
                perform_rotation = true;
            }
        }
        // we also need to perform a rotation if the validator set is being removed
        else {
            tracing::debug!(
                method = "validator_set_update_v0",
                "rotation: new quorums not containing current quorum current {:?}, {}",
                block_execution_context
                    .block_platform_state
                    .validator_sets
                    .keys()
                    .map(|quorum_hash| format!("{}", quorum_hash)),
                &platform_state.current_validator_set_quorum_hash
            );
            perform_rotation = true;
        }

        if perform_rotation {
            // get the index of the previous quorum
            let mut index = platform_state
                .validator_sets
                .get_index_of(&platform_state.current_validator_set_quorum_hash)
                .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                    "current quorums do not contain current validator set",
                )))?;
            // we should rotate the quorum
            let quorum_count = platform_state.validator_sets.len();
            match quorum_count {
                0 => Err(Error::Execution(ExecutionError::CorruptedCachedState(
                    "no current quorums",
                ))),
                1 => Ok(None),
                count => {
                    let start_index = index;
                    index = (index + 1) % count;
                    // We can't just take the next item because it might no longer be in the state
                    while index != start_index {
                        let (quorum_hash, _) = platform_state
                            .validator_sets
                            .get_index(index)
                            .expect("expected next validator set");
                        // We still have it in the state
                        if let Some(new_validator_set) = block_execution_context
                            .block_platform_state
                            .validator_sets
                            .get(quorum_hash)
                        {
                            tracing::debug!(
                                method = "validator_set_update_v0",
                                "rotation: to new quorum: {} with {} members",
                                &quorum_hash,
                                new_validator_set.members.len()
                            );
                            block_execution_context
                                .block_platform_state
                                .next_validator_set_quorum_hash = Some(*quorum_hash);
                            return Ok(Some(new_validator_set.into()));
                        }
                        index = (index + 1) % count;
                    }
                    // All quorums changed
                    if let Some((quorum_hash, new_quorum)) = block_execution_context
                        .block_platform_state
                        .validator_sets
                        .first()
                    {
                        block_execution_context
                            .block_platform_state
                            .next_validator_set_quorum_hash = Some(*quorum_hash);
                        tracing::debug!(
                            method = "validator_set_update_v0",
                            "rotation: all quorums changed, rotation to new quorum: {}",
                            &quorum_hash
                        );
                        return Ok(Some(new_quorum.into()));
                    }
                    tracing::debug!("no new quorums to choose from");
                    Ok(None)
                }
            }
        } else {
            let current_validator_set = block_execution_context
                .block_platform_state
                .current_validator_set()?;
            if current_validator_set != platform_state.current_validator_set()? {
                // Something changed, for example the IP of a validator changed, or someone's ban status

                tracing::debug!(
                    method = "validator_set_update_v0",
                    "validator set update without rotation"
                );
                Ok(Some(current_validator_set.into()))
            } else {
                tracing::debug!(
                    method = "validator_set_update_v0",
                    "no validator set update"
                );
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::execution::types::block_execution_context::v0::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfo;
    use std::collections::BTreeMap;

    use crate::platform_types::validator_set::v0::ValidatorSet;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::bls_signatures::PublicKey as BlsPublicKey;

    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{ProTxHash, QuorumHash};
    use std::ops::Deref;

    use crate::platform_types::validator::v0::Validator;

    fn generate_validator() -> Validator {
        Validator {
            pro_tx_hash: Default::default(),
            public_key: None,
            node_ip: "".to_string(),
            node_id: Default::default(),
            core_port: 0,
            platform_http_port: 0,
            platform_p2p_port: 0,
            is_banned: false,
        }
    }

    fn generate_healthy_validator_set() -> BTreeMap<ProTxHash, Validator> {
        BTreeMap::from_iter((1..=10).map(|_| {
            let validator = generate_validator();
            (validator.pro_tx_hash, validator)
        }))
    }

    fn generate_unhealthy_validator_set() -> BTreeMap<ProTxHash, Validator> {
        let mut set = BTreeMap::from_iter((1..=9).map(|_| {
            let validator = generate_validator();
            (validator.pro_tx_hash, validator)
        }));

        let mut unhealthy_validator = generate_validator();
        unhealthy_validator.is_banned = true;
        set.insert(unhealthy_validator.pro_tx_hash, unhealthy_validator);
        set
    }

    #[test]
    pub fn should_perform_rotation_when_low_health() {
        let test_platform = TestPlatformBuilder::new().build_with_mock_rpc();

        let mut platform_state = test_platform.platform.state.read().unwrap().clone();

        let current_quorum_hash = platform_state.current_validator_set_quorum_hash;
        let next_quorum_hash = QuorumHash::from_slice(&[1u8; 32]).unwrap();

        let current_quorum = ValidatorSet {
            quorum_hash: current_quorum_hash,
            core_height: 10,
            members: generate_healthy_validator_set(),
            threshold_public_key: BlsPublicKey::generate(),
        };

        let next_quorum = ValidatorSet {
            quorum_hash: next_quorum_hash,
            core_height: 11,
            members: generate_healthy_validator_set(),
            threshold_public_key: BlsPublicKey::generate(),
        };

        platform_state
            .validator_sets
            .insert(current_quorum_hash, current_quorum);
        platform_state
            .validator_sets
            .insert(next_quorum_hash, next_quorum);
        println!("validator sets: {:?}", platform_state.validator_sets);
        println!(
            "current validator set hash: {:?}",
            platform_state.current_validator_set_quorum_hash
        );

        let current_validator_set = platform_state
            .current_validator_set()
            .expect("Current validator set to exist")
            .deref()
            .clone();

        println!("current validator set: {:?}", current_validator_set);

        let mut execution_context = BlockExecutionContext {
            block_state_info: BlockStateInfo {
                height: 10,
                round: 0,
                block_time_ms: 0,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0; 32],
                core_chain_locked_height: 12,
                block_hash: None,
                app_hash: None,
            },
            epoch_info: Default::default(),
            hpmn_count: 0,
            withdrawal_transactions: Default::default(),
            block_platform_state: platform_state.clone(),
            proposer_results: None,
        };

        let res = test_platform
            .platform
            .validator_set_update_v0(&platform_state, &mut execution_context)
            .expect("To execute validator set update");

        // Current quorum is healthy, no need to rotate
        assert!(matches!(res, None));

        // Now replacing the current validator set with an unhealthy one
        let unhealthy_validator_set = ValidatorSet {
            quorum_hash: current_quorum_hash,
            core_height: 10,
            members: generate_unhealthy_validator_set(),
            threshold_public_key: BlsPublicKey::generate(),
        };
        // Checking that it is indeed unhealthy
        assert!(unhealthy_validator_set.is_low_health());
        // Replace the current validator set with the unhealthy one
        execution_context
            .block_platform_state
            .validator_sets
            .insert(current_quorum_hash, unhealthy_validator_set);

        let res = test_platform
            .platform
            .validator_set_update_v0(&platform_state, &mut execution_context)
            .expect("To execute validator set update");

        let update = res.expect("To have an update");
        // Check that we are indeed rotating to the next quorum
        assert_eq!(update.quorum_hash.to_vec(), next_quorum_hash.to_vec());
    }
}
