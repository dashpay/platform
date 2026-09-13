use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::platform_types::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Network;
use std::sync::atomic::Ordering;
use tenderdash_abci::proto::abci as proto;

pub fn finalize_block<'a, A, C>(
    app: &A,
    request: proto::RequestFinalizeBlock,
) -> Result<proto::ResponseFinalizeBlock, Error>
where
    A: PlatformApplication<C> + TransactionalApplication<'a> + BlockExecutionApplication,
    C: CoreRPCLike,
{
    let _timer = crate::metrics::abci_request_duration("finalize_block");

    let transaction_guard = app.transaction().read().unwrap();
    let transaction =
        transaction_guard
            .as_ref()
            .ok_or(Error::Execution(ExecutionError::NotInTransaction(
                "trying to finalize block without a current transaction",
            )))?;

    // Get current block platform version
    let block_execution_context = app
        .block_execution_context()
        .write()
        .unwrap()
        .take()
        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "block execution context must be set in block begin handler for finalize block",
        )))?;

    let platform_version = block_execution_context
        .block_platform_state()
        .current_platform_version()?;

    let request_finalize_block: FinalizeBlockCleanedRequest = request.try_into()?;

    let block_height = request_finalize_block.height;

    let block_finalization_outcome = app.platform().finalize_block_proposal(
        request_finalize_block,
        block_execution_context,
        transaction,
        platform_version,
    )?;

    drop(transaction_guard);

    //FIXME: tell tenderdash about the problem instead
    // This can not go to production!
    if !block_finalization_outcome.validation_result.is_valid() {
        return Err(Error::Abci(
            block_finalization_outcome
                .validation_result
                .errors
                .into_iter()
                .next()
                .unwrap(),
        ));
    }

    let result = app.commit_transaction(platform_version);

    // Mainnet's vote-cleanup incident began at 32326 (platform#2309). Tenderdash#966
    // let validators continue after the resulting commit conflict with partially committed
    // state and updated caches. Reproduce that outcome only for the expected Busy error.
    // The upper bound must match Drive::remove_all_votes_given_by_identities: starting
    // at 32329, vote deletions stay inside the block transaction again.
    let config = &app.platform().config;
    let historical_conflict = config.network == Network::Mainnet
        && config.abci.chain_id == "evo1"
        && (32326..32329).contains(&block_height)
        && matches!(
            &result,
            Err(Error::Drive(drive::error::Error::GroveDB(error)))
                if matches!(
                    error.as_ref(),
                    drive::grovedb::Error::StorageError(
                        drive::grovedb_storage::error::Error::RocksDBError(error)
                    ) if error.kind() == rocksdb::ErrorKind::Busy
                )
        );

    if historical_conflict {
        tracing::warn!(
            error = ?result.as_ref().err(),
            block_height,
            "historical mainnet vote-cleanup commit conflict; proceeding as the \
             network did at the time (see platform#2309, tenderdash#966)"
        );
    } else {
        // A failed commit leaves caches ahead of durable state. Restart Drive so the
        // caches are restored from disk before retrying the block.
        result.expect("commit transaction");
    }

    app.platform()
        .committed_block_height_guard
        .store(block_height, Ordering::Relaxed);

    // Reclaim the storage of any TTL'd time-range buckets the block's
    // transaction flat-dropped (grovedb#848 / PR #849): the drops' redo
    // records became visible at the commit above, and the flush turns them
    // into DB-level range tombstones. Outside consensus by design — it
    // never touches the root hash, the report is telemetry, and a failure
    // leaves the records in place for the next flush (or the startup
    // flush) to retry, so the finalized block is unaffected.
    match app
        .platform()
        .drive
        .grove
        .flush_pending_prefix_drops(&platform_version.drive.grove_version)
    {
        Ok(report) => {
            if report.reclaimed_records > 0 || report.skipped_live > 0 {
                tracing::debug!(
                    reclaimed_records = report.reclaimed_records,
                    skipped_live = report.skipped_live,
                    "flushed pending prefix drops"
                );
            }
        }
        Err(error) => {
            tracing::warn!(
                ?error,
                "failed to flush pending prefix drops; records persist and will be retried"
            );
        }
    }

    // Create GroveDB checkpoint after the transaction is committed (so it captures committed state)
    if block_finalization_outcome.checkpoint_needed {
        app.platform().create_grovedb_checkpoint(platform_version)?;
    }

    Ok(proto::ResponseFinalizeBlock { retain_height: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::{FullAbciApplication, TransactionalApplication};
    use crate::config::PlatformConfig;
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform::Platform;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::version::PlatformVersion;
    use drive::grovedb::Transaction;
    use std::sync::RwLock;
    use tenderdash_abci::proto::abci::CommitInfo;
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::types::{
        Block, BlockId, Data, EvidenceList, Header, PartSetHeader,
    };
    use tenderdash_abci::proto::version::Consensus;

    /// An ABCI application whose `commit_transaction` always fails.
    ///
    /// Injects either a real RocksDB error or an unrelated execution error to verify
    /// that only historical transaction conflicts are tolerated.
    struct FailingCommitApplication<'a> {
        platform: &'a Platform<MockCoreRPCLike>,
        commit_error: RwLock<Option<Error>>,
        transaction: RwLock<Option<Transaction<'a>>>,
        block_execution_context: RwLock<Option<BlockExecutionContext>>,
    }

    impl PlatformApplication<MockCoreRPCLike> for FailingCommitApplication<'_> {
        fn platform(&self) -> &Platform<MockCoreRPCLike> {
            self.platform
        }
    }

    impl BlockExecutionApplication for FailingCommitApplication<'_> {
        fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>> {
            &self.block_execution_context
        }
    }

    impl<'a> TransactionalApplication<'a> for FailingCommitApplication<'a> {
        fn start_transaction(&self) {
            let transaction = self.platform.drive.grove.start_transaction();
            self.transaction.write().unwrap().replace(transaction);
        }

        fn transaction(&self) -> &RwLock<Option<Transaction<'a>>> {
            &self.transaction
        }

        fn commit_transaction(&self, _platform_version: &PlatformVersion) -> Result<(), Error> {
            // Consume the transaction like the real implementation would, then fail.
            self.transaction.write().unwrap().take();
            Err(self
                .commit_error
                .write()
                .unwrap()
                .take()
                .expect("commit error"))
        }
    }

    /// Runs `finalize_block` at `height` against a platform configured with `config`, with a
    /// commit that is guaranteed to fail.
    fn finalize_block_with_failing_commit(
        config: PlatformConfig,
        height: u64,
        commit_error: Error,
    ) -> Result<proto::ResponseFinalizeBlock, Error> {
        let platform: TempPlatform<MockCoreRPCLike> = TestPlatformBuilder::new()
            .with_config(config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let app = FailingCommitApplication {
            platform: &platform.platform,
            commit_error: RwLock::new(Some(commit_error)),
            transaction: Default::default(),
            block_execution_context: Default::default(),
        };

        app.start_transaction();

        let platform_state = (**platform.state.load()).clone();
        let quorum_hash: [u8; 32] = platform_state.current_validator_set_quorum_hash().into();

        let block_hash = [1u8; 32];
        let app_hash = [2u8; 32];
        let block_time_ms = 1_700_000_000_000u64;

        app.block_execution_context.write().unwrap().replace(
            BlockExecutionContextV0 {
                block_state_info: BlockStateInfoV0 {
                    height,
                    round: 0,
                    block_time_ms,
                    previous_block_time_ms: None,
                    proposer_pro_tx_hash: [0u8; 32],
                    core_chain_locked_height: 1,
                    block_hash: Some(block_hash),
                    app_hash: Some(app_hash),
                }
                .into(),
                epoch_info: EpochInfo::V0(EpochInfoV0 {
                    current_epoch_index: 0,
                    previous_epoch_index: None,
                    is_epoch_change: false,
                }),
                unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
                block_address_balance_changes: Default::default(),
                block_platform_state: platform_state,
                proposer_results: None,
            }
            .into(),
        );

        let request = proto::RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: 0,
                quorum_hash: quorum_hash.to_vec(),
                block_signature: vec![0u8; 96],
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: block_hash.to_vec(),
            height: height as i64,
            round: 0,
            block: Some(Block {
                header: Some(Header {
                    version: Some(Consensus { block: 0, app: 1 }),
                    chain_id: platform.config.abci.chain_id.clone(),
                    height: height as i64,
                    time: Some(Timestamp {
                        seconds: (block_time_ms / 1000) as i64,
                        nanos: 0,
                    }),
                    last_block_id: None,
                    last_commit_hash: vec![0u8; 32],
                    data_hash: vec![0u8; 32],
                    validators_hash: vec![0u8; 32],
                    next_validators_hash: vec![0u8; 32],
                    consensus_hash: vec![0u8; 32],
                    next_consensus_hash: vec![0u8; 32],
                    app_hash: app_hash.to_vec(),
                    results_hash: vec![0u8; 32],
                    evidence_hash: vec![],
                    proposed_app_version: 1,
                    proposer_pro_tx_hash: vec![0u8; 32],
                    core_chain_locked_height: 1,
                }),
                data: Some(Data { txs: vec![] }),
                evidence: Some(EvidenceList { evidence: vec![] }),
                last_commit: None,
                core_chain_lock: None,
            }),
            block_id: Some(BlockId {
                hash: block_hash.to_vec(),
                part_set_header: Some(PartSetHeader {
                    total: 0,
                    hash: vec![0u8; 32],
                }),
                state_id: vec![0u8; 32],
            }),
        };

        finalize_block::<_, MockCoreRPCLike>(&app, request)
    }

    fn mainnet_evo1_config() -> PlatformConfig {
        let mut config = PlatformConfig::default_mainnet();
        config.abci.chain_id = "evo1".to_string();
        config.testing_configs.block_commit_signature_verification = false;
        config
    }

    /// Produce the actual RocksDB error returned when another transaction writes the same key.
    fn busy_commit_error() -> Error {
        let directory = tempfile::tempdir().expect("temporary database directory");
        let db = rocksdb::OptimisticTransactionDB::<rocksdb::SingleThreaded>::open_default(
            directory.path(),
        )
        .expect("open database");
        let transaction = db.transaction();
        transaction
            .put(b"key", b"pending")
            .expect("transaction write");
        db.put(b"key", b"committed").expect("independent write");
        let error = transaction
            .commit()
            .expect_err("conflicting commit must fail");
        assert_eq!(error.kind(), rocksdb::ErrorKind::Busy);
        drive::grovedb::Error::StorageError(drive::grovedb_storage::error::Error::RocksDBError(
            error,
        ))
        .into()
    }

    #[test]
    fn finalize_block_tolerates_busy_commit_on_mainnet_evo1_during_incident() {
        for height in 32326..32329 {
            let result = finalize_block_with_failing_commit(
                mainnet_evo1_config(),
                height,
                busy_commit_error(),
            );
            assert!(
                result.is_ok(),
                "historical conflict at {height}: {result:?}"
            );
        }
    }

    #[test]
    #[should_panic(expected = "commit transaction")]
    fn finalize_block_panics_on_busy_commit_before_incident() {
        let _ =
            finalize_block_with_failing_commit(mainnet_evo1_config(), 32325, busy_commit_error());
    }

    #[test]
    #[should_panic(expected = "commit transaction")]
    fn finalize_block_panics_on_busy_commit_on_mainnet_evo1_at_32329() {
        // The second crash is prevented by keeping vote deletions inside the transaction
        // at 32329, not by tolerating another failed commit here.
        let _ =
            finalize_block_with_failing_commit(mainnet_evo1_config(), 32329, busy_commit_error());
    }

    #[test]
    #[should_panic(expected = "commit transaction")]
    fn finalize_block_panics_on_busy_commit_outside_mainnet() {
        let mut config = PlatformConfig::default_testnet();
        config.abci.chain_id = "evo1".to_string();
        config.testing_configs.block_commit_signature_verification = false;
        let _ = finalize_block_with_failing_commit(config, 32326, busy_commit_error());
    }

    #[test]
    #[should_panic(expected = "commit transaction")]
    fn finalize_block_panics_on_busy_commit_with_another_mainnet_chain_id() {
        let mut config = mainnet_evo1_config();
        config.abci.chain_id = "another-chain".to_string();
        let _ = finalize_block_with_failing_commit(config, 32326, busy_commit_error());
    }

    #[test]
    #[should_panic(expected = "commit transaction")]
    fn finalize_block_panics_on_io_error_during_incident() {
        // A regular file cannot serve as RocksDB's write-ahead log directory.
        let directory = tempfile::tempdir().expect("temporary database directory");
        let file = tempfile::NamedTempFile::new().expect("temporary file");
        let mut options = rocksdb::Options::default();
        options.create_if_missing(true);
        options.set_wal_dir(file.path());
        let error = rocksdb::OptimisticTransactionDB::<rocksdb::SingleThreaded>::open(
            &options,
            directory.path(),
        )
        .expect_err("WAL path is not a directory");
        assert_eq!(error.kind(), rocksdb::ErrorKind::IOError);
        let error = drive::grovedb::Error::StorageError(
            drive::grovedb_storage::error::Error::RocksDBError(error),
        )
        .into();
        let _ = finalize_block_with_failing_commit(mainnet_evo1_config(), 32328, error);
    }

    #[test]
    #[should_panic(expected = "commit transaction")]
    fn finalize_block_panics_on_execution_error_during_incident() {
        let error = Error::Execution(ExecutionError::CorruptedCodeExecution(
            "injected commit failure",
        ));
        let _ = finalize_block_with_failing_commit(mainnet_evo1_config(), 32326, error);
    }

    #[test]
    fn finalize_block_fails_when_no_transaction() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // No transaction started, no block execution context
        let request = proto::RequestFinalizeBlock {
            hash: vec![0u8; 32],
            height: 1,
            round: 0,
            ..Default::default()
        };

        let result = finalize_block::<_, MockCoreRPCLike>(&app, request);
        assert!(
            matches!(
                result,
                Err(Error::Execution(ExecutionError::NotInTransaction(_)))
            ),
            "Expected NotInTransaction error, got: {result:?}"
        );
    }

    #[test]
    fn finalize_block_fails_when_no_block_execution_context() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let app = FullAbciApplication::<MockCoreRPCLike>::new(&platform.platform);

        // Start a transaction but do not set block execution context
        app.start_transaction();

        let request = proto::RequestFinalizeBlock {
            hash: vec![0u8; 32],
            height: 1,
            round: 0,
            ..Default::default()
        };

        let result = finalize_block::<_, MockCoreRPCLike>(&app, request);
        assert!(
            matches!(
                result,
                Err(Error::Execution(ExecutionError::CorruptedCodeExecution(_)))
            ),
            "Expected CorruptedCodeExecution error, got: {result:?}"
        );
    }
}
