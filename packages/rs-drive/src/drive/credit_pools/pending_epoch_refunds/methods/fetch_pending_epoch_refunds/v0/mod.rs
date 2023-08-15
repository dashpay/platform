use crate::drive::credit_pools::pending_epoch_refunds::pending_epoch_refunds_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::balances::credits::Creditable;
use dpp::fee::epoch::CreditsPerEpoch;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Element, PathQuery, Query, TransactionArg};

impl Drive {
    /// Fetches all pending epoch refunds
    pub(super) fn fetch_pending_epoch_refunds_v0(
        &self,
        transaction: TransactionArg,
    ) -> Result<CreditsPerEpoch, Error> {
        let mut query = Query::new();

        query.insert_all();

        let (query_result, _) = self
            .grove
            .query_raw(
                &PathQuery::new_unsized(pending_epoch_refunds_path_vec(), query),
                transaction.is_some(),
                QueryResultType::QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        query_result
            .to_key_elements()
            .into_iter()
            .map(|(epoch_index_key, element)| {
                let epoch_index =
                    u16::from_be_bytes(epoch_index_key.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(
                            "epoch index for pending pool updates must be i64",
                        ))
                    })?);

                if let Element::SumItem(credits, _) = element {
                    Ok((epoch_index, credits.to_unsigned()))
                } else {
                    Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "pending refund credits must be sum items",
                    )))
                }
            })
            .collect::<Result<CreditsPerEpoch, Error>>()
    }
}
