use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformInitializationState;
use dashcore::hashes::Hash;
use dashcore::QuorumHash;
use dashcore_rpc::dashcore_rpc_json::Bip9SoftforkStatus;
use dpp::block::block_info::BlockInfo;
use dpp::identity::TimestampMillis;
use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;
use std::cmp::Ordering;
use tenderdash_abci::proto::abci::{RequestInitChain, ResponseInitChain};
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Initialize the chain
    pub fn init_chain(
        &self,
        request: RequestInitChain,
        transaction: &Transaction,
    ) -> Result<ResponseInitChain, Error> {
        // We receive the activation height, if core is not yet at this height

        let fork_info = self.core_rpc.get_fork_info("DEPLOYMENT_V20")?.ok_or(
            ExecutionError::InitializationForkNotActive("fork is not yet known".to_string()),
        )?;
        if fork_info.status != Bip9SoftforkStatus::Active {
            // fork is not good yet
            return Err(ExecutionError::InitializationForkNotActive(format!(
                "fork is not yet known (currently {:?})",
                fork_info.status
            ))
            .into());
        }

        let genesis_time = request
            .time
            .ok_or(Error::Execution(ExecutionError::InitializationError(
                "genesis time is required in init chain",
            )))?
            .to_milis() as TimestampMillis;

        self.create_genesis_state(
            genesis_time,
            self.config.abci.keys.clone().into(),
            Some(transaction),
        )?;

        let mut state_cache = self.state.write().unwrap();

        self.update_quorum_info(&mut state_cache, fork_info.since, true)?;

        self.update_masternode_list(
            &mut state_cache,
            fork_info.since,
            true,
            &BlockInfo::genesis(),
            transaction,
        )?;

        state_cache.current_validator_set_quorum_hash = state_cache
            .validator_sets
            .get_index(0)
            .ok_or(ExecutionError::InitializationError(
                "we should have at least one quorum",
            ))
            .map(|(quorum_hash, _)| *quorum_hash)?;

        state_cache.initialization_information = Some(PlatformInitializationState {
            core_initialization_height: fork_info.since,
        });

        let app_hash = self
            .drive
            .grove
            .root_hash(Some(transaction))
            .unwrap()
            .map_err(GroveDB)?;

        Ok(ResponseInitChain {
            consensus_params: None, //todo
            app_hash: app_hash.to_vec(),
            validator_set_update: None,
            next_core_chain_lock_update: None,
            initial_core_height: fork_info.since, // we send back the core height when the fork happens
        })
    }
}
