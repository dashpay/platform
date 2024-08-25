use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

use dpp::block::block_info::BlockInfo;
use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;

use crate::platform_types::cleaned_abci_messages::request_init_chain_cleaned_params;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;
use std::sync::Arc;
use tenderdash_abci::proto::abci::{RequestInitChain, ResponseInitChain, ValidatorSetUpdate};
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::serializers::timestamp::FromMilis;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Initialize the chain
    pub(super) fn init_chain_v0(
        &self,
        request: RequestInitChain,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<ResponseInitChain, Error> {
        let request =
            request_init_chain_cleaned_params::v0::RequestInitChainCleanedParams::try_from(
                request,
            )?;

        // Wait until we have an initial core height to start the chain
        let (core_height, genesis_time) = loop {
            match self.initial_core_height_and_time(request.initial_core_height, platform_version) {
                Ok(height) => break height,
                Err(e) => match e {
                    Error::Execution(ExecutionError::InitializationForkNotActive(_))
                    | Error::Execution(ExecutionError::InitializationHeightIsNotLocked {
                        ..
                    })
                    | Error::Execution(ExecutionError::InitializationGenesisTimeInFuture {
                        ..
                    }) => {
                        tracing::warn!(
                            error = ?e,
                            "Failed to obtain deterministic initial core height to start the chain. Retrying in 30 seconds.",
                        );

                        // We need to wait for the fork to be active
                        std::thread::sleep(std::time::Duration::from_secs(30));
                    }
                    e => return Err(e),
                },
            }
        };

        // Create genesis drive state
        self.create_genesis_state(
            core_height,
            genesis_time,
            Some(transaction),
            platform_version,
        )?;

        // Create platform execution state
        let mut initial_platform_state = PlatformState::default_with_protocol_versions(
            request.initial_protocol_version,
            request.initial_protocol_version,
            &self.config,
        )?;

        let genesis_block_info = BlockInfo {
            height: request.initial_height,
            core_height,
            time_ms: genesis_time,
            ..Default::default()
        };

        // !!! Very important to understand !!!
        // We update the core info at the initial core height. This means that we use the quorums
        // at the initial core height for block 1.
        // The initial core height is either the height of the fork at which platform activates
        //  or it is the request.initial_core_height.
        // Block 1 is signed with the quorum chosen based on this info/height.
        // It is also worth saying that the quorum chosen will be the most recently built quorum.
        // On block 1 the proposer will most likely propose a new core chain locked height.
        // That will cause the core info to update again, so very often block 2 will be signed by
        //  a different quorum.

        self.update_core_info(
            None,
            &mut initial_platform_state,
            core_height,
            true,
            &genesis_block_info,
            transaction,
            platform_version,
        )?;

        let (quorum_hash, validator_set) = {
            let validator_set_inner = initial_platform_state.validator_sets().first().ok_or(
                ExecutionError::InitializationError("we should have at least one quorum"),
            )?;

            (
                *validator_set_inner.0,
                ValidatorSetUpdate::from(validator_set_inner.1),
            )
        };

        initial_platform_state.set_current_validator_set_quorum_hash(quorum_hash);

        initial_platform_state.set_genesis_block_info(Some(genesis_block_info));

        initial_platform_state
            .set_current_protocol_version_in_consensus(request.initial_protocol_version);

        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                platform_state_fingerprint = hex::encode(initial_platform_state.fingerprint()?),
                "platform runtime state",
            );
        }

        self.state.store(Arc::new(initial_platform_state));

        let app_hash = self
            .drive
            .grove
            .root_hash(Some(transaction), &platform_version.drive.grove_version)
            .unwrap()
            .map_err(GroveDB)?;

        Ok(ResponseInitChain {
            consensus_params: None,
            app_hash: app_hash.to_vec(),
            validator_set_update: Some(validator_set),
            next_core_chain_lock_update: None,
            initial_core_height: core_height, // we send back the core height when the fork happens
            genesis_time: Some(Timestamp::from_milis(genesis_time)),
        })
    }
}
