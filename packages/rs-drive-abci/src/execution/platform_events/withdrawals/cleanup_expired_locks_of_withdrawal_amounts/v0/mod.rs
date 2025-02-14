use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::drive::identity::withdrawals::paths::get_withdrawal_transactions_sum_tree_path_vec;
use drive::grovedb::{MaybeTree, PathQuery, QueryItem, Transaction};
use drive::util::grove_operations::BatchDeleteApplyType;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    pub(super) fn cleanup_expired_locks_of_withdrawal_amounts_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if platform_version
            .drive_abci
            .withdrawal_constants
            .cleanup_expired_locks_of_withdrawal_amounts_limit
            == 0
        {
            // No clean up
            return Ok(());
        }
        let mut batch_operations = vec![];

        let sum_path = get_withdrawal_transactions_sum_tree_path_vec();

        let mut path_query = PathQuery::new_single_query_item(
            sum_path,
            QueryItem::RangeTo(..block_info.time_ms.to_be_bytes().to_vec()),
        );

        path_query.query.limit = Some(
            platform_version
                .drive_abci
                .withdrawal_constants
                .cleanup_expired_locks_of_withdrawal_amounts_limit,
        );

        self.drive.batch_delete_items_in_path_query(
            &path_query,
            true,
            // we know that we are not deleting a subtree
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            },
            Some(transaction),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        self.drive.apply_batch_low_level_drive_operations(
            None,
            Some(transaction),
            batch_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }
}
