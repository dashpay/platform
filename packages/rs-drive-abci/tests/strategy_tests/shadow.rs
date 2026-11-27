//! Shadow mode on chains of the strategy tests: the two engines agree block by block, and a
//! divergence of each kind that is compared is caught.

#[cfg(test)]
mod tests {
    use crate::addresses_with_balance::AddressesWithBalance;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::tests::json_document::json_document_to_created_contract;
    use drive::config::DriveConfig;
    use drive::drive::{Drive, RootTree};
    use drive::grovedb::Element;
    use drive_abci::abci::app::shadow::testing::{
        no_shadow_for_the_chains_of_this_thread, report_directory,
        shadow_for_the_chains_of_this_thread, shadow_on, Fault, ForATest,
    };
    use drive_abci::abci::app::shadow::{
        compare_stores, Counts, HeightRange, Shadow, ShadowEngine, ShadowError, ShadowPlatform,
        SkippedHeight, STATUS_FILE,
    };
    use drive_abci::abci::app::FullAbciApplication;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, OnDivergence, PlatformConfig,
        PlatformTestConfig, ValidatorSetConfig,
    };
    use drive_abci::platform_types::platform::Platform;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::rpc::core::MockCoreRPCLike;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use prost::Message;
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::{Path, PathBuf};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::{RequestFinalizeBlock, ResponseProcessProposal};

    /// A future that gives the panic of the future inside it as its output.
    struct Caught<F>(Pin<Box<F>>);

    impl<F: Future> Future for Caught<F> {
        type Output = Result<F::Output, String>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            match catch_unwind(AssertUnwindSafe(|| self.0.as_mut().poll(context))) {
                Ok(Poll::Pending) => Poll::Pending,
                Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
                Err(panic) => Poll::Ready(Err(panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
                    .unwrap_or_default())),
            }
        }
    }

    /// The configuration of the chains of these tests. A node stores its platform state
    /// with every block, and so do they, which most strategy tests do not: what a start
    /// reads of a database is then there to be read, and is in what the two engines are
    /// compared by.
    fn config() -> PlatformConfig {
        PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..PlatformTestConfig::default_minimal_verifications()
            },
            ..Default::default()
        }
    }

    fn network(strategy: Strategy) -> NetworkStrategy {
        NetworkStrategy {
            strategy,
            total_hpmns: 20,
            extra_normal_mns: 0,
            validator_quorum_count: 4,
            chain_lock_quorum_count: 4,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        }
    }

    /// Blocks with nothing in them.
    fn nothing() -> NetworkStrategy {
        network(Strategy {
            start_contracts: vec![],
            operations: vec![],
            start_identities: StartIdentities::default(),
            start_addresses: StartAddresses::default(),
            identity_inserts: IdentityInsertInfo::default(),
            identity_contract_nonce_gaps: None,
            signer: None,
        })
    }

    /// A new identity a block: what a chain goes on with after its first blocks.
    fn identities() -> NetworkStrategy {
        network(Strategy {
            start_contracts: vec![],
            operations: vec![],
            start_identities: StartIdentities::default(),
            start_addresses: StartAddresses::default(),
            identity_inserts: IdentityInsertInfo {
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
                ..Default::default()
            },
            identity_contract_nonce_gaps: None,
            signer: None,
        })
    }

    /// A new identity a block, and a document made and one deleted.
    fn identities_and_documents() -> NetworkStrategy {
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            PlatformVersion::latest(),
        )
        .expect("a contract");
        let contract = created_contract.data_contract().clone();
        let document = |action| {
            let op = DocumentOp {
                contract: contract.clone(),
                action,
                document_type: contract
                    .document_type_for_name("contactRequest")
                    .expect("a document type")
                    .to_owned_document_type(),
            };
            Operation {
                op_type: OperationType::Document(op),
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
            }
        };
        network(Strategy {
            start_contracts: vec![(created_contract, None)],
            operations: vec![
                document(DocumentAction::DocumentActionInsertRandom(
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                )),
                document(DocumentAction::DocumentActionDelete),
            ],
            start_identities: StartIdentities::default(),
            start_addresses: StartAddresses::default(),
            identity_inserts: IdentityInsertInfo {
                frequency: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
                },
                ..Default::default()
            },
            identity_contract_nonce_gaps: None,
            signer: None,
        })
    }

    /// The parameters that go on from where `outcome` ended, for `blocks` more.
    fn from_the_end_of(
        outcome: &ChainExecutionOutcome<'_>,
        blocks: u64,
    ) -> ChainExecutionParameters {
        let state = outcome.abci_app.platform.state.load();
        ChainExecutionParameters {
            block_start: state.last_committed_block_height() + 1,
            core_height_start: 1,
            block_count: blocks,
            proposers: outcome.proposers.clone(),
            validator_quorums: outcome.validator_quorums.clone(),
            current_validator_quorum_hash: outcome.current_validator_quorum_hash,
            instant_lock_quorums: outcome.instant_lock_quorums.clone(),
            current_proposer_versions: Some(outcome.current_proposer_versions.clone()),
            current_identity_nonce_counter: outcome.identity_nonce_counter.clone(),
            current_identity_contract_nonce_counter: outcome
                .identity_contract_nonce_counter
                .clone(),
            current_votes: BTreeMap::default(),
            start_time_ms: 1681094380000,
            current_time_ms: outcome.end_time_ms,
            current_identities: outcome.identities.clone(),
            current_addresses_with_balance: AddressesWithBalance::default(),
        }
    }

    /// `blocks` more blocks on the chain of `outcome`.
    async fn more<'a>(
        outcome: ChainExecutionOutcome<'a>,
        blocks: u64,
        strategy: &NetworkStrategy,
    ) -> ChainExecutionOutcome<'a> {
        let parameters = from_the_end_of(&outcome, blocks);
        continue_chain_for_strategy(
            outcome.abci_app,
            parameters,
            strategy.clone(),
            config(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await
    }

    /// A chain of `blocks` blocks of `strategy`, with the shadow `asked`.
    async fn chain<'a>(
        platform: &'a mut Platform<MockCoreRPCLike>,
        blocks: u64,
        strategy: &NetworkStrategy,
        asked: ForATest,
    ) -> ChainExecutionOutcome<'a> {
        shadow_for_the_chains_of_this_thread(Some(asked));
        let outcome = run_chain_for_strategy(
            platform,
            blocks,
            strategy.clone(),
            config(),
            15,
            &mut None,
            &mut None,
        )
        .await;
        shadow_for_the_chains_of_this_thread(None);
        outcome
    }

    fn shadow_of<'b, 'a>(
        outcome: &'b ChainExecutionOutcome<'a>,
    ) -> &'b Shadow<'a, MockCoreRPCLike> {
        outcome.abci_app.shadow.as_ref().expect("a shadow")
    }

    /// The Platform of the shadow of a chain, whose database is open.
    fn shadow_platform_of<'a>(
        outcome: &ChainExecutionOutcome<'a>,
    ) -> &'a ShadowPlatform<'a, MockCoreRPCLike> {
        shadow_of(outcome).platform().expect("an open shadow")
    }

    fn height_of<C>(platform: &Platform<C>) -> u64 {
        platform.state.load().last_committed_block_height()
    }

    fn app_hash_of<C>(platform: &Platform<C>) -> [u8; 32] {
        platform
            .state
            .load()
            .last_committed_block_app_hash()
            .expect("an app hash")
    }

    fn root_of<C>(platform: &Platform<C>) -> [u8; 32] {
        platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .unwrap()
            .expect("a root")
    }

    /// Gives `platform` the state its database holds, as a start reads it, in place of
    /// the one it has in memory.
    fn read_from_storage<C>(platform: &Platform<C>) {
        let platform_version = platform
            .state
            .load()
            .current_platform_version()
            .expect("a version");
        let state = Platform::<C>::fetch_platform_state(&platform.drive, None, platform_version)
            .expect("the state is read")
            .expect("a state");
        platform.state.store(Arc::new(state));
    }

    /// The counts as the report's file has them.
    fn counts_in(report: &Path) -> Counts {
        serde_json::from_slice(&std::fs::read(report.join(STATUS_FILE)).expect("the counts"))
            .expect("counts")
    }

    /// The lines of the blocks the report holds.
    fn lines_in(report: &Path) -> Vec<serde_json::Value> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(report)
            .expect("the report")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("blocks-"))
            })
            .collect();
        files.sort();
        files
            .iter()
            .flat_map(|file| {
                std::fs::read_to_string(file)
                    .expect("the lines")
                    .lines()
                    .map(|line| serde_json::from_str(line).expect("a line"))
                    .collect::<Vec<serde_json::Value>>()
            })
            .collect()
    }

    /// The directory that keeps the divergence at `height`, and its differences.
    fn kept_at(report: &Path, height: u64) -> (PathBuf, String) {
        let directory = report.join("divergences").join(height.to_string());
        let differs = std::fs::read_to_string(directory.join("differs.txt")).expect("differences");
        (directory, differs)
    }

    /// The names of what a directory holds, in order.
    fn names_in(directory: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(directory)
            .expect("the directory")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Every one of `blocks` blocks was compared and none differs.
    fn no_divergence(counts: &Counts, blocks: u64) {
        assert_eq!(counts.blocks_compared, blocks);
        assert_eq!(counts.blocks_in_all(), blocks);
        assert_eq!(counts.blocks_diverged, 0);
        assert_eq!(counts.first_compared_height, Some(1));
        assert_eq!(counts.first_counted_height, Some(1));
        assert!(counts.rewinds.is_empty());
        assert!(counts.blocks_uncounted.is_empty());
        assert_eq!(counts.height, blocks);
        assert_eq!(counts.shadow_height, blocks);
        assert!(counts.divergences.is_empty(), "{:?}", counts.divergences);
        assert!(counts.divergence_heights.is_empty());
        assert!(counts.blocks_not_compared.is_empty());
        assert!(counts.heights_skipped.is_empty());
        assert_eq!(counts.blocks_without_shadow, 0);
        assert_eq!(counts.shadow_stopped, None);
    }

    /// The app hash a node with no shadow has after `blocks` blocks of `strategy` and
    /// `after` more of `then`: what a node in shadow mode has to commit too.
    async fn app_hash_alone(
        blocks: u64,
        strategy: &NetworkStrategy,
        after: u64,
        then: &NetworkStrategy,
    ) -> [u8; 32] {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        no_shadow_for_the_chains_of_this_thread();
        let outcome = run_chain_for_strategy(
            &mut platform,
            blocks,
            strategy.clone(),
            config(),
            15,
            &mut None,
            &mut None,
        )
        .await;
        shadow_for_the_chains_of_this_thread(None);
        assert!(outcome.abci_app.shadow.is_none());
        let outcome = match after {
            0 => outcome,
            after => more(outcome, after, then).await,
        };
        assert_eq!(height_of(outcome.abci_app.platform), blocks + after);
        app_hash_of(outcome.abci_app.platform)
    }

    /// Twenty blocks of identities and documents: every block is compared and none
    /// differs, the lines say what the blocks held, the stored records of the two
    /// databases are equal, and the primary commits what a node with no shadow commits.
    async fn the_two_databases_agree(engine: ShadowEngine) {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let outcome = chain(&mut platform, 20, &strategy, ForATest::on(engine)).await;
        let shadow = shadow_of(&outcome);
        let shadow_platform = shadow_platform_of(&outcome);
        let primary = outcome.abci_app.platform;

        let counts = shadow.report().counts();
        no_divergence(&counts, 20);
        assert_eq!(counts, counts_in(&report));
        match engine {
            ShadowEngine::Other => assert_ne!(counts.primary_engine, counts.shadow_engine),
            ShadowEngine::Same => assert_eq!(counts.primary_engine, counts.shadow_engine),
        }
        assert_eq!(height_of(shadow_platform), 20);
        assert_eq!(app_hash_of(primary), app_hash_of(shadow_platform));
        assert_eq!(root_of(primary), root_of(shadow_platform));
        assert_eq!(
            app_hash_of(primary),
            app_hash_alone(20, &strategy, 0, &strategy).await
        );

        // The lines: one a block, in order, with what the chain held. The strategy makes
        // transactions and fee calculations, and the lines have to show them, or the
        // comparison compared nothing: a block has a fee calculation at least for every
        // transaction in it that paid.
        let lines = lines_in(&report);
        assert_eq!(lines.len(), 20);
        let number = |line: &serde_json::Value, field: &str| line[field].as_u64().expect(field);
        let mut paid_in_all = 0;
        for (index, line) in lines.iter().enumerate() {
            assert_eq!(number(line, "height"), index as u64 + 1);
            assert_eq!(line["root_is_app_hash"], true);
            assert_eq!(line["grove_root"], line["app_hash"]);
            let paid = line["tx"]
                .as_array()
                .expect("tx")
                .iter()
                .filter(|tx| tx[0] == 0 && tx[1].as_i64().expect("a fee") > 0)
                .count() as u64;
            assert!(number(line, "fee_calcs") >= paid, "{line}");
            assert!(paid == 0 || number(line, "fee_ops") > paid, "{line}");
            assert_eq!(
                line["fee_ops_sha256"].as_str().map(str::len),
                Some(64),
                "{line}"
            );
            paid_in_all += paid;
        }
        assert!(paid_in_all >= 20, "{paid_in_all} paid transactions");

        // The stored records.
        let platform_version = primary
            .state
            .load()
            .current_platform_version()
            .expect("a version");
        let stores = compare_stores(
            &primary.drive.grove,
            &shadow_platform.drive.grove,
            20,
            platform_version,
        );
        assert_eq!(stores.error, None);
        assert_eq!(stores.differing, 0, "{:?}", stores.differences);
        assert!(
            stores.subtrees_compared > 50,
            "{}",
            stores.subtrees_compared
        );
        assert!(stores.root_elements_compared > 5);
        assert!(stores.bytes > 10_000);
    }

    #[tokio::test]
    async fn the_two_engines_agree_block_by_block_and_hold_the_same_records() {
        the_two_databases_agree(ShadowEngine::Other).await;
    }

    #[tokio::test]
    async fn two_platforms_on_one_engine_agree_block_by_block() {
        the_two_databases_agree(ShadowEngine::Same).await;
    }

    /// A shadow that is configured to read otherwise than the primary is charged otherwise
    /// by its engine for the same block: the costs of an operation differ, then the fee of
    /// the transaction, then the app hash. All of it is found before either database
    /// commits the block, with the first operation whose cost differs and both costs, and
    /// both answers are kept as they were given.
    #[tokio::test]
    async fn an_engine_that_charges_otherwise_is_found_at_the_first_operation() {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest {
            alter: Some(|config| config.drive.has_raw_enabled = !config.drive.has_raw_enabled),
            ..ForATest::on(ShadowEngine::Same)
        };
        let outcome = chain(&mut platform, 10, &strategy, asked).await;
        let shadow = shadow_of(&outcome);
        let shadow_platform = shadow_platform_of(&outcome);
        let counts = shadow.report().counts();

        assert_eq!(
            counts.divergences.get("state"),
            Some(&1),
            "{:?}",
            counts.divergences
        );
        assert_eq!(counts.divergences_in_all(), 1);
        let height = counts.divergence_heights[0];
        let stopped = counts.shadow_stopped.clone().expect("the shadow stopped");
        assert_eq!(stopped.height, height);
        // Before either commits: the shadow stays below the block, the primary goes on.
        assert_eq!(height_of(shadow_platform), height - 1);
        assert_eq!(height_of(outcome.abci_app.platform), 10);
        assert_eq!(counts.blocks_compared, height - 1);
        assert_eq!(counts.blocks_diverged, 1);
        assert_eq!(counts.blocks_without_shadow, 10 - height);
        assert_eq!(counts.blocks_in_all(), 10);

        let (kept, differs) = kept_at(&report, height);
        for field in ["app_hash", "tx", "proposal_sha256", "fee_ops_sha256"] {
            assert!(
                differs.contains(&format!("DIFFERS height={height} field={field} this=")),
                "{field} in {differs}"
            );
        }
        let first = differs
            .lines()
            .find(|line| line.starts_with("fee calculation "))
            .expect("the first fee calculation that differs");
        assert!(first.contains(" operation "), "{first}");
        assert!(
            first.contains("this=cost seeks=") && first.contains(" other=cost seeks="),
            "{first}"
        );
        let text = |name: &str| std::fs::read_to_string(kept.join(name)).expect(name);
        assert_ne!(text("primary-fees.txt"), text("shadow-fees.txt"));
        let line = |name: &str| -> BTreeMap<String, serde_json::Value> {
            serde_json::from_str(&text(name)).expect("a line")
        };
        assert_ne!(
            line("primary.json")["app_hash"],
            line("shadow.json")["app_hash"]
        );
        // The two answers as they were given, which the lines are made of.
        let answer = |name: &str| {
            ResponseProcessProposal::decode(std::fs::read(kept.join(name)).expect(name).as_slice())
                .expect("an answer")
        };
        let (of_primary, of_shadow) = (
            answer("primary-process-proposal.bin"),
            answer("shadow-process-proposal.bin"),
        );
        assert_ne!(of_primary.app_hash, of_shadow.app_hash);
        assert_eq!(
            line("primary.json")["app_hash"].as_str(),
            Some(hex::encode(&of_primary.app_hash).as_str())
        );
        assert_eq!(
            line("shadow.json")["app_hash"].as_str(),
            Some(hex::encode(&of_shadow.app_hash).as_str())
        );

        // The block, and both databases as they were before it.
        let request = RequestFinalizeBlock::decode(
            std::fs::read(kept.join("finalize-block-request.bin"))
                .expect("the request")
                .as_slice(),
        )
        .expect("a request");
        assert_eq!(request.height as u64, height);
        let platform_version = PlatformVersion::latest();
        let kept_root = |name: &str, config: &DriveConfig| {
            Drive::open_checkpoint_at(kept.join(name), config)
                .expect("a kept store")
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("a root")
        };
        let kept_primary = kept_root("primary-store", &outcome.abci_app.platform.drive.config);
        let kept_shadow = kept_root("shadow-store", &shadow_platform.drive.config);
        assert_eq!(kept_primary, kept_shadow);
        assert_eq!(kept_shadow, root_of(shadow_platform));

        // The lines: the blocks before it, and the primary's line of it, which has what
        // the primary executed and nothing of a commit.
        let lines = lines_in(&report);
        assert_eq!(lines.len() as u64, height);
        let last = lines.last().expect("the line of the block");
        assert_eq!(last["height"].as_u64(), Some(height));
        assert_eq!(last["app_hash"], line("primary.json")["app_hash"]);
        assert!(last
            .get("grove_root")
            .is_none_or(serde_json::Value::is_null));
        if let Some(before) = lines.iter().rev().nth(1) {
            assert_eq!(before["height"].as_u64(), Some(height - 1));
            assert_eq!(
                before["grove_root"].as_str(),
                Some(hex::encode(kept_primary).as_str())
            );
        }
    }

    /// The same divergence where the configuration ends the node: the request is answered
    /// with an error and neither database commits the block.
    #[tokio::test]
    async fn a_node_that_is_to_stop_at_a_divergence_commits_the_block_on_neither_database() {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest {
            alter: Some(|config| config.drive.has_raw_enabled = !config.drive.has_raw_enabled),
            on_divergence: OnDivergence::Stop,
            ..ForATest::on(ShadowEngine::Same)
        };
        shadow_for_the_chains_of_this_thread(Some(asked));
        let ended = Caught(Box::pin(run_chain_for_strategy(
            &mut platform,
            10,
            strategy.clone(),
            config(),
            15,
            &mut None,
            &mut None,
        )))
        .await;
        shadow_for_the_chains_of_this_thread(None);
        let said = ended
            .err()
            .unwrap_or_else(|| panic!("the chain ends at the divergence"));
        assert!(
            said.contains("the two storage engines diverge at height"),
            "{said}"
        );
        assert!(said.contains("SHADOW_ON_DIVERGENCE=stop"), "{said}");

        let counts = counts_in(&report);
        assert_eq!(counts.divergences.get("state"), Some(&1));
        let height = counts.divergence_heights[0];
        assert!(
            said.contains(&format!("diverge at height {height} ")),
            "{said}"
        );
        assert_eq!(counts.height, height - 1);
        assert_eq!(counts.shadow_height, height - 1);
        assert_eq!(counts.blocks_compared, height - 1);
        assert_eq!(counts.blocks_in_all(), height - 1);
        assert_eq!(counts.shadow_stopped, None);
        assert_eq!(height_of(&platform), height - 1);
        assert!(kept_at(&report, height).1.contains("field=app_hash"));
    }

    /// What a fault gave where it was planted in the sixth block of a chain of eight.
    struct Planted {
        /// The counts at the end.
        counts: Counts,
        /// The differences that were kept.
        differs: String,
        /// The names of what was kept with them.
        kept: Vec<String>,
        /// The heights of the primary and of the shadow at the end.
        heights: (u64, u64),
    }

    impl Planted {
        fn kept(&self, names: &[&str]) {
            for name in names {
                assert!(
                    self.kept.iter().any(|kept| kept == name),
                    "{name} is not in {:?}",
                    self.kept
                );
            }
        }

        fn not_kept(&self, names: &[&str]) {
            for name in names {
                assert!(
                    !self.kept.iter().any(|kept| kept == name),
                    "{name} is in {:?}",
                    self.kept
                );
            }
        }

        fn divergences(&self, kind: &str) {
            assert_eq!(
                self.counts.divergences,
                BTreeMap::from([(kind.to_string(), 1)])
            );
            assert_eq!(self.counts.divergence_heights, vec![6]);
            assert_eq!(self.counts.blocks_diverged, 1);
            assert_eq!(self.counts.blocks_in_all(), 8);
            assert_eq!(self.counts.height, 8);
            assert!(self.counts.blocks_not_compared.is_empty());
            assert!(self.counts.heights_skipped.is_empty());
        }

        /// The shadow stopped: `stops_at` is the first block it did not apply.
        fn stopped_at(&self, stops_at: u64) {
            let stopped = self.counts.shadow_stopped.as_ref().expect("stopped");
            assert_eq!(stopped.height, stops_at);
            assert_eq!(self.heights, (8, stops_at - 1));
            assert_eq!(self.counts.shadow_height, stops_at - 1);
            assert_eq!(self.counts.blocks_compared, 5);
            assert_eq!(self.counts.blocks_without_shadow, 2);
        }
    }

    /// What is kept of a block that both executed.
    const OF_THE_EXECUTIONS: [&str; 7] = [
        "finalize-block-request.bin",
        "primary-process-proposal.bin",
        "shadow-process-proposal.bin",
        "primary-fees.txt",
        "shadow-fees.txt",
        "primary-store",
        "shadow-store",
    ];
    const THE_LINES: [&str; 2] = ["primary.json", "shadow.json"];
    const OF_THE_COMMITS: [&str; 2] = ["primary-finalize-block.bin", "shadow-finalize-block.bin"];

    async fn planted(fault: Fault) -> Planted {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let outcome = chain(
            &mut platform,
            5,
            &strategy,
            ForATest::on(ShadowEngine::Other),
        )
        .await;
        no_divergence(&shadow_of(&outcome).report().counts(), 5);
        shadow_of(&outcome).plant(fault);
        let outcome = more(outcome, 3, &identities()).await;
        let shadow = shadow_of(&outcome);
        let counts = shadow.report().counts();
        assert_eq!(counts, counts_in(&report));
        let (kept, differs) = kept_at(&report, 6);
        Planted {
            counts,
            differs,
            kept: names_in(&kept),
            heights: (
                height_of(outcome.abci_app.platform),
                height_of(shadow_platform_of(&outcome)),
            ),
        }
    }

    /// A cost that differs alone, with every answer and hash equal, is counted and kept
    /// with both costs, and the shadow goes on.
    #[tokio::test]
    async fn a_cost_that_differs_alone_is_counted_and_the_shadow_goes_on() {
        let planted = planted(Fault::ShadowCost).await;
        planted.divergences("cost");
        assert_eq!(planted.counts.shadow_stopped, None);
        assert_eq!(planted.heights, (8, 8));
        assert_eq!(planted.counts.blocks_compared, 7);
        assert_eq!(planted.counts.blocks_without_shadow, 0);
        let differs = &planted.differs;
        assert!(
            differs.contains("DIFFERS height=6 field=fee_ops_sha256 this="),
            "{differs}"
        );
        assert!(!differs.contains("field=app_hash"), "{differs}");
        assert!(!differs.contains("field=tx"), "{differs}");
        assert!(differs.contains("fee calculation "), "{differs}");
        planted.kept(&OF_THE_EXECUTIONS);
        planted.kept(&THE_LINES);
        planted.not_kept(&OF_THE_COMMITS);
    }

    #[tokio::test]
    async fn a_fee_of_a_transaction_that_differs_stops_the_shadow_before_the_commit() {
        let planted = planted(Fault::ShadowTransactionFee).await;
        planted.divergences("state");
        planted.stopped_at(6);
        let differs = &planted.differs;
        assert!(
            differs.contains("DIFFERS height=6 field=tx this="),
            "{differs}"
        );
        assert!(
            differs.contains("DIFFERS height=6 field=proposal_sha256 this="),
            "{differs}"
        );
        assert!(!differs.contains("field=app_hash"), "{differs}");
        planted.kept(&OF_THE_EXECUTIONS);
        planted.kept(&THE_LINES);
        planted.not_kept(&OF_THE_COMMITS);
    }

    #[tokio::test]
    async fn a_code_of_a_transaction_that_differs_stops_the_shadow_before_the_commit() {
        let planted = planted(Fault::ShadowTransactionCode).await;
        planted.divergences("state");
        planted.stopped_at(6);
        let differs = &planted.differs;
        assert!(
            differs.contains("DIFFERS height=6 field=tx this=[[0,"),
            "{differs}"
        );
        assert!(differs.contains(" other=[[1,"), "{differs}");
        assert!(!differs.contains("field=app_hash"), "{differs}");
        planted.kept(&OF_THE_EXECUTIONS);
    }

    /// A difference in the answer to `ProcessProposal` that is in no app hash, code or
    /// fee is found by the hash of the whole answer.
    #[tokio::test]
    async fn the_rest_of_an_answer_that_differs_stops_the_shadow_before_the_commit() {
        let planted = planted(Fault::ShadowRestOfTheAnswer).await;
        planted.divergences("state");
        planted.stopped_at(6);
        let differs = &planted.differs;
        assert!(
            differs.starts_with("DIFFERS height=6 field=proposal_sha256 this="),
            "{differs}"
        );
        assert_eq!(differs.matches("DIFFERS ").count(), 1, "{differs}");
        planted.kept(&OF_THE_EXECUTIONS);
        planted.kept(&THE_LINES);
    }

    #[tokio::test]
    async fn an_error_of_the_shadow_is_a_divergence_and_the_primary_goes_on() {
        let planted = planted(Fault::ShadowFailsAtItsCommit).await;
        planted.divergences("shadow_error");
        planted.stopped_at(6);
        assert!(planted.counts.divergences_with_core_errors.is_empty());
        let differs = &planted.differs;
        assert!(!differs.contains("Dash Core answered"), "{differs}");
        assert!(
            differs.contains("DIFFERS height=6 field=shadow_error this=\"ok\" other="),
            "{differs}"
        );
        assert!(differs.contains("the commit of the block"), "{differs}");
        assert!(!differs.contains("the flush"), "{differs}");
        assert!(
            differs.contains("the shadow's database does not hold the block"),
            "{differs}"
        );
        // What both made of the block before the shadow failed is kept.
        planted.kept(&OF_THE_EXECUTIONS);
        planted.not_kept(&OF_THE_COMMITS);
    }

    /// The shadow fails after its commit, where its database is flushed: it holds the
    /// block, the primary commits it too, and the shadow applies none after it.
    #[tokio::test]
    async fn an_error_of_the_shadow_at_its_flush_leaves_it_with_the_block() {
        let planted = planted(Fault::ShadowFailsAtItsFlush).await;
        planted.divergences("shadow_error");
        planted.stopped_at(7);
        let differs = &planted.differs;
        assert!(
            differs.contains("the flush after its commit of the block"),
            "{differs}"
        );
        assert!(
            differs.contains("the shadow's database holds the block"),
            "{differs}"
        );
        planted.kept(&OF_THE_EXECUTIONS);
        // Its answer to `FinalizeBlock` was made, and is kept; the primary's is not made
        // where the divergence is kept.
        planted.kept(&["shadow-finalize-block.bin"]);
        planted.not_kept(&["primary-finalize-block.bin"]);
    }

    /// The shadow's handler fails after its commit: the shadow's database holds the
    /// block, which is read from it, and the shadow stops at the block after.
    #[tokio::test]
    async fn an_error_of_the_shadow_after_its_commit_leaves_it_with_the_block() {
        let planted = planted(Fault::ShadowFailsAfterItsCommit).await;
        planted.divergences("shadow_error");
        planted.stopped_at(7);
        let differs = &planted.differs;
        assert!(differs.contains("the commit of the block"), "{differs}");
        assert!(
            differs.contains("the shadow's database holds the block"),
            "{differs}"
        );
        planted.kept(&OF_THE_EXECUTIONS);
        planted.not_kept(&OF_THE_COMMITS);
    }

    /// Dash Core answers the shadow with an error in a block in which the shadow then
    /// fails: the failure is a divergence as any, the error of Core is kept with it, and
    /// the height is written down as one at which Core answered the shadow so.
    #[tokio::test]
    async fn an_error_of_core_in_the_block_is_kept_with_a_failure_of_the_shadow() {
        let planted = planted(Fault::CoreFailsTheShadow).await;
        planted.divergences("shadow_error");
        planted.stopped_at(6);
        assert_eq!(planted.counts.divergences_with_core_errors, vec![6]);
        let differs = &planted.differs;
        assert!(
            differs.contains(
                "Dash Core answered the shadow with an error before this was found: \
                 get_transaction: planted by a test"
            ),
            "{differs}"
        );
        planted.kept(&OF_THE_EXECUTIONS);
    }

    /// A root of the primary that is not read after the commits is no failure of the
    /// shadow and no difference: the block is counted as not compared, with the reason,
    /// and the shadow goes on.
    #[tokio::test]
    async fn a_root_of_the_primary_that_is_not_read_is_a_block_that_was_not_compared() {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let outcome = chain(
            &mut platform,
            5,
            &strategy,
            ForATest::on(ShadowEngine::Other),
        )
        .await;
        shadow_of(&outcome).plant(Fault::PrimaryRootIsNotRead);
        let outcome = more(outcome, 3, &identities()).await;
        let counts = shadow_of(&outcome).report().counts();
        assert!(counts.divergences.is_empty(), "{:?}", counts.divergences);
        assert_eq!(counts.shadow_stopped, None);
        assert_eq!(counts.blocks_not_compared.len(), 1);
        let not_compared = &counts.blocks_not_compared[0];
        assert_eq!((not_compared.height, not_compared.through), (6, 6));
        assert_eq!(
            not_compared.reason,
            "not compared: the roots after the commit, for the primary's was not read (planted \
             by a test); everything else was"
        );
        assert_eq!((counts.blocks_compared, counts.blocks_in_all()), (7, 8));
        assert_eq!(
            (
                height_of(outcome.abci_app.platform),
                height_of(shadow_platform_of(&outcome))
            ),
            (8, 8)
        );
        assert!(!report.join("divergences").exists());
    }

    /// A divergence that is found with the block committed by both, on a node that is to
    /// stop: the block is answered, the node ends after it, and the next `FinalizeBlock`
    /// is answered with the error. The shadow applies no block after it either: a node
    /// that is started again goes on without it.
    #[tokio::test]
    async fn a_node_that_is_to_stop_ends_after_a_block_that_both_committed() {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest {
            on_divergence: OnDivergence::Stop,
            ..ForATest::on(ShadowEngine::Other)
        };
        let outcome = chain(&mut platform, 5, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        shadow_of(&outcome).plant(Fault::ShadowFinalizeAnswer);
        let ended = Caught(Box::pin(more(outcome, 2, &identities()))).await;
        let said = ended
            .err()
            .unwrap_or_else(|| panic!("the chain ends after the divergence"));
        assert!(
            said.contains("should finalize block #7") && said.contains("diverge at height 6 "),
            "{said}"
        );
        assert_eq!((height_of(primary), height_of(shadow_platform)), (6, 6));
        let counts = counts_in(&report);
        assert_eq!(
            counts.divergences,
            BTreeMap::from([("state".to_string(), 1)])
        );
        assert_eq!((counts.height, counts.blocks_diverged), (6, 1));
        assert_eq!(counts.blocks_in_all(), 6);
        assert_eq!(counts.shadow_stopped.expect("stopped").height, 7);

        let again = shadow_on(primary, None, asked).expect("a shadow that stays stopped");
        assert!(again.platform().is_none());
    }

    #[tokio::test]
    async fn a_panic_of_the_shadow_is_caught_and_the_primary_goes_on() {
        let planted = planted(Fault::ShadowPanicsAtItsCommit).await;
        planted.divergences("shadow_error");
        planted.stopped_at(6);
        assert!(
            planted.differs.contains("panic: planted by a test"),
            "{}",
            planted.differs
        );
        planted.kept(&OF_THE_EXECUTIONS);
    }

    #[tokio::test]
    async fn an_answer_to_finalize_block_that_differs_is_found_after_the_commit() {
        let planted = planted(Fault::ShadowFinalizeAnswer).await;
        planted.divergences("state");
        // Both committed the block: the shadow stops at the next.
        planted.stopped_at(7);
        let differs = &planted.differs;
        assert!(
            differs.contains("DIFFERS height=6 field=finalize_sha256 this="),
            "{differs}"
        );
        assert!(!differs.contains("field=grove_root"), "{differs}");
        assert!(
            differs.contains("found after both databases committed the block"),
            "{differs}"
        );
        planted.kept(&OF_THE_EXECUTIONS);
        planted.kept(&THE_LINES);
        planted.kept(&OF_THE_COMMITS);
    }

    #[tokio::test]
    async fn a_root_that_differs_after_the_commit_is_found() {
        let planted = planted(Fault::ShadowRootAfterTheCommit).await;
        planted.divergences("state");
        planted.stopped_at(7);
        let differs = &planted.differs;
        assert!(
            differs.contains("DIFFERS height=6 field=grove_root this="),
            "{differs}"
        );
        assert!(
            differs.contains("DIFFERS height=6 field=root_is_app_hash this=true other=false"),
            "{differs}"
        );
        assert!(!differs.contains("field=finalize_sha256"), "{differs}");
        planted.kept(&OF_THE_EXECUTIONS);
        planted.kept(&THE_LINES);
        planted.kept(&OF_THE_COMMITS);
    }

    /// A root that is not read after the commits is not a root that was compared: a
    /// divergence, with the block committed on both.
    #[tokio::test]
    async fn a_root_that_is_not_read_after_the_commit_is_a_divergence() {
        let planted = planted(Fault::ShadowRootIsNotRead).await;
        planted.divergences("shadow_error");
        planted.stopped_at(7);
        assert!(
            planted.differs.contains("the root after the commit"),
            "{}",
            planted.differs
        );
        planted.kept(&OF_THE_EXECUTIONS);
        planted.kept(&OF_THE_COMMITS);
    }

    /// A node that proposes can be given `FinalizeBlock` for its proposal with no
    /// `ProcessProposal` before it. Its handler then answers from the proposal it
    /// prepared and calculates no fee again: the fee calculations of the block are the
    /// ones of the preparation, and the block is compared as every block is.
    #[tokio::test]
    async fn a_block_that_is_finalized_right_after_it_was_prepared_is_compared() {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let outcome = chain(
            &mut platform,
            5,
            &strategy,
            ForATest::on(ShadowEngine::Other),
        )
        .await;
        shadow_of(&outcome).plant(Fault::ProcessProposalIsNotGiven);
        let outcome = more(outcome, 1, &identities()).await;
        let counts = shadow_of(&outcome).report().counts();
        no_divergence(&counts, 6);
        let lines = lines_in(&report);
        let line = lines.last().expect("the line of the block");
        assert_eq!(line["height"].as_u64(), Some(6));
        assert!(!line["tx"].as_array().expect("tx").is_empty(), "{line}");
        assert!(line["fee_calcs"].as_u64().expect("fee_calcs") > 0, "{line}");
        assert!(line["fee_ops"].as_u64().expect("fee_ops") > 0, "{line}");
    }

    /// A database of the shadow that was written to from outside holds another state than
    /// its own platform state says: the shadow refuses the next block, which is a
    /// divergence found before either commits, and the primary goes on. A node that starts
    /// again has a shadow that stays stopped and whose database is not opened.
    #[tokio::test]
    async fn a_shadow_database_that_was_changed_from_outside_is_found_at_the_next_block() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest::on(ShadowEngine::Other);
        let outcome = chain(&mut platform, 5, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let platform_version = PlatformVersion::latest();
        shadow_platform
            .drive
            .grove
            .insert(
                &[&[RootTree::Misc as u8][..]],
                b"written from outside",
                Element::new_item(vec![1]),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("an item in the shadow's database");
        let outcome = more(outcome, 2, &strategy).await;
        let counts = shadow_of(&outcome).report().counts();
        assert_eq!(
            counts.divergences,
            BTreeMap::from([("shadow_error".to_string(), 1)])
        );
        assert_eq!(counts.shadow_stopped.clone().expect("stopped").height, 6);
        assert_eq!((height_of(primary), height_of(shadow_platform)), (7, 5));
        assert_eq!((counts.blocks_compared, counts.blocks_diverged), (5, 1));
        assert_eq!(counts.blocks_without_shadow, 1);
        assert_eq!(counts.blocks_in_all(), 7);
        let (kept, differs) = kept_at(&report, 6);
        assert!(differs.contains("field=shadow_error"), "{differs}");
        assert!(differs.contains("the execution of the block"), "{differs}");
        // The shadow's handler finds that its database is not at its own state, and
        // panics: caught, and the text of the panic is the difference.
        assert!(
            differs.contains("panic: drive and platform state app hash mismatch"),
            "{differs}"
        );
        let kept = names_in(&kept);
        for name in [
            "finalize-block-request.bin",
            "primary-process-proposal.bin",
            "primary-fees.txt",
            "shadow-fees.txt",
            "primary-store",
            "shadow-store",
        ] {
            assert!(kept.iter().any(|kept| kept == name), "{name} in {kept:?}");
        }

        // A new application on the primary's database, as after a start: the shadow
        // stopped, so that its database is not opened.
        let parameters = from_the_end_of(&outcome, 1);
        drop(outcome.abci_app);
        let again = shadow_on(primary, None, asked).expect("a shadow that stays stopped");
        assert!(again.platform().is_none());
        let application = FullAbciApplication::new(primary).with_shadow(again);
        let outcome = continue_chain_for_strategy(
            application,
            parameters,
            strategy.clone(),
            config(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        let counts = shadow_of(&outcome).report().counts();
        assert_eq!(counts.starts, 2);
        assert_eq!(counts.blocks_without_shadow, 2);
        assert_eq!(counts.divergences_in_all(), 1);
        assert_eq!(counts.blocks_in_all(), 8);
        assert_eq!(counts.shadow_stopped.expect("stopped").height, 6);
        assert_eq!((height_of(primary), height_of(shadow_platform)), (8, 5));
    }

    /// A node that stops after the shadow committed a block and before the primary did
    /// starts with the shadow one block ahead: each database holds what it committed,
    /// which is read from it as a start reads it. The primary is given the block again,
    /// the shadow skips it, and the app hash the primary commits is held against the
    /// shadow's. The block after it is compared as every block is, and the primary ends
    /// where a node with no shadow ends that was given the same blocks.
    #[tokio::test]
    async fn a_shadow_that_is_a_block_ahead_after_a_start_skips_it_and_goes_on() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest::on(ShadowEngine::Other);
        let outcome = chain(&mut platform, 5, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let parameters = from_the_end_of(&outcome, 2);
        shadow_of(&outcome).plant(Fault::NodeStopsBetweenTheCommits);
        let stopped = Caught(Box::pin(more(outcome, 1, &strategy))).await;
        assert!(stopped.is_err());

        // What a start finds in the two databases.
        read_from_storage(primary);
        read_from_storage(shadow_platform);
        assert_eq!((height_of(primary), height_of(shadow_platform)), (5, 6));
        assert_eq!(counts_in(&report).height, 5);

        let again =
            shadow_on(primary, Some(shadow_platform), asked).expect("a shadow that is ahead");
        let application = FullAbciApplication::new(primary).with_shadow(again);
        let outcome = continue_chain_for_strategy(
            application,
            parameters,
            strategy.clone(),
            config(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        let counts = shadow_of(&outcome).report().counts();
        assert_eq!(
            counts.heights_skipped,
            vec![SkippedHeight {
                height: 6,
                app_hash_compared: true
            }]
        );
        assert!(counts.divergences.is_empty(), "{:?}", counts.divergences);
        assert_eq!(counts.shadow_stopped, None);
        assert_eq!(counts.blocks_compared, 6);
        assert_eq!(counts.blocks_in_all(), 7);
        assert!(counts.blocks_not_compared.is_empty());
        assert_eq!(counts.starts, 2);

        // What the two databases hold at the end, and what a node alone holds.
        read_from_storage(primary);
        read_from_storage(shadow_platform);
        assert_eq!((height_of(primary), height_of(shadow_platform)), (7, 7));
        assert_eq!(app_hash_of(primary), app_hash_of(shadow_platform));
        assert_eq!(root_of(primary), root_of(shadow_platform));
        assert_eq!(
            app_hash_of(primary),
            app_hash_alone(5, &strategy, 2, &strategy).await
        );
    }

    /// The same start, where the block the primary is given again is another than the one
    /// the shadow holds: the app hash the primary commits at the shadow's height is not
    /// the shadow's, which is found, and the shadow stops at the next block.
    #[tokio::test]
    async fn a_shadow_that_is_ahead_with_another_block_is_found_at_its_height() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest::on(ShadowEngine::Other);
        let outcome = chain(&mut platform, 5, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let parameters = from_the_end_of(&outcome, 2);
        shadow_of(&outcome).plant(Fault::NodeStopsBetweenTheCommits);
        assert!(Caught(Box::pin(more(outcome, 1, &strategy))).await.is_err());
        read_from_storage(primary);
        read_from_storage(shadow_platform);

        let again =
            shadow_on(primary, Some(shadow_platform), asked).expect("a shadow that is ahead");
        let application = FullAbciApplication::new(primary).with_shadow(again);
        // A sixth block with an identity in it, where the shadow's sixth block has none.
        let outcome = continue_chain_for_strategy(
            application,
            parameters,
            identities(),
            config(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        let counts = shadow_of(&outcome).report().counts();
        assert_eq!(
            counts.heights_skipped,
            vec![SkippedHeight {
                height: 6,
                app_hash_compared: true
            }]
        );
        assert_eq!(
            counts.divergences,
            BTreeMap::from([("state".to_string(), 1)])
        );
        assert_eq!(counts.shadow_stopped.clone().expect("stopped").height, 7);
        assert_eq!(counts.blocks_without_shadow, 1);
        assert_eq!(counts.blocks_in_all(), 7);
        assert_eq!((height_of(primary), height_of(shadow_platform)), (7, 6));
        let (_, differs) = kept_at(&report, 6);
        assert!(
            differs.starts_with("DIFFERS height=6 field=app_hash this=\""),
            "{differs}"
        );
        assert!(
            differs.contains("found at a height the shadow held before the primary"),
            "{differs}"
        );
    }

    /// A node that stopped after it committed blocks on both databases and before it
    /// counted them starts with counts that are behind its database. The blocks between
    /// are written down then as uncounted: no height is left out, and nothing is said of
    /// what became of them.
    #[tokio::test]
    async fn blocks_that_were_committed_and_not_counted_are_written_down_at_the_next_start() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest::on(ShadowEngine::Other);
        let outcome = chain(&mut platform, 5, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let parameters = from_the_end_of(&outcome, 1);
        drop(outcome.abci_app);

        // The counts as they were after the third block.
        let mut counts = counts_in(&report);
        no_divergence(&counts, 5);
        counts.height = 3;
        counts.shadow_height = 3;
        counts.blocks_compared = 3;
        std::fs::write(
            report.join(STATUS_FILE),
            serde_json::to_vec(&counts).expect("counts are JSON"),
        )
        .expect("written");

        let again = shadow_on(primary, Some(shadow_platform), asked).expect("a shadow");
        let counts = again.report().counts();
        assert_eq!(counts.starts, 2);
        assert_eq!((counts.height, counts.shadow_height), (5, 5));
        assert_eq!(counts.blocks_compared, 3);
        assert!(counts.blocks_not_compared.is_empty());
        assert_eq!(
            counts.blocks_uncounted,
            vec![HeightRange {
                height: 4,
                through: 5
            }]
        );
        // The two heights are in no count of blocks.
        assert_eq!(counts.blocks_in_all(), 3);
        assert_eq!(counts, counts_in(&report));

        // The next block is compared, and a start after it finds nothing to add.
        let application = FullAbciApplication::new(primary).with_shadow(again);
        let outcome = continue_chain_for_strategy(
            application,
            parameters,
            strategy.clone(),
            config(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        let counts = shadow_of(&outcome).report().counts();
        assert_eq!((counts.blocks_compared, counts.blocks_in_all()), (4, 4));
        drop(outcome.abci_app);
        let third = shadow_on(primary, Some(shadow_platform), asked).expect("a shadow");
        let counts = third.report().counts();
        assert_eq!(counts.starts, 3);
        assert_eq!(counts.blocks_uncounted.len(), 1);
        assert_eq!(counts.blocks_in_all(), 4);
    }

    /// An answer to `InitChain` that differs is a divergence at the first height, and the
    /// shadow applies no block.
    #[tokio::test]
    async fn an_answer_to_init_chain_that_differs_is_found() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let asked = ForATest {
            plant: Some(Fault::ShadowInitChainAnswer),
            ..ForATest::on(ShadowEngine::Other)
        };
        let outcome = chain(&mut platform, 3, &strategy, asked).await;
        let shadow = shadow_of(&outcome);
        let counts = shadow.report().counts();
        assert_eq!(
            counts.divergences,
            BTreeMap::from([("state".to_string(), 1)])
        );
        let height = counts.divergence_heights[0];
        assert_eq!(
            counts.shadow_stopped.clone().expect("stopped").height,
            height
        );
        assert_eq!(counts.blocks_compared, 0);
        assert_eq!(counts.blocks_without_shadow, 3);
        assert_eq!(counts.blocks_in_all(), 3);
        assert_eq!(height_of(outcome.abci_app.platform), 3);
        assert_eq!(height_of(shadow_platform_of(&outcome)), 0);
        let (kept, differs) = kept_at(&report, height);
        assert!(
            differs.starts_with(&format!(
                "DIFFERS height={height} field=init_chain_sha256 this=\""
            )),
            "{differs}"
        );
        for name in [
            "init-chain-request.bin",
            "primary-init-chain.bin",
            "shadow-init-chain.bin",
        ] {
            assert!(kept.join(name).is_file(), "{name}");
        }
    }

    /// A shadow that is behind the primary cannot be given the blocks between: shadow mode
    /// does not start on the two databases.
    #[tokio::test]
    async fn a_shadow_that_is_behind_the_primary_is_refused() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let asked = ForATest::on(ShadowEngine::Other);
        let outcome = chain(&mut platform, 3, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let parameters = from_the_end_of(&outcome, 2);
        drop(outcome.abci_app);
        // Two blocks with no shadow.
        continue_chain_for_strategy(
            FullAbciApplication::new(primary),
            parameters,
            strategy.clone(),
            config(),
            StrategyRandomness::SeedEntropy(7),
        )
        .await;
        assert_eq!((height_of(primary), height_of(shadow_platform)), (5, 3));
        let refused = shadow_on(primary, Some(shadow_platform), asked).expect_err("behind");
        let ShadowError::Refused(said) = refused else {
            panic!("{refused}");
        };
        assert!(
            said.contains("the shadow's database is at height 3 and the primary's at 5"),
            "{said}"
        );
        assert!(said.contains("the shadow is behind"), "{said}");
    }

    /// Where the comparisons of stored records keep their checkpoints of the database at
    /// `database`.
    fn comparison_checkpoint(database: &Path, height: u64) -> PathBuf {
        database.join("shadow-compare").join(height.to_string())
    }

    /// With an interval between the comparisons of the stored records, none begins before
    /// it has passed, and one begins after the first block that is committed once it has.
    #[tokio::test]
    async fn the_stored_records_are_compared_when_the_interval_has_passed() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let asked = ForATest {
            store_compare_interval_s: 3600,
            ..ForATest::on(ShadowEngine::Other)
        };
        let outcome = chain(&mut platform, 2, &strategy, asked).await;
        let comparisons = shadow_of(&outcome).report().counts().store_comparisons;
        assert_eq!(
            (comparisons.made, comparisons.failed, comparisons.running),
            (0, 0, false)
        );

        // Two hours later, as far as the shadow knows.
        shadow_of(&outcome).the_last_store_comparison_began(std::time::Duration::from_secs(7200));
        let gate = shadow_of(&outcome).hold_the_store_comparisons();
        let outcome = more(outcome, 1, &strategy).await;
        let shadow = shadow_of(&outcome);
        let comparisons = shadow.report().counts().store_comparisons;
        assert_eq!(
            (comparisons.made, comparisons.failed, comparisons.running),
            (0, 0, true)
        );
        gate.open();
        gate.wait_until_done();
        let comparisons = shadow.report().counts().store_comparisons;
        assert_eq!(
            (
                comparisons.made,
                comparisons.failed,
                comparisons.differing,
                comparisons.running
            ),
            (1, 0, 0, false)
        );
        assert_eq!(comparisons.last.expect("the comparison").height, 3);

        // The interval begins again with it: the next block begins none.
        let outcome = more(outcome, 1, &strategy).await;
        let comparisons = shadow_of(&outcome).report().counts().store_comparisons;
        assert_eq!(
            (comparisons.made, comparisons.failed, comparisons.running),
            (1, 0, false)
        );
    }

    /// A comparison of the stored records that is asked for by the file begins after the
    /// next block, on checkpoints of the two databases at that block. The node does not
    /// wait for it: while the comparison is held at its start, the next blocks are
    /// committed and compared. A file that asks while one runs begins the next when that
    /// one has ended.
    #[tokio::test]
    async fn the_stored_records_are_compared_on_checkpoints_while_the_node_goes_on() {
        let strategy = identities_and_documents();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let database = platform.config.db_path.clone();
        let report = report_directory(&database);
        let outcome = chain(
            &mut platform,
            5,
            &strategy,
            ForATest::on(ShadowEngine::Other),
        )
        .await;
        assert_eq!(
            shadow_of(&outcome).report().counts().store_comparisons.made,
            0
        );
        let checkpoints = [
            comparison_checkpoint(&database, 6),
            comparison_checkpoint(&shadow_platform_of(&outcome).config.db_path, 6),
        ];
        let asks = report.join(Shadow::<MockCoreRPCLike>::COMPARE_NOW_FILE);
        std::fs::write(&asks, b"").expect("the file that asks");
        let gate = shadow_of(&outcome).hold_the_store_comparisons();
        let outcome = more(outcome, 1, &identities()).await;

        // It began, and waits where the test holds it.
        let comparisons = shadow_of(&outcome).report().counts().store_comparisons;
        assert_eq!(
            (comparisons.made, comparisons.failed, comparisons.running),
            (0, 0, true)
        );
        assert!(!asks.exists());
        for checkpoint in &checkpoints {
            assert!(checkpoint.is_dir(), "{}", checkpoint.display());
        }

        // The node goes on: two more blocks, compared as every block is, and a file that
        // asks again begins nothing while the first comparison runs.
        std::fs::write(&asks, b"").expect("the file that asks");
        let outcome = more(outcome, 2, &identities()).await;
        let counts = shadow_of(&outcome).report().counts();
        no_divergence(&counts, 8);
        assert_eq!(
            (
                counts.store_comparisons.made,
                counts.store_comparisons.failed,
                counts.store_comparisons.running
            ),
            (0, 0, true)
        );
        assert!(asks.exists());
        assert_eq!(lines_in(&report).len(), 8);

        gate.open();
        gate.wait_until_done();
        let shadow = shadow_of(&outcome);
        let comparisons = shadow.report().counts().store_comparisons;
        assert_eq!(
            (
                comparisons.made,
                comparisons.failed,
                comparisons.differing,
                comparisons.running
            ),
            (1, 0, 0, false)
        );
        // It compared the databases as they were at the sixth block, with the node two
        // blocks on.
        let last = comparisons.last.expect("the comparison");
        assert_eq!(
            (last.height, last.error.clone(), last.differing),
            (6, None, 0)
        );
        assert!(last.subtrees_compared > 50, "{}", last.subtrees_compared);
        for checkpoint in &checkpoints {
            assert!(!checkpoint.exists(), "{}", checkpoint.display());
        }
        assert!(shadow.report().counts().divergences.is_empty());

        // The file that asked meanwhile begins the next one after the next block.
        let second = shadow.hold_the_store_comparisons();
        let outcome = more(outcome, 1, &identities()).await;
        assert!(!asks.exists());
        second.open();
        second.wait_until_done();
        let counts = shadow_of(&outcome).report().counts();
        assert_eq!(
            (
                counts.store_comparisons.made,
                counts.store_comparisons.failed
            ),
            (2, 0)
        );
        assert_eq!(
            counts
                .store_comparisons
                .last
                .clone()
                .expect("the comparison")
                .height,
            9
        );
        no_divergence(&counts, 9);
    }

    /// A chain of seven blocks whose comparison of stored records, begun after the sixth,
    /// is held at its start and given a checkpoint of the shadow at the seventh block in
    /// place of the one at the sixth: stored records that differ, with both databases as
    /// they should be. Returns the chain after the comparison ended.
    async fn stored_records_that_differ<'a>(
        platform: &'a mut Platform<MockCoreRPCLike>,
        on_divergence: OnDivergence,
    ) -> (ChainExecutionOutcome<'a>, [PathBuf; 2]) {
        let database = platform.config.db_path.clone();
        let report = report_directory(&database);
        let asked = ForATest {
            on_divergence,
            ..ForATest::on(ShadowEngine::Other)
        };
        let outcome = chain(platform, 5, &identities_and_documents(), asked).await;
        let shadow_platform = shadow_platform_of(&outcome);
        let checkpoints = [
            comparison_checkpoint(&database, 6),
            comparison_checkpoint(&shadow_platform.config.db_path, 6),
        ];
        std::fs::write(
            report.join(Shadow::<MockCoreRPCLike>::COMPARE_NOW_FILE),
            b"",
        )
        .expect("the file that asks");
        let gate = shadow_of(&outcome).hold_the_store_comparisons();
        let outcome = more(outcome, 2, &identities()).await;
        no_divergence(&shadow_of(&outcome).report().counts(), 7);

        std::fs::remove_dir_all(&checkpoints[1]).expect("the checkpoint at the sixth block");
        shadow_platform
            .drive
            .grove
            .create_checkpoint(&checkpoints[1])
            .expect("a checkpoint at the seventh block");
        gate.open();
        gate.wait_until_done();

        let counts = shadow_of(&outcome).report().counts();
        assert_eq!(
            counts.divergences,
            BTreeMap::from([("store".to_string(), 1)])
        );
        assert_eq!(counts.divergence_heights, vec![6]);
        let comparisons = &counts.store_comparisons;
        assert_eq!(
            (
                comparisons.made,
                comparisons.failed,
                comparisons.differing,
                comparisons.running
            ),
            (1, 0, 1, false)
        );
        // The checkpoints are the evidence, and stay.
        for checkpoint in &checkpoints {
            assert!(checkpoint.is_dir(), "{}", checkpoint.display());
            assert!(checkpoint.with_file_name("6.kept").is_file());
        }
        let (kept, differs) = kept_at(&report, 6);
        assert!(
            differs.starts_with("DIFFERS height=6 field=stored_records this=\""),
            "{differs}"
        );
        assert!(kept.join("stores.json").is_file());
        (outcome, checkpoints)
    }

    /// Stored records that differ are found by a thread that executes no block. The
    /// thread that does learns of it at its next block: the shadow applies none from
    /// there on, and the node goes on.
    #[tokio::test]
    async fn stored_records_that_differ_stop_the_shadow_at_the_next_block() {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let (outcome, checkpoints) =
            stored_records_that_differ(&mut platform, OnDivergence::Continue).await;
        // The stop is written with the comparison as one that is asked for, and with its
        // height by the next block.
        let counts = counts_in(&report);
        assert_eq!(counts.shadow_stopped, None);
        assert_eq!(
            counts.shadow_stop_pending.as_deref(),
            Some("the stored records of the two databases differ at height 6")
        );
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);

        let outcome = more(outcome, 2, &identities()).await;
        let counts = shadow_of(&outcome).report().counts();
        let stopped = counts.shadow_stopped.clone().expect("stopped");
        assert_eq!(stopped.height, 8);
        assert_eq!(
            stopped.reason,
            "the stored records of the two databases differ at height 6"
        );
        assert_eq!((height_of(primary), height_of(shadow_platform)), (9, 7));
        assert_eq!(
            (
                counts.blocks_compared,
                counts.blocks_diverged,
                counts.blocks_without_shadow
            ),
            (7, 0, 2)
        );
        assert_eq!(counts.blocks_in_all(), 9);
        assert_eq!(counts.divergences_in_all(), 1);
        assert_eq!(counts.shadow_stop_pending, None);

        // A start after it removes nothing of the evidence, and opens no shadow.
        drop(outcome.abci_app);
        let asked = ForATest::on(ShadowEngine::Other);
        let again = shadow_on(primary, None, asked).expect("a shadow that stays stopped");
        assert_eq!(again.report().counts().starts, 2);
        for checkpoint in &checkpoints {
            assert!(checkpoint.is_dir(), "{}", checkpoint.display());
        }
    }

    /// The same where the configuration ends the node: it ends after the block it is at,
    /// and the next `FinalizeBlock` is answered with the error, which names the height of
    /// the comparison.
    #[tokio::test]
    async fn stored_records_that_differ_end_a_node_that_is_to_stop() {
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let report = report_directory(&platform.config.db_path);
        let (outcome, _) = stored_records_that_differ(&mut platform, OnDivergence::Stop).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let ended = Caught(Box::pin(more(outcome, 1, &identities()))).await;
        let said = ended
            .err()
            .unwrap_or_else(|| panic!("the chain ends after the comparison"));
        assert!(
            said.contains("the two storage engines diverge at height 6 "),
            "{said}"
        );
        assert_eq!((height_of(primary), height_of(shadow_platform)), (7, 7));
        let counts = counts_in(&report);
        assert_eq!(counts.shadow_stopped, None);
        assert_eq!(
            counts.shadow_stop_pending.as_deref(),
            Some("the stored records of the two databases differ at height 6")
        );
        assert_eq!((counts.height, counts.blocks_in_all()), (7, 7));

        // A node that is started again goes on without the shadow, which is not opened.
        let asked = ForATest {
            on_divergence: OnDivergence::Stop,
            ..ForATest::on(ShadowEngine::Other)
        };
        let again = shadow_on(primary, None, asked).expect("a shadow that stays stopped");
        let stopped = again.report().counts().shadow_stopped.expect("stopped");
        assert_eq!(stopped.height, 8);
        assert!(
            stopped
                .reason
                .starts_with("the stored records of the two databases differ at height 6; "),
            "{}",
            stopped.reason
        );
    }

    /// A checkpoint that a comparison left behind, as where the node stopped while one
    /// ran, is removed at the next start, from both databases.
    #[tokio::test]
    async fn what_a_comparison_left_behind_is_removed_at_the_next_start() {
        let strategy = nothing();
        let mut platform = TestPlatformBuilder::new()
            .with_config(config())
            .build_with_mock_rpc();
        let database = platform.config.db_path.clone();
        let report = report_directory(&database);
        let asked = ForATest::on(ShadowEngine::Other);
        let outcome = chain(&mut platform, 2, &strategy, asked).await;
        let primary = outcome.abci_app.platform;
        let shadow_platform = shadow_platform_of(&outcome);
        let checkpoints = [
            comparison_checkpoint(&database, 3),
            comparison_checkpoint(&shadow_platform.config.db_path, 3),
        ];
        std::fs::write(
            report.join(Shadow::<MockCoreRPCLike>::COMPARE_NOW_FILE),
            b"",
        )
        .expect("the file that asks");
        // The comparison is held and not let go on: the application is dropped with it
        // running, as a node that stops.
        let gate = shadow_of(&outcome).hold_the_store_comparisons();
        let outcome = more(outcome, 1, &strategy).await;
        assert!(counts_in(&report).store_comparisons.running);
        for checkpoint in &checkpoints {
            assert!(checkpoint.is_dir(), "{}", checkpoint.display());
        }
        drop(outcome.abci_app);

        let again = shadow_on(primary, Some(shadow_platform), asked).expect("a shadow");
        let comparisons = again.report().counts().store_comparisons;
        assert_eq!(
            (comparisons.made, comparisons.failed, comparisons.running),
            (0, 0, false)
        );
        for checkpoint in &checkpoints {
            assert!(!checkpoint.exists(), "{}", checkpoint.display());
        }
        // The thread of the comparison that the stop left behind ends with nothing done.
        gate.dismiss();
        gate.wait_until_done();
        assert_eq!(again.report().counts().store_comparisons, comparisons);
    }
}
