use crate::error::Error;
use crate::platform_types::block_credit_mints::BlockCreditMints;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::Txid;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Mints that name no asset lock (the epoch Core rewards, and in principle a state
    /// transition mint without one) are recorded by the block time through
    /// `Drive::record_credit_inflow`, which records nothing for a zero amount. Asset lock
    /// mints are dated by the Core block that mined each asset lock: Core is asked, once for the
    /// whole block, where it mined them, and only a height at or below the block's chain locked
    /// height is taken (every node agrees on those); any other asset lock waits as pending until
    /// a scanned Core block holds it (`Drive::record_asset_lock_credit_inflow`).
    pub(super) fn record_credit_inflows_for_withdrawals_v0(
        &self,
        state_transition_mints: &BlockCreditMints,
        block_fee_mints: Credits,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.record_credit_inflow(
            block_fee_mints
                .saturating_add(state_transition_mints.not_attributed_to_an_asset_lock()),
            block_info,
            Some(transaction),
            platform_version,
        )?;

        let asset_lock_mints: Vec<([u8; 32], Credits)> = state_transition_mints
            .by_asset_lock()
            .iter()
            .filter(|(_, amount)| **amount > 0)
            .map(|(asset_lock_txid, amount)| (*asset_lock_txid, *amount))
            .collect();

        if asset_lock_mints.is_empty() {
            return Ok(());
        }

        let txids: Vec<Txid> = asset_lock_mints
            .iter()
            .map(|(asset_lock_txid, _)| Txid::from_byte_array(*asset_lock_txid))
            .collect();

        let mined_heights = self.core_rpc.get_transactions_mined_heights(&txids)?;

        for ((asset_lock_txid, amount), mined_height) in
            asset_lock_mints.into_iter().zip(mined_heights)
        {
            // A height above the block's chain locked one is not final, and may differ between
            // nodes whose Core is further ahead: such an asset lock is not mined yet here.
            let mined_at_core_height =
                mined_height.filter(|mined_height| *mined_height <= block_info.core_height);

            self.drive.record_asset_lock_credit_inflow(
                asset_lock_txid,
                amount,
                mined_at_core_height,
                block_info,
                Some(transaction),
                platform_version,
            )?;
        }

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
    use dpp::dashcore::{OutPoint, Txid};
    use dpp::fee::Credits;
    use dpp::platform_value::Bytes36;
    use dpp::version::PlatformVersion;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_pending_asset_lock_inflows_path, get_withdrawal_root_path,
        WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY, WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
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

    /// Asset lock mints are dated by the Core block that mined them: one mined at or below the
    /// block's chain locked height counts from that Core block (and adds nothing once a window
    /// old), one Core reports above it or does not know waits as pending, and none of them is
    /// recorded by the block time.
    #[test]
    fn should_date_asset_lock_mints_by_the_core_block_that_mined_them() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let mut core_rpc = MockCoreRPCLike::new();
        core_rpc
            .expect_get_transactions_mined_heights()
            .times(1)
            .returning(|tx_ids| {
                Ok(tx_ids
                    .iter()
                    .map(|txid| match txid.to_byte_array()[0] {
                        1 => Some(995),  // mined recently: counts until 995 + 552
                        2 => Some(400),  // mined a window ago: adds nothing
                        3 => Some(1001), // above the chain locked height: pending
                        _ => None,       // unknown or in the mempool: pending
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
                &asset_lock_mints(&[(1, 100), (2, 200), (3, 300), (4, 400)]),
                dash_to_credits!(1),
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("expected to record");

        let sum_tree = |key: &[u8; 1]| {
            platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    key,
                    DirectQueryType::StatefulDirectQuery,
                    Some(&transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected the sum")
        };
        // Only the block fee mint is dated by the block time.
        assert_eq!(
            sum_tree(&WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY),
            dash_to_credits!(1) as i64
        );
        // Only the recently mined asset lock is dated by Core.
        assert_eq!(
            sum_tree(&WITHDRAWAL_CORE_DATED_CREDIT_INFLOWS_SUM_TREE_KEY),
            100
        );

        let pending = |asset_lock: u8| {
            platform
                .drive
                .grove_get_raw_optional(
                    (&get_withdrawal_pending_asset_lock_inflows_path()).into(),
                    &[asset_lock; 32],
                    DirectQueryType::StatefulDirectQuery,
                    Some(&transaction),
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to read")
                .is_some()
        };
        assert!(!pending(1));
        assert!(!pending(2));
        assert!(pending(3));
        assert!(pending(4));
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
