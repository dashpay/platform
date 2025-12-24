use crate::drive::identity::withdrawals::paths::get_withdrawal_transactions_queue_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use crate::util::batch::drive_op_batch::WithdrawalOperationType;
use crate::util::batch::DriveOperation;
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, SizedQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::convert::TryInto;
use std::ops::RangeFull;

impl Drive {
    pub(super) fn dequeue_untied_withdrawal_transactions_v0(
        &self,
        limit: u16,
        transaction: TransactionArg,
        drive_operation_types: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<WithdrawalTransactionIndexAndBytes>, Error> {
        let mut query = Query::new_with_direction(true);

        query.insert_item(QueryItem::RangeFull(RangeFull));

        let path_query = PathQuery {
            path: get_withdrawal_transactions_queue_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(limit),
                offset: None,
            },
        };

        let result_items = self
            .grove
            .query_raw(
                &path_query,
                true,
                true,
                true,
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::from)?
            .0
            .to_key_elements();

        let withdrawal_transactions = result_items
            .into_iter()
            .map(|(index_bytes, element)| match element {
                Element::Item(bytes, _) => {
                    let index = WithdrawalTransactionIndex::from_be_bytes(
                        index_bytes.try_into().map_err(|_| {
                            Error::Drive(DriveError::CorruptedSerialization(String::from(
                                "withdrawal index must be u64",
                            )))
                        })?,
                    );

                    Ok((index, bytes))
                }
                _ => Err(Error::Drive(DriveError::CorruptedWithdrawalNotItem(
                    "withdrawal is not an item",
                ))),
            })
            .collect::<Result<Vec<WithdrawalTransactionIndexAndBytes>, Error>>()?;

        let indexes: Vec<WithdrawalTransactionIndex> = withdrawal_transactions
            .iter()
            .map(|(index, _)| *index)
            .collect();
        if !indexes.is_empty() {
            drive_operation_types.push(DriveOperation::WithdrawalOperation(
                WithdrawalOperationType::MoveWithdrawalTransactionsToBroadcasted { indexes },
            ));
        }

        Ok(withdrawal_transactions)
    }
}
