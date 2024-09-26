use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_fees::v0::BlockFeesV0Methods;
use crate::execution::types::block_fees::BlockFees;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use drive::drive::credit_pools::operations::update_unpaid_epoch_index_operation;
use platform_version::version::PlatformVersion;
use std::sync::Arc;

pub(crate) fn fast_forward_to_block(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: u64,
    height: u64,
    core_block_height: u32,
    epoch_index: u16,
    should_process_epoch_change: bool,
) {
    let platform_state = platform.state.load();

    let mut platform_state = (**platform_state).clone();

    let protocol_version = platform_state.current_protocol_version_in_consensus();
    let platform_version = PlatformVersion::get(protocol_version).unwrap();

    let block_info = BlockInfo {
        time_ms, //less than 2 weeks
        height,
        core_height: core_block_height,
        epoch: Epoch::new(epoch_index).unwrap(),
    };

    platform_state.set_last_committed_block_info(Some(
        ExtendedBlockInfoV0 {
            basic_info: block_info,
            app_hash: platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            proposer_pro_tx_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        }
        .into(),
    ));

    platform.state.store(Arc::new(platform_state.clone()));

    if should_process_epoch_change {
        process_epoch_change(
            platform,
            Some(platform_state),
            time_ms,
            height,
            core_block_height,
            epoch_index,
        )
    }
}

pub(crate) fn process_epoch_change(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: Option<PlatformState>,
    time_ms: u64,
    height: u64,
    core_block_height: u32,
    epoch_index: u16,
) {
    let platform_state = platform_state.unwrap_or_else(|| {
        let platform_state = platform.state.load();

        (**platform_state).clone()
    });

    let protocol_version = platform_state.current_protocol_version_in_consensus();
    let platform_version = PlatformVersion::get(protocol_version).unwrap();

    let block_execution_context: BlockExecutionContext = BlockExecutionContextV0 {
        block_state_info: BlockStateInfoV0 {
            height,
            round: 0,
            block_time_ms: time_ms,
            previous_block_time_ms: time_ms.checked_sub(3000),
            proposer_pro_tx_hash: [0; 32],
            core_chain_locked_height: core_block_height,
            block_hash: Some([0; 32]),
            app_hash: None,
        }
        .into(),
        epoch_info: EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: epoch_index,
            previous_epoch_index: epoch_index.checked_sub(1),
            is_epoch_change: true,
        }),
        unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
        block_platform_state: platform_state,
        proposer_results: None,
    }
    .into();

    let block_fees: BlockFees = BlockFees::from_fees(0, 0);

    let mut operations = vec![];

    platform
        .add_process_epoch_change_operations(
            &block_execution_context,
            &block_fees,
            None,
            &mut operations,
            platform_version,
        )
        .expect("expected to process change operations");

    operations.push(drive::util::batch::DriveOperation::GroveDBOperation(
        update_unpaid_epoch_index_operation(epoch_index),
    ));

    platform
        .drive
        .apply_drive_operations(
            operations,
            true,
            &BlockInfo {
                time_ms,
                height,
                core_height: core_block_height,
                epoch: Epoch::new(epoch_index).unwrap(),
            },
            None,
            platform_version,
            None,
        )
        .expect("expected to apply drive operations");
}
