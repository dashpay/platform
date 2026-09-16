use crate::abci::app::{BlockExecutionApplication, PlatformApplication, TransactionalApplication};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::platform_types::cleaned_abci_messages::finalized_block_cleaned_request::v0::FinalizeBlockCleanedRequest;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Network;
use std::sync::atomic::Ordering;
use std::sync::Arc;
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
    #[cfg(debug_assertions)]
    let mut phases = crate::perf::PhaseTimer::new("finalize_block");

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

    #[cfg(debug_assertions)]
    phases.end_phase("setup");

    let block_finalization_outcome = app.platform().finalize_block_proposal(
        request_finalize_block,
        block_execution_context,
        transaction,
        platform_version,
    )?;

    #[cfg(debug_assertions)]
    phases.end_phase("finalize_block_proposal");

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

        // The block's saved platform state was in the failed transaction, so
        // what is on disk is behind the state cache this block published as
        // saved. Mark all of it unsaved again: the next block then writes the
        // full record and every entry rather than only its own changes on top
        // of stale ones, which a restart would otherwise read back.
        let platform = app.platform();
        let mut state = platform.state.load().as_ref().clone();
        state.mark_all_unsaved();
        platform.state.store(Arc::new(state));
    } else {
        // A failed commit leaves caches ahead of durable state. Restart Drive so the
        // caches are restored from disk before retrying the block.
        result.expect("commit transaction");
    }

    #[cfg(debug_assertions)]
    phases.end_phase("commit_transaction");

    // The block is durable now, so the contracts it read and rewrote through its transaction
    // are committed state: promote them to the global cache. This must follow the commit.
    // Promoting earlier would serve an uncommitted block's definitions to committed-state
    // readers and, worse, let a query that read committed state between the promotion and
    // the commit (still the pre-block definition) publish it as current afterwards.
    app.platform()
        .drive
        .cache
        .data_contracts
        .merge_and_clear_block_cache();

    #[cfg(debug_assertions)]
    phases.end_phase("promote_data_contract_cache");

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

    #[cfg(debug_assertions)]
    phases.end_phase("flush_pending_prefix_drops");

    // Create GroveDB checkpoint after the transaction is committed (so it captures
    // committed state). Checkpoints are restore points, auxiliary to the block: the
    // block is final once the commit above succeeded, and the transaction and block
    // execution context are already consumed, so a checkpoint failure must not be
    // reported as a failed block. Every node checkpoints at the same heights, so a
    // failed attempt is retried once right here, at this height, and otherwise the
    // checkpoint is skipped: the attempt leaves nothing behind and counts as this
    // interval's, so `should_checkpoint` does not ask again before the next one.
    if block_finalization_outcome.checkpoint_needed {
        let platform = app.platform();
        let result = platform
            .create_grovedb_checkpoint(platform_version)
            .or_else(|error| {
                tracing::warn!(
                    ?error,
                    block_height,
                    "failed to create grovedb checkpoint; retrying once"
                );
                platform.create_grovedb_checkpoint(platform_version)
            });
        match result {
            Ok(()) => {
                crate::metrics::abci_last_checkpoint_height(block_height);
                tracing::debug!(block_height, "created grovedb checkpoint");
            }
            Err(error) => {
                crate::metrics::abci_checkpoint_failed();
                tracing::error!(
                    ?error,
                    block_height,
                    "failed to create grovedb checkpoint twice after committing the block; \
                     the block is final and this checkpoint is skipped"
                );
            }
        }
    }

    #[cfg(debug_assertions)]
    phases.end_phase_if(
        block_finalization_outcome.checkpoint_needed,
        "create_grovedb_checkpoint",
    );
    // Merge this handler's phases into the totals before the block is counted.
    #[cfg(debug_assertions)]
    drop(phases);
    #[cfg(debug_assertions)]
    crate::perf::end_block(block_height);

    Ok(proto::ResponseFinalizeBlock { retain_height: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abci::app::{FullAbciApplication, TransactionalApplication};
    use crate::config::{CheckpointStep, PlatformConfig};
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform::Platform;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::version::PlatformVersion;
    use drive::grovedb::Transaction;
    use std::sync::{Arc, Mutex, RwLock};
    use tenderdash_abci::proto::abci::CommitInfo;
    use tenderdash_abci::proto::google::protobuf::Timestamp;
    use tenderdash_abci::proto::types::{
        Block, BlockId, Data, EvidenceList, Header, PartSetHeader,
    };
    use tenderdash_abci::proto::version::Consensus;
    use tracing_subscriber::layer::SubscriberExt;

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

    const BLOCK_HASH: [u8; 32] = [1u8; 32];
    const APP_HASH: [u8; 32] = [2u8; 32];

    /// The block execution context `finalize_block` expects for a block at `height`.
    fn block_execution_context(
        platform_state: PlatformState,
        height: u64,
        block_time_ms: u64,
        previous_block_time_ms: Option<u64>,
    ) -> BlockExecutionContext {
        BlockExecutionContextV0 {
            block_state_info: BlockStateInfoV0 {
                height,
                round: 0,
                block_time_ms,
                previous_block_time_ms,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: Some(BLOCK_HASH),
                app_hash: Some(APP_HASH),
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
        .into()
    }

    /// The finalize request for the block [`block_execution_context`] describes.
    fn finalize_block_request(
        chain_id: String,
        quorum_hash: [u8; 32],
        height: u64,
        block_time_ms: u64,
    ) -> proto::RequestFinalizeBlock {
        proto::RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: 0,
                quorum_hash: quorum_hash.to_vec(),
                block_signature: vec![0u8; 96],
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: BLOCK_HASH.to_vec(),
            height: height as i64,
            round: 0,
            block: Some(Block {
                header: Some(Header {
                    version: Some(Consensus { block: 0, app: 1 }),
                    chain_id,
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
                    app_hash: APP_HASH.to_vec(),
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
                hash: BLOCK_HASH.to_vec(),
                part_set_header: Some(PartSetHeader {
                    total: 0,
                    hash: vec![0u8; 32],
                }),
                state_id: vec![0u8; 32],
            }),
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

        finalize_block_with_failing_commit_on(&platform, height, commit_error)
    }

    /// Like [`finalize_block_with_failing_commit`], on a platform the caller keeps
    /// so it can inspect the state left behind.
    fn finalize_block_with_failing_commit_on(
        platform: &TempPlatform<MockCoreRPCLike>,
        height: u64,
        commit_error: Error,
    ) -> Result<proto::ResponseFinalizeBlock, Error> {
        let app = FailingCommitApplication {
            platform: &platform.platform,
            commit_error: RwLock::new(Some(commit_error)),
            transaction: Default::default(),
            block_execution_context: Default::default(),
        };

        app.start_transaction();

        let platform_state = (**platform.state.load()).clone();
        let quorum_hash: [u8; 32] = platform_state.current_validator_set_quorum_hash().into();
        let block_time_ms = 1_700_000_000_000u64;

        app.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context(
                platform_state,
                height,
                block_time_ms,
                None,
            ));

        let request = finalize_block_request(
            platform.config.abci.chain_id.clone(),
            quorum_hash,
            height,
            block_time_ms,
        );

        finalize_block::<_, MockCoreRPCLike>(&app, request)
    }

    /// Runs a block through `finalize_block` on `app`, committing it into `platform` for real.
    fn finalize_real_block<'a>(
        platform: &'a TempPlatform<MockCoreRPCLike>,
        app: &FullAbciApplication<'a, MockCoreRPCLike>,
        height: u64,
        block_time_ms: u64,
        previous_block_time_ms: Option<u64>,
    ) -> Result<proto::ResponseFinalizeBlock, Error> {
        app.start_transaction();

        let platform_state = (**platform.state.load()).clone();
        let quorum_hash: [u8; 32] = platform_state.current_validator_set_quorum_hash().into();

        app.block_execution_context
            .write()
            .unwrap()
            .replace(block_execution_context(
                platform_state,
                height,
                block_time_ms,
                previous_block_time_ms,
            ));

        let request = finalize_block_request(
            platform.config.abci.chain_id.clone(),
            quorum_hash,
            height,
            block_time_ms,
        );

        finalize_block::<_, MockCoreRPCLike>(app, request)
    }

    /// The data contract block cache is promoted to the global cache by this handler, after
    /// the commit. A contract the block rewrote must replace the committed copy a concurrent
    /// reader left in the global cache, and the block's record of rewrites must be reset.
    #[test]
    fn finalize_block_promotes_the_data_contract_block_cache_after_the_commit() {
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use drive::drive::contract::DataContractFetchInfo;

        let mut config = PlatformConfig::default_testnet();
        config.testing_configs.block_commit_signature_verification = false;
        let platform: TempPlatform<MockCoreRPCLike> = TestPlatformBuilder::new()
            .with_config(config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let app = FullAbciApplication::new(&platform.platform);
        let cache = &platform.drive.cache.data_contracts;

        // What a concurrent committed-state reader left behind, and what the block wrote.
        let committed = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        let contract_id = committed.contract.id().to_buffer();
        let mut rewritten = committed.clone();
        rewritten
            .contract
            .set_version(committed.contract.version() + 1);
        cache.insert_committed(Arc::new(committed.clone()), cache.committed_generation());
        cache.mark_modified_in_block(contract_id);
        cache.insert_block(Arc::new(rewritten.clone()));
        let generation_before = cache.committed_generation();

        finalize_real_block(&platform, &app, 1, 1_700_000_000_000, None)
            .expect("the block must finalize");

        assert_eq!(platform.state.load().last_committed_block_height(), 1);
        assert_eq!(
            cache
                .get(contract_id, false)
                .expect("the global cache must hold the promoted contract")
                .contract
                .version(),
            rewritten.contract.version(),
            "the block's definition must replace the committed copy once the block is committed"
        );
        assert!(
            !cache.is_modified_in_block(contract_id),
            "the record of the block's rewrites must be reset by the promotion"
        );
        assert_ne!(
            generation_before,
            cache.committed_generation(),
            "committed-state reads that began before the commit must no longer be able to publish"
        );
    }

    /// Records the events emitted while `capture` runs, to assert a failure is observable.
    #[derive(Clone, Default)]
    struct CapturedLogs(Arc<Mutex<Vec<(tracing::Level, String)>>>);

    impl CapturedLogs {
        fn capture<T>(&self, f: impl FnOnce() -> T) -> T {
            let subscriber = tracing_subscriber::registry().with(self.clone());
            tracing::subscriber::with_default(subscriber, f)
        }

        fn events(&self) -> Vec<(tracing::Level, String)> {
            self.0.lock().unwrap().clone()
        }

        fn has(&self, level: tracing::Level, needle: &str) -> bool {
            self.events()
                .iter()
                .any(|(event_level, message)| *event_level == level && message.contains(needle))
        }
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CapturedLogs {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            struct Message(String);

            impl tracing::field::Visit for Message {
                fn record_debug(
                    &mut self,
                    field: &tracing::field::Field,
                    value: &dyn std::fmt::Debug,
                ) {
                    if field.name() == "message" {
                        self.0 = format!("{value:?}");
                    }
                }
            }

            let mut message = Message(String::new());
            event.record(&mut message);
            self.0
                .lock()
                .unwrap()
                .push((*event.metadata().level(), message.0));
        }
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

    /// The block's saved platform state sat in the transaction that failed to
    /// commit, so the published state cache is ahead of the full record on disk.
    /// The handler must leave it dirty, or the next historical block writes only
    /// the small record on top of a stale full one.
    #[test]
    fn finalize_block_leaves_the_state_dirty_after_a_tolerated_commit_conflict() {
        let platform: TempPlatform<MockCoreRPCLike> = TestPlatformBuilder::new()
            .with_config(mainnet_evo1_config())
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        finalize_block_with_failing_commit_on(&platform, 32326, busy_commit_error())
            .expect("the incident's commit conflict is tolerated");

        let state = platform.state.load();
        assert!(
            state.heavy_fields_dirty,
            "a block whose commit failed must not leave the state cache clean"
        );
        assert!(
            state.masternode_changes.rewrite_all && state.validator_set_changes.rewrite_all,
            "its entries were in the failed transaction too"
        );
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

    fn checkpointing_config() -> PlatformConfig {
        let mut config = PlatformConfig::default_testnet();
        config.testing_configs.block_commit_signature_verification = false;
        config.testing_configs.disable_checkpoints = false;
        config
    }

    fn checkpointing_platform() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_config(checkpointing_config())
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state()
    }

    fn inject_checkpoint_faults(
        platform: &TempPlatform<MockCoreRPCLike>,
        faults: impl IntoIterator<Item = CheckpointStep>,
    ) {
        platform
            .config
            .testing_configs
            .checkpoint_faults
            .lock()
            .unwrap()
            .extend(faults);
    }

    fn registered_checkpoint_heights(platform: &TempPlatform<MockCoreRPCLike>) -> Vec<u64> {
        platform.drive.checkpoints.load().keys().copied().collect()
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before the unix epoch")
            .as_millis() as u64
    }

    /// Block times for three blocks: two inside the current checkpoint interval and
    /// one at the same offset into the next. They sit a minute past the interval
    /// start, so neither the clock advancing during the test nor the boundary
    /// itself can move a block into the neighbouring interval, and none of them is
    /// old enough to count as history.
    fn block_times_around_a_checkpoint_interval() -> [u64; 3] {
        let interval_ms = PlatformVersion::latest()
            .drive_abci
            .checkpoints
            .frequency_seconds as u64
            * 1000;
        let now = now_ms();
        let first = now - now % interval_ms + 60_000;
        [first, first + 1_000, first + interval_ms]
    }

    const ALL_CHECKPOINT_STEPS: [CheckpointStep; 4] = [
        CheckpointStep::CreateDirectory,
        CheckpointStep::CreateCheckpoint,
        CheckpointStep::WriteState,
        CheckpointStep::OpenCheckpoint,
    ];

    /// The checkpoint runs after the commit, so a failing step must not fail the
    /// block. A transient failure is retried once at the same height, which keeps
    /// this node's checkpoint heights aligned with the rest of the network, and the
    /// retry's checkpoint spends the interval like any other.
    #[test]
    fn finalize_block_retries_a_failed_checkpoint_once_at_the_same_height() {
        for step in ALL_CHECKPOINT_STEPS {
            let platform = checkpointing_platform();
            let app = FullAbciApplication::new(&platform.platform);
            let checkpoints_path = platform.config.db_path.join("checkpoints");
            let [first_time, second_time, _] = block_times_around_a_checkpoint_interval();

            inject_checkpoint_faults(&platform, [step]);

            let logs = CapturedLogs::default();
            let response = logs
                .capture(|| finalize_real_block(&platform, &app, 1, first_time, None))
                .unwrap_or_else(|error| {
                    panic!("{step:?}: the block is committed, so it must succeed: {error:?}")
                });
            assert_eq!(response.retain_height, 0);

            // The commit stands and the committed-height guard follows it
            assert_eq!(platform.state.load().last_committed_block_height(), 1);
            assert_eq!(
                platform
                    .committed_block_height_guard
                    .load(Ordering::Relaxed),
                1,
                "{step:?}"
            );

            // The retry created the checkpoint at this block's height
            assert_eq!(
                registered_checkpoint_heights(&platform),
                vec![1],
                "{step:?}"
            );
            assert_eq!(
                platform
                    .checkpoint_platform_states
                    .load()
                    .get(&1)
                    .map(|state| state.last_committed_block_height()),
                Some(1),
                "{step:?}"
            );
            assert!(
                checkpoints_path
                    .join("1")
                    .join("platform_state.bin")
                    .is_file(),
                "{step:?}"
            );

            // The first failure is observable, and no checkpoint was skipped
            assert!(
                logs.has(tracing::Level::WARN, "retrying once"),
                "{step:?}: {:?}",
                logs.events()
            );
            assert!(
                !logs.has(tracing::Level::ERROR, "checkpoint"),
                "{step:?}: {:?}",
                logs.events()
            );

            // The next block in the same interval does not checkpoint again
            finalize_real_block(&platform, &app, 2, second_time, Some(first_time))
                .unwrap_or_else(|error| panic!("{step:?}: second block: {error:?}"));
            assert_eq!(
                registered_checkpoint_heights(&platform),
                vec![1],
                "{step:?}"
            );
            assert!(!checkpoints_path.join("2").exists(), "{step:?}");
        }
    }

    /// When the immediate retry fails too, the checkpoint is skipped rather than
    /// retried at a later block: the block still succeeds, the failed attempts
    /// leave nothing behind, the failure is logged, the rest of the interval makes
    /// no attempt, and the next interval boundary checkpoints again.
    #[test]
    fn finalize_block_skips_a_checkpoint_that_fails_twice() {
        for step in ALL_CHECKPOINT_STEPS {
            let platform = checkpointing_platform();
            let app = FullAbciApplication::new(&platform.platform);
            let checkpoints_path = platform.config.db_path.join("checkpoints");
            let [first_time, second_time, next_interval_time] =
                block_times_around_a_checkpoint_interval();

            inject_checkpoint_faults(&platform, [step, step]);

            let logs = CapturedLogs::default();
            let response = logs
                .capture(|| finalize_real_block(&platform, &app, 1, first_time, None))
                .unwrap_or_else(|error| {
                    panic!("{step:?}: the block is committed, so it must succeed: {error:?}")
                });
            assert_eq!(response.retain_height, 0);

            // The commit stands and the committed-height guard follows it
            assert_eq!(platform.state.load().last_committed_block_height(), 1);
            assert_eq!(
                platform
                    .committed_block_height_guard
                    .load(Ordering::Relaxed),
                1,
                "{step:?}"
            );

            // Both attempts ran, registered nothing, and left nothing on disk
            assert!(
                platform
                    .config
                    .testing_configs
                    .checkpoint_faults
                    .lock()
                    .unwrap()
                    .is_empty(),
                "{step:?}: both injected faults must have been consumed"
            );
            assert!(
                registered_checkpoint_heights(&platform).is_empty(),
                "{step:?}"
            );
            assert!(
                platform.checkpoint_platform_states.load().is_empty(),
                "{step:?}"
            );
            assert!(!checkpoints_path.join("1").exists(), "{step:?}");

            // The skip is observable
            assert!(
                logs.has(tracing::Level::ERROR, "this checkpoint is skipped"),
                "{step:?}: {:?}",
                logs.events()
            );

            // The rest of the interval makes no attempt at all
            let logs = CapturedLogs::default();
            logs.capture(|| finalize_real_block(&platform, &app, 2, second_time, Some(first_time)))
                .unwrap_or_else(|error| panic!("{step:?}: second block: {error:?}"));
            assert!(
                registered_checkpoint_heights(&platform).is_empty(),
                "{step:?}"
            );
            assert!(!checkpoints_path.join("2").exists(), "{step:?}");
            assert!(
                !logs
                    .events()
                    .iter()
                    .any(|(_, message)| message.contains("checkpoint")),
                "{step:?}: no attempt expected in the same interval: {:?}",
                logs.events()
            );

            // The next interval boundary checkpoints again
            finalize_real_block(&platform, &app, 3, next_interval_time, Some(second_time))
                .unwrap_or_else(|error| panic!("{step:?}: third block: {error:?}"));
            assert_eq!(
                platform
                    .committed_block_height_guard
                    .load(Ordering::Relaxed),
                3,
                "{step:?}"
            );
            assert_eq!(
                registered_checkpoint_heights(&platform),
                vec![3],
                "{step:?}"
            );
            assert_eq!(
                platform
                    .checkpoint_platform_states
                    .load()
                    .get(&3)
                    .map(|state| state.last_committed_block_height()),
                Some(3),
                "{step:?}"
            );
            assert!(
                checkpoints_path
                    .join("3")
                    .join("platform_state.bin")
                    .is_file(),
                "{step:?}"
            );
        }
    }

    /// The attempt record survives a restart: a node restarted inside the interval
    /// after a skipped checkpoint still makes no attempt until the next interval
    /// boundary, so its checkpoint heights stay the network's.
    #[test]
    fn finalize_block_keeps_skipping_a_failed_checkpoint_after_a_restart() {
        let platform = checkpointing_platform();
        let app = FullAbciApplication::new(&platform.platform);
        let [first_time, second_time, next_interval_time] =
            block_times_around_a_checkpoint_interval();

        inject_checkpoint_faults(
            &platform,
            [CheckpointStep::WriteState, CheckpointStep::WriteState],
        );
        finalize_real_block(&platform, &app, 1, first_time, None).expect("first block");
        assert!(registered_checkpoint_heights(&platform).is_empty());

        // Restart: release the database and reopen it from the same directory
        drop(app);
        let TempPlatform { platform, tempdir } = platform;
        drop(platform);
        let platform = TempPlatform::open_with_tempdir(tempdir, checkpointing_config());
        let app = FullAbciApplication::new(&platform.platform);

        assert_eq!(platform.state.load().last_committed_block_height(), 1);
        assert_eq!(
            platform
                .last_checkpoint_attempt_block_time_ms
                .load(Ordering::Relaxed),
            first_time,
            "the attempt record is loaded from disk"
        );

        // The rest of the interval still makes no attempt
        let logs = CapturedLogs::default();
        logs.capture(|| finalize_real_block(&platform, &app, 2, second_time, Some(first_time)))
            .expect("second block after the restart");
        assert!(registered_checkpoint_heights(&platform).is_empty());
        assert!(
            !logs
                .events()
                .iter()
                .any(|(_, message)| message.contains("checkpoint")),
            "no attempt expected in the same interval: {:?}",
            logs.events()
        );

        // The next interval boundary checkpoints again
        finalize_real_block(&platform, &app, 3, next_interval_time, Some(second_time))
            .expect("third block after the restart");
        assert_eq!(registered_checkpoint_heights(&platform), vec![3]);
    }
}
