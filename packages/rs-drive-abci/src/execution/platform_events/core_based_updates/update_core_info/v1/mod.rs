use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::{CoreRPCLike, V24_COINBASE_PAYLOAD_VERSION};
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::drive::system::core_fork_activations::CoreFork;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the core information in the platform state based on the given core block height.
    ///
    /// Version 0's steps, after first recording the Core forks seen active for the first time
    /// at `core_block_height` (`record_core_fork_activations_v1`). Like the other steps, the
    /// recording runs only at init chain and in a block that moves the chain locked height.
    ///
    /// # Arguments
    ///
    /// * platform_state - A reference to the platform state before execution of current block.
    /// * block_platform_state - A mutable reference to the current platform state in the block
    ///   execution context to be updated.
    /// * core_block_height - The current block height in the Dash Core.
    /// * is_init_chain - A boolean indicating if the chain is being initialized.
    /// * block_info - A reference to the block information.
    /// * transaction - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    ///   Core cannot tell whether a fork is active, or if there is a problem updating the
    ///   masternode list, quorum information, or the state.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_core_info_v1(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // the core height of the block platform state is the last committed
        if !is_init_chain && block_platform_state.last_committed_core_height() == core_block_height
        {
            // if we get the same height that we know we do not need to update core info
            return Ok(());
        }

        self.record_core_fork_activations_v1(core_block_height, transaction, platform_version)?;

        self.update_masternode_list(
            platform_state,
            block_platform_state,
            core_block_height,
            is_init_chain,
            block_info,
            transaction,
            platform_version,
        )?;

        self.update_quorum_info(
            platform_state,
            block_platform_state,
            core_block_height,
            false,
            platform_version,
        )
    }

    /// Records `core_block_height` as the height at which Core's V24 is active, if Core's V24
    /// is active there and nothing is recorded yet. This generation tracks V24 only; tracking
    /// another fork needs a new generation with that fork's signal.
    ///
    /// V24's signal is the payload version of the coinbase of the Core block at
    /// `core_block_height`: Core requires version 4 exactly while V24 is active, so every node
    /// gets the same answer at a chain locked height. Forks never switch off, so once V24 is
    /// recorded Core is not asked again. A Core error or a coinbase without a payload fails the
    /// block rather than read as "not active".
    ///
    /// The record is read and written only through the block's transaction, so a round that is
    /// thrown away takes its write with it.
    fn record_core_fork_activations_v1(
        &self,
        core_block_height: u32,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if self
            .drive
            .fetch_core_fork_activation_height(CoreFork::V24, Some(transaction), platform_version)?
            .is_some()
        {
            return Ok(());
        }
        // Every block at a Core height Platform follows carries a coinbase payload.
        let coinbase_payload_version = self
            .core_rpc
            .get_coinbase_payload_version(core_block_height)
            .inspect_err(|error| {
                tracing::error!(
                    core_fork = ?CoreFork::V24,
                    core_block_height,
                    ?error,
                    "could not read the coinbase that tells whether a core fork is active"
                )
            })?
            .ok_or_else(|| {
                ExecutionError::DashCoreBadResponseError(format!(
                    "the coinbase of the block at core height {core_block_height} has no payload"
                ))
            })?;
        if coinbase_payload_version < V24_COINBASE_PAYLOAD_VERSION {
            tracing::debug!(
                core_fork = ?CoreFork::V24,
                core_block_height,
                coinbase_payload_version,
                "core fork not active at this chain locked core height"
            );
            return Ok(());
        }
        self.drive.store_core_fork_activation_height(
            CoreFork::V24,
            core_block_height,
            Some(transaction),
            platform_version,
        )?;
        tracing::info!(
            core_fork = ?CoreFork::V24,
            core_block_height,
            coinbase_payload_version,
            "core fork active at this chain locked core height, recorded in the block being executed"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::execution::ExecutionError;
    use crate::error::Error;
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumListResult, MasternodeListDiff};
    use dpp::dashcore_rpc::Error as CoreRpcError;
    use dpp::version::PlatformVersion;
    use drive::drive::system::core_fork_activations::CoreFork;
    use drive::grovedb::Transaction;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// The Core heights Platform asked the coinbase payload version of, in order
    type CoinbaseReads = Arc<Mutex<Vec<u32>>>;

    /// A platform at `platform_version` whose Core reports no masternode list changes and no
    /// quorums, and `coinbase` for the coinbase payload version at each height. Returns the
    /// heights Platform reads the coinbase at.
    fn platform_with_core(
        platform_version: &PlatformVersion,
        coinbase: impl Fn(u32) -> Result<Option<u16>, CoreRpcError> + Send + 'static,
    ) -> (TempPlatform<MockCoreRPCLike>, CoinbaseReads) {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        // Without the default answers, which would shadow the ones set here.
        platform.core_rpc.checkpoint();
        platform
            .core_rpc
            .expect_get_protx_diff_with_masternodes()
            .returning(|base_height, block_height| {
                Ok(MasternodeListDiff {
                    base_height: base_height.unwrap_or_default(),
                    block_height,
                    added_mns: vec![],
                    removed_mns: vec![],
                    updated_mns: vec![],
                })
            });
        let quorum_types = [
            platform.config.validator_set.quorum_type,
            platform.config.chain_lock.quorum_type,
            platform.config.instant_lock.quorum_type,
        ];
        platform
            .core_rpc
            .expect_get_quorum_listextended()
            .returning(move |_| {
                Ok(ExtendedQuorumListResult {
                    quorums_by_type: quorum_types
                        .iter()
                        .map(|quorum_type| (*quorum_type, Default::default()))
                        .collect::<HashMap<_, _>>(),
                })
            });
        let reads = CoinbaseReads::default();
        let recorded_reads = Arc::clone(&reads);
        platform
            .core_rpc
            .expect_get_coinbase_payload_version()
            .returning(move |core_height| {
                recorded_reads
                    .lock()
                    .expect("expected the coinbase reads")
                    .push(core_height);
                coinbase(core_height)
            });
        (platform, reads)
    }

    /// A Core whose coinbase has payload version 4 from `activation_height` on, as Core requires
    /// while V24 is active, and 3 before.
    fn v24_from(activation_height: u32) -> impl Fn(u32) -> Result<Option<u16>, CoreRpcError> {
        move |core_height| {
            Ok(Some(if core_height >= activation_height {
                4
            } else {
                3
            }))
        }
    }

    /// Updates the core info of a block at chain locked height `core_height` whose previous
    /// block was committed at `previous_core_height`, as `run_block_proposal` does.
    fn run_block(
        platform: &TempPlatform<MockCoreRPCLike>,
        block_state: &mut PlatformState,
        previous_core_height: u32,
        core_height: u32,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let height = block_state.last_committed_block_height() + 1;
        block_state.set_last_committed_block_info(Some(
            ExtendedBlockInfoV0 {
                basic_info: BlockInfo {
                    height: height - 1,
                    core_height: previous_core_height,
                    ..Default::default()
                },
                app_hash: [0; 32],
                quorum_hash: [0; 32],
                block_id_hash: [0; 32],
                proposer_pro_tx_hash: [0; 32],
                signature: [0; 96],
                round: 0,
            }
            .into(),
        ));
        platform.update_core_info(
            None,
            block_state,
            core_height,
            false,
            &BlockInfo {
                height,
                core_height,
                ..Default::default()
            },
            transaction,
            platform_version,
        )
    }

    fn recorded_v24(
        platform: &TempPlatform<MockCoreRPCLike>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Option<u32> {
        platform
            .drive
            .fetch_core_fork_activation_height(CoreFork::V24, Some(transaction), platform_version)
            .expect("expected to read the V24 record")
    }

    /// Core requires coinbase payload version 4 exactly while V24 is active, and refuses it
    /// otherwise; a later version keeps V24's fields, so it means V24 too.
    #[test]
    fn should_record_v24_where_the_coinbase_has_payload_version_4_or_more() {
        let platform_version = PlatformVersion::latest();
        for (payload_version, expected) in [(3, None), (4, Some(101)), (5, Some(101))] {
            let (platform, reads) =
                platform_with_core(platform_version, move |_| Ok(Some(payload_version)));
            let transaction = platform.drive.grove.start_transaction();
            let mut block_state = platform.state.load().as_ref().clone();

            run_block(
                &platform,
                &mut block_state,
                100,
                101,
                &transaction,
                platform_version,
            )
            .expect("expected to update the core info");

            assert_eq!(
                recorded_v24(&platform, &transaction, platform_version),
                expected,
                "coinbase payload version {payload_version}"
            );
            assert_eq!(*reads.lock().unwrap(), vec![101]);
        }
    }

    /// Forks never switch off, so once V24 is recorded no block depends on Core about it.
    #[test]
    fn should_never_ask_core_about_v24_once_it_is_recorded() {
        let platform_version = PlatformVersion::latest();
        let (platform, reads) = platform_with_core(platform_version, |core_height| {
            panic!("Core asked about the coinbase at {core_height} after V24 was recorded")
        });
        let transaction = platform.drive.grove.start_transaction();
        platform
            .drive
            .store_core_fork_activation_height(
                CoreFork::V24,
                90,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to record V24");
        let mut block_state = platform.state.load().as_ref().clone();

        for (previous_core_height, core_height) in [(100, 101), (101, 150)] {
            run_block(
                &platform,
                &mut block_state,
                previous_core_height,
                core_height,
                &transaction,
                platform_version,
            )
            .expect("expected to update the core info");
        }

        assert!(reads.lock().unwrap().is_empty());
        assert_eq!(
            recorded_v24(&platform, &transaction, platform_version),
            Some(90)
        );
    }

    /// A Core error or a coinbase without a payload is never read as "not active": the block
    /// fails and nothing is recorded.
    #[test]
    fn should_fail_without_recording_when_core_cannot_tell() {
        let platform_version = PlatformVersion::latest();

        let (platform, _) = platform_with_core(platform_version, |_| Ok(None));
        let transaction = platform.drive.grove.start_transaction();
        let mut block_state = platform.state.load().as_ref().clone();
        let result = run_block(
            &platform,
            &mut block_state,
            100,
            101,
            &transaction,
            platform_version,
        );
        assert!(
            matches!(
                result,
                Err(Error::Execution(ExecutionError::DashCoreBadResponseError(
                    _
                )))
            ),
            "a coinbase without a payload must fail the block, got {result:?}"
        );
        assert_eq!(
            recorded_v24(&platform, &transaction, platform_version),
            None
        );

        let (platform, _) = platform_with_core(platform_version, |_| {
            Err(CoreRpcError::UnexpectedStructure(
                "Core is unreachable".to_string(),
            ))
        });
        let transaction = platform.drive.grove.start_transaction();
        let mut block_state = platform.state.load().as_ref().clone();
        let result = run_block(
            &platform,
            &mut block_state,
            100,
            101,
            &transaction,
            platform_version,
        );
        assert!(
            matches!(result, Err(Error::CoreRpc(_))),
            "a Core error must fail the block, got {result:?}"
        );
        assert_eq!(
            recorded_v24(&platform, &transaction, platform_version),
            None
        );
    }

    /// A block that keeps the chain locked height makes no Core call, so a Core outage there
    /// does not stop Platform.
    #[test]
    fn should_not_ask_core_about_forks_at_an_unchanged_chain_locked_height() {
        let platform_version = PlatformVersion::latest();
        let (platform, reads) = platform_with_core(platform_version, v24_from(0));
        let transaction = platform.drive.grove.start_transaction();
        let mut block_state = platform.state.load().as_ref().clone();

        run_block(
            &platform,
            &mut block_state,
            100,
            100,
            &transaction,
            platform_version,
        )
        .expect("expected to skip the core info update");

        assert!(reads.lock().unwrap().is_empty());
        assert_eq!(
            recorded_v24(&platform, &transaction, platform_version),
            None
        );
    }

    /// At init chain the core info is updated from scratch at the genesis core height, although
    /// the state already holds that height as its last committed one, so a chain born at this
    /// protocol version records a V24 active there and asks Core about one that is not.
    #[test]
    fn should_ask_core_about_v24_at_init_chain() {
        const GENESIS_CORE_HEIGHT: u32 = 1_000;
        let platform_version = PlatformVersion::latest();
        for (activation_height, expected) in [
            (GENESIS_CORE_HEIGHT, Some(GENESIS_CORE_HEIGHT)),
            (GENESIS_CORE_HEIGHT + 1, None),
        ] {
            let (platform, reads) =
                platform_with_core(platform_version, v24_from(activation_height));
            let transaction = platform.drive.grove.start_transaction();
            let mut block_state = platform.state.load().as_ref().clone();
            block_state.set_genesis_block_info(Some(BlockInfo {
                core_height: GENESIS_CORE_HEIGHT,
                ..Default::default()
            }));
            assert_eq!(
                block_state.last_committed_core_height(),
                GENESIS_CORE_HEIGHT
            );

            platform
                .update_core_info(
                    None,
                    &mut block_state,
                    GENESIS_CORE_HEIGHT,
                    true,
                    &BlockInfo {
                        core_height: GENESIS_CORE_HEIGHT,
                        ..Default::default()
                    },
                    &transaction,
                    platform_version,
                )
                .expect("expected to update the core info");

            assert_eq!(*reads.lock().unwrap(), vec![GENESIS_CORE_HEIGHT]);
            assert_eq!(
                recorded_v24(&platform, &transaction, platform_version),
                expected,
                "V24 active from {activation_height}"
            );
        }
    }

    /// The record is a cache of Core's answer, not a new rule: after every block that moves the
    /// chain locked height, "recorded" equals Core's answer at that height, whatever the heights
    /// and however far they jump past the activation, and Core is asked only until it is
    /// recorded.
    #[test]
    fn should_agree_with_core_after_every_block_that_moves_the_chain_locked_height() {
        let platform_version = PlatformVersion::latest();
        let cases: [(u32, &[u32], Option<u32>); 6] = [
            // Activation inside the run, one height at a time.
            (103, &[101, 102, 103, 104, 105], Some(103)),
            // A jump over the activation height records the height jumped to.
            (103, &[101, 101, 102, 110, 110, 111], Some(110)),
            // Unchanged heights before and after the activation.
            (102, &[100, 100, 102, 102, 102, 103], Some(102)),
            // Active before the run; the first block keeps the committed height, 100.
            (50, &[100, 101, 101, 140], Some(101)),
            // Never active in the run.
            (1_000, &[101, 120, 120, 999], None),
            // Activation exactly at a height reached by a jump.
            (120, &[101, 120, 121], Some(120)),
        ];
        for (activation_height, core_heights, expected) in cases {
            let core = v24_from(activation_height);
            let core_says_active = |core_height| core(core_height).unwrap() == Some(4);
            let (platform, reads) =
                platform_with_core(platform_version, v24_from(activation_height));
            let transaction = platform.drive.grove.start_transaction();
            let mut block_state = platform.state.load().as_ref().clone();
            let mut previous_core_height = 100;
            let mut expected_reads = vec![];
            let mut moved = false;
            for &core_height in core_heights {
                let recorded_before = recorded_v24(&platform, &transaction, platform_version);
                run_block(
                    &platform,
                    &mut block_state,
                    previous_core_height,
                    core_height,
                    &transaction,
                    platform_version,
                )
                .expect("expected to update the core info");
                if core_height != previous_core_height {
                    moved = true;
                    if recorded_before.is_none() {
                        expected_reads.push(core_height);
                    }
                }
                if moved {
                    assert_eq!(
                        recorded_v24(&platform, &transaction, platform_version).is_some(),
                        core_says_active(core_height),
                        "activation {activation_height}, heights {core_heights:?}, at {core_height}"
                    );
                }
                previous_core_height = core_height;
            }
            assert_eq!(
                recorded_v24(&platform, &transaction, platform_version),
                expected,
                "activation {activation_height}, heights {core_heights:?}"
            );
            assert_eq!(
                *reads.lock().unwrap(),
                expected_reads,
                "Core must be asked at each height the chain locked height moves to, until V24 \
                 is recorded, and nowhere else"
            );
        }
    }

    /// Protocol version 14 shipped without the record: its blocks neither ask Core about the
    /// coinbase nor write anything for a Core fork.
    #[test]
    fn should_neither_ask_core_nor_record_at_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
        let (platform, reads) = platform_with_core(platform_version, v24_from(0));
        let transaction = platform.drive.grove.start_transaction();
        let mut block_state = platform.state.load().as_ref().clone();
        let root_hash = |transaction: &Transaction| {
            platform
                .drive
                .grove
                .root_hash(Some(transaction), &platform_version.drive.grove_version)
                .unwrap()
                .expect("expected a root hash")
        };
        let root_before = root_hash(&transaction);

        run_block(
            &platform,
            &mut block_state,
            100,
            101,
            &transaction,
            platform_version,
        )
        .expect("expected to update the core info");

        assert!(reads.lock().unwrap().is_empty());
        assert_eq!(root_hash(&transaction), root_before);
        assert!(platform
            .drive
            .fetch_core_fork_activation_height(
                CoreFork::V24,
                Some(&transaction),
                PlatformVersion::latest()
            )
            .expect("expected to read the V24 record")
            .is_none());
    }
}
