use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::block_credit_mints::BlockCreditMints;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::Txid;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::withdrawal::core_credit_pool_unlock_limit::core_credit_pool_window_blocks;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Records the block's mints with one `Drive::record_credit_inflow` call, which records
    /// nothing for a zero amount. An asset lock Core mined longer ago than its credit pool
    /// window minus `core_credit_pool_unlock_mining_delay_blocks` is left out: Core's own
    /// limit already reads it from the window start balance, so it adds only a percent of
    /// itself there, and counting it in full here would let a lock published to Platform late
    /// raise the limit. Every other mint counts by the block time, the schedule the
    /// withdrawal reservations follow, so a deposit and the withdrawal it funds cancel exactly.
    /// Core is asked once for the whole block where it mined the asset locks.
    pub(super) fn record_credit_inflows_for_withdrawals_v0(
        &self,
        state_transition_mints: &BlockCreditMints,
        block_fee_mints: Credits,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut credit_inflows = block_fee_mints
            .saturating_add(state_transition_mints.not_attributed_to_an_asset_lock());

        let asset_lock_mints: Vec<([u8; 32], Credits)> = state_transition_mints
            .by_asset_lock()
            .iter()
            .filter(|(_, amount)| **amount > 0)
            .map(|(asset_lock_txid, amount)| (*asset_lock_txid, *amount))
            .collect();

        if !asset_lock_mints.is_empty() {
            let mining_delay_blocks = platform_version
                .system_limits
                .core_credit_pool_unlock_mining_delay_blocks
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "record_credit_inflows_for_withdrawals v0 requires system_limits.core_credit_pool_unlock_mining_delay_blocks",
                )))?;
            // Mined at or below this height, Core already counts the asset lock in the window
            // start balance of an unlock pooled now (or soon will).
            let stale_at_or_below = block_info.core_height.checked_sub(
                core_credit_pool_window_blocks(self.config.network, platform_version)?
                    .saturating_sub(mining_delay_blocks),
            );

            // Core answers from its active chain and transaction index; one that has not
            // reached the chain locked height yet would report a mined asset lock as unknown.
            // Fail the block on that node rather than record a different inflow.
            self.core_rpc.get_block_hash(block_info.core_height)?;

            let txids: Vec<Txid> = asset_lock_mints
                .iter()
                .map(|(asset_lock_txid, _)| Txid::from_byte_array(*asset_lock_txid))
                .collect();

            let mined_heights = self.core_rpc.get_transactions_mined_heights(&txids)?;

            for ((_, amount), mined_height) in asset_lock_mints.into_iter().zip(mined_heights) {
                // Only a height at or below the chain locked one is final and the same on
                // every node; anything else (above it, in the mempool, unknown) counts.
                let stale = mined_height.zip(stale_at_or_below).is_some_and(
                    |(mined_height, stale_at_or_below)| mined_height <= stale_at_or_below,
                );
                if !stale {
                    credit_inflows = credit_inflows.saturating_add(amount);
                }
            }
        }

        self.drive.record_credit_inflow(
            credit_inflows,
            block_info,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::platform_types::block_credit_mints::BlockCreditMints;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::dash_to_credits;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::BlockHash;
    use dpp::dashcore::{OutPoint, Txid};
    use dpp::fee::Credits;
    use dpp::platform_value::Bytes36;
    use dpp::version::PlatformVersion;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_root_path, WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
    };
    use drive::util::batch::{DriveOperation, SystemOperationType};
    use drive::util::grove_operations::DirectQueryType;

    fn asset_lock_mints(spends: &[(u8, Credits)]) -> BlockCreditMints {
        let mut mints = BlockCreditMints::default();
        for (asset_lock, amount) in spends {
            mints.add(BlockCreditMints::of_operations(&[
                DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                    amount: *amount,
                }),
                DriveOperation::SystemOperation(SystemOperationType::AddUsedAssetLock {
                    asset_lock_outpoint: Bytes36::new(
                        OutPoint::new(Txid::from_byte_array([*asset_lock; 32]), 0).into(),
                    ),
                    asset_lock_value: AssetLockValue::new(
                        *amount,
                        vec![],
                        0,
                        vec![],
                        PlatformVersion::latest(),
                    )
                    .expect("expected an asset lock value"),
                }),
            ]));
        }
        mints
    }

    /// The event records the block's other mints in the credit inflows sum tree,
    /// accumulating within a block time, and records nothing for a block that minted nothing.
    #[test]
    fn should_record_the_blocks_mints_and_skip_zero() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 10,
            epoch: Epoch::default(),
        };

        let inflows = |transaction: &_| {
            platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    Some(transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected the inflows sum")
        };

        platform
            .record_credit_inflows_for_withdrawals(
                &BlockCreditMints::default(),
                0,
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to record nothing");
        assert_eq!(inflows(&transaction), 0);

        platform
            .record_credit_inflows_for_withdrawals(
                &BlockCreditMints::default(),
                dash_to_credits!(3),
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to record");
        platform
            .record_credit_inflows_for_withdrawals(
                &BlockCreditMints::default(),
                dash_to_credits!(2),
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to add to the same entry");

        assert_eq!(inflows(&transaction), dash_to_credits!(5) as i64);
    }

    /// Asset lock mints count by the block time like every other mint, except one Core mined
    /// so long ago that Core's own limit already reads it from its window start balance:
    /// mined at or below the chain locked height minus Core's window (576 on mainnet) plus
    /// `core_credit_pool_unlock_mining_delay_blocks` (48).
    #[test]
    fn should_leave_out_asset_locks_core_mined_a_window_ago() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_block_hash()
            .withf(|core_height| *core_height == 1000)
            .times(1)
            .returning(|_| Ok(BlockHash::all_zeros()));
        core_rpc
            .expect_get_transactions_mined_heights()
            .times(1)
            .returning(|tx_ids| {
                Ok(tx_ids
                    .iter()
                    .map(|txid| match txid.to_byte_array()[0] {
                        1 => Some(995),  // mined recently: counts
                        2 => Some(472),  // mined 528 blocks ago: left out
                        3 => Some(473),  // mined 527 blocks ago: counts
                        4 => Some(1001), // above the chain locked height: counts
                        _ => None,       // unknown or in the mempool: counts
                    })
                    .collect())
            });
        platform.core_rpc = core_rpc;

        let transaction = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 1000,
            epoch: Epoch::default(),
        };

        platform
            .record_credit_inflows_for_withdrawals(
                &asset_lock_mints(&[(1, 100), (2, 200), (3, 300), (4, 400), (5, 500)]),
                dash_to_credits!(1),
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to record");

        assert_eq!(
            platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    Some(&transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected the sum"),
            (dash_to_credits!(1) + 100 + 300 + 400 + 500) as i64
        );
    }

    /// A Core that has not reached the chain locked height would report an asset lock mined
    /// below it as unknown; the block fails on that node instead of recording another inflow.
    #[test]
    fn should_fail_while_core_has_not_reached_the_chain_locked_height() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc.expect_get_block_hash().returning(|_| {
            Err(dpp::dashcore_rpc::Error::UnexpectedStructure(
                "Block height out of range".to_string(),
            ))
        });
        core_rpc.expect_get_transactions_mined_heights().times(0);
        platform.core_rpc = core_rpc;

        let transaction = platform.drive.grove.start_transaction();

        assert!(platform
            .record_credit_inflows_for_withdrawals(
                &asset_lock_mints(&[(1, 100)]),
                0,
                &BlockInfo {
                    core_height: 1000,
                    ..Default::default()
                },
                &transaction,
                platform_version,
            )
            .is_err());
    }

    /// Before protocol version 14 the version slot is `None` and the event does nothing.
    #[test]
    fn should_do_nothing_before_the_feature_exists() {
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version =
            PlatformVersion::get(13).expect("expected to get platform version 13");
        let transaction = platform.drive.grove.start_transaction();

        platform
            .record_credit_inflows_for_withdrawals(
                &asset_lock_mints(&[(1, 100)]),
                dash_to_credits!(3),
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected the event to be a no-op before v14");
    }
}
