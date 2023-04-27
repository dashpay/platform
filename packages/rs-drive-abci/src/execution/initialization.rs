use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use crate::state::PlatformInitializationState;
use dashcore_rpc::dashcore_rpc_json::{
    Bip9SoftforkInfo, Bip9SoftforkStatus, GetChainTipsResultStatus,
};
use dpp::block::block_info::BlockInfo;
use dpp::identity::TimestampMillis;
use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;

use tenderdash_abci::proto::abci::{RequestInitChain, ResponseInitChain, ValidatorSetUpdate};
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

        let fork_info = self.core_rpc.get_fork_info("v20")?.ok_or(
            ExecutionError::InitializationForkNotActive("fork is not yet known".to_string()),
        )?;
        if fork_info.status != Bip9SoftforkStatus::Active {
            // fork is not good yet
            return Err(ExecutionError::InitializationForkNotActive(format!(
                "fork is not yet known (currently {:?})",
                fork_info.status
            ))
            .into());
        } else {
            tracing::debug!(?fork_info, "core fork v20 is active");
        };

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

        let core_height = self.initial_core_height(request.initial_core_height, &fork_info)?;
        self.update_core_info(
            &mut state_cache,
            core_height,
            true,
            &BlockInfo::genesis(),
            transaction,
        )?;

        let validator_set =
            state_cache
                .validator_sets
                .first()
                .ok_or(ExecutionError::InitializationError(
                    "we should have at least one quorum",
                ))?;
        let quorum_hash = validator_set.0;
        let validator_set = ValidatorSetUpdate::from(validator_set.1);

        state_cache.current_validator_set_quorum_hash = quorum_hash.to_owned();

        state_cache.initialization_information = Some(PlatformInitializationState {
            core_initialization_height: core_height,
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
            validator_set_update: Some(validator_set),
            next_core_chain_lock_update: None,
            initial_core_height: core_height, // we send back the core height when the fork happens
        })
    }

    /// Determine initial core height.
    ///
    /// TODO: rewrite this, it is non-deterministic
    /// We use either core height received from Tenderdash (from genesis file), OR the current tip of active core chain.
    /// We use current tip as default because we need a fully functional, up-to-date validator set.
    fn initial_core_height(
        &self,
        requested: u32,
        fork_info: &Bip9SoftforkInfo,
    ) -> Result<u32, Error> {
        let core_height = if requested != 0 {
            requested
        } else {
            fork_info.since
        };

        Ok(core_height)
    }
}
