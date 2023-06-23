use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::Bip9SoftforkStatus;
use dpp::block::block_info::BlockInfo;
use drive::error::Error::GroveDB;
use drive::grovedb::Transaction;

use crate::platform_types::cleaned_abci_messages::request_init_chain_cleaned_params;
use crate::platform_types::platform_state::v0::{
    PlatformInitializationState, PlatformStateV0Methods,
};
use tenderdash_abci::proto::abci::{RequestInitChain, ResponseInitChain, ValidatorSetUpdate};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Initialize the chain
    pub(super) fn init_chain_v0(
        &self,
        request: RequestInitChain,
        transaction: &Transaction,
    ) -> Result<ResponseInitChain, Error> {
        let request =
            request_init_chain_cleaned_params::v0::RequestInitChainCleanedParams::try_from(
                request,
            )?;
        // We get core height early, as this also verifies v20 fork
        let core_height = self.initial_core_height(request.initial_core_height)?;

        let genesis_time = request.genesis_time;

        self.create_genesis_state_v0(
            genesis_time,
            self.config.abci.keys.clone().into(),
            Some(transaction),
        )?;

        let mut state_cache = self.state.write().unwrap();

        self.update_core_info_v0(
            None,
            &mut state_cache,
            core_height,
            true,
            &BlockInfo::genesis(),
            transaction,
        )?;

        let (quorum_hash, validator_set) =
            {
                let validator_set_inner = state_cache.validator_sets().first().ok_or(
                    ExecutionError::InitializationError("we should have at least one quorum"),
                )?;

                (
                    *validator_set_inner.0,
                    ValidatorSetUpdate::from(validator_set_inner.1),
                )
            };

        state_cache.set_current_validator_set_quorum_hash(quorum_hash);

        state_cache.set_initialization_information(Some(PlatformInitializationState {
            core_initialization_height: core_height,
        }));

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
    /// Use core height received from Tenderdash (from genesis.json) by default,
    /// otherwise we go with height of v20 fork.
    ///
    /// Core height is verified to ensure that it is both at or after v20 fork, and
    /// before or at last chain lock.
    ///
    /// ## Error handling
    ///
    /// This function will fail if:
    ///
    /// * v20 fork is not yet active
    /// * `requested` core height is before v20 fork
    /// * `requested` core height is after current best chain lock
    ///
    fn initial_core_height(&self, requested: Option<u32>) -> Result<u32, Error> {
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
        let v20_fork = fork_info.since;

        if let Some(requested) = requested {
            let best = self.core_rpc.get_best_chain_lock()?.core_block_height;

            tracing::trace!(
                requested,
                v20_fork,
                best,
                "selecting initial core lock height"
            );
            // TODO in my opinion, the condition should be:
            //
            // `v20_fork <= requested && requested <= best`
            //
            // but it results in 1440 <=  1243 <= 1545
            //
            // So, fork_info.since differs? is it non-deterministic?
            if requested <= best {
                Ok(requested)
            } else {
                Err(ExecutionError::InitializationBadCoreLockedHeight {
                    requested,
                    best,
                    v20_fork,
                }
                .into())
            }
        } else {
            tracing::trace!(v20_fork, "used fork height as initial core lock height");
            Ok(v20_fork)
        }
    }
}
