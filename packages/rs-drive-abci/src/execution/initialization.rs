use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore::hashes::Hash;
use dashcore::QuorumHash;
use dpp::identity::TimestampMillis;
use drive::drive::block_info::BlockInfo;
use tenderdash_abci::proto::abci::RequestInitChain;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Initialize the chain
    pub fn init_chain(&self, request: RequestInitChain) -> Result<(), Error> {
        let transaction = self.drive.grove.start_transaction();
        let genesis_time = request
            .time
            .ok_or(Error::Execution(ExecutionError::InitializationError(
                "genesis time is required in init chain",
            )))?
            .to_milis() as TimestampMillis;

        self.create_genesis_state(
            genesis_time,
            self.config.abci.keys.clone().into(),
            Some(&transaction),
        )?;

        let mut state_cache = self.state.write().unwrap();

        self.update_quorum_info(&mut state_cache, request.initial_core_height)?;

        self.update_masternode_list(
            &mut state_cache,
            request.initial_core_height,
            &BlockInfo::genesis(),
            &transaction,
        )?;

        state_cache.current_validator_set_quorum_hash = QuorumHash::from_slice(
            request
                .validator_set
                .expect("expected validator set on init chain")
                .quorum_hash
                .as_slice(),
        )
        .expect("expected initial valid quorum hash");

        self.drive
            .commit_transaction(transaction)
            .map_err(Error::Drive)
    }
}
