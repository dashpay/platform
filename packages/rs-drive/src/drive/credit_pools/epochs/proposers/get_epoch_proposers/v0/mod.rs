use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, PathQuery, Query, SizedQuery, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee_pools::epochs::paths::EpochProposers;
use dpp::block::epoch::Epoch;

impl Drive {
    /// Returns a list of the Epoch's block proposers
    pub(super) fn get_epoch_proposers_v0(
        &self,
        epoch_tree: &Epoch,
        limit: Option<u16>,
        transaction: TransactionArg,
    ) -> Result<Vec<(Vec<u8>, u64)>, Error> {
        let path_as_vec = epoch_tree.get_proposers_path_vec();

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path_as_vec, SizedQuery::new(query, limit, None));

        let key_elements = self
            .grove
            .query_raw(
                &path_query,
                transaction.is_some(),
                true,
                true,
                QueryKeyElementPairResultType,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB)?
            .0
            .to_key_elements();

        let proposers = key_elements
            .into_iter()
            .map(|(pro_tx_hash, element)| {
                let Element::Item(encoded_block_count, _) = element else {
                    return Err(Error::Drive(DriveError::UnexpectedElementType(
                        "epochs proposer block count must be an item",
                    )));
                };

                let block_count = u64::from_be_bytes(
                    encoded_block_count.as_slice().try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedSerialization(String::from(
                            "epochs proposer block count must be u64",
                        )))
                    })?,
                );

                Ok((pro_tx_hash, block_count))
            })
            .collect::<Result<_, _>>()?;

        Ok(proposers)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::drive::batch::GroveDbOpBatch;
    use crate::fee_pools::epochs::operations_factory::EpochOperations;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;

    use dpp::version::PlatformVersion;

    #[test]
    fn test_value() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let pro_tx_hash: [u8; 32] = rand::random();
        let block_count = 42;

        let epoch = Epoch::new(0).unwrap();

        let mut batch = GroveDbOpBatch::new();

        batch.push(epoch.init_proposers_tree_operation());

        batch.push(epoch.update_proposer_block_count_operation(&pro_tx_hash, block_count));

        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let result = drive
            .get_epoch_proposers(&epoch, Some(100), Some(&transaction), platform_version)
            .expect("should get proposers");

        assert_eq!(result, vec!((pro_tx_hash.to_vec(), block_count)));
    }
}
