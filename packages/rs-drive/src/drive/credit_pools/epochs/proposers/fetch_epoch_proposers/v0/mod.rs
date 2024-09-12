use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, TransactionArg};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::proposer_block_count_query::ProposerQueryType;
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Retrieves the list of block proposers for a given epoch.
    ///
    /// This function fetches the proposers for a specified epoch, returning their transaction hashes and
    /// the number of blocks they proposed. The query can either be limited by a range or a set of proposer IDs.
    ///
    /// # Parameters
    ///
    /// - `epoch_tree`: A reference to an `Epoch` instance representing the epoch for which to retrieve proposers.
    /// - `query_type`: The type of query to perform (`ProposerQueryType`), which can either specify a range of
    ///   proposers or fetch specific proposers by their IDs.
    /// - `transaction`: A `TransactionArg` that provides the transaction context for the query execution.
    /// - `platform_version`: The version of the platform, represented by a `PlatformVersion` instance, ensuring compatibility
    ///   between different platform versions.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Vec<(Vec<u8>, u64)>`: A vector of tuples where each tuple contains:
    ///   - A byte vector (`Vec<u8>`) representing the proposer's transaction hash.
    ///   - A `u64` representing the number of blocks proposed by that proposer.
    /// - `Error`: An error if the query fails due to an invalid platform version, transaction issues, or invalid epoch data.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported.
    /// - There is an issue with retrieving the proposers, such as a database or transaction error.
    /// - The provided epoch or query type is invalid.
    pub(super) fn fetch_epoch_proposers_v0(
        &self,
        epoch_tree: &Epoch,
        query_type: ProposerQueryType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Vec<u8>, u64)>, Error> {
        let use_optional = query_type.allows_optional();

        let path_query = query_type.into_path_query(epoch_tree);

        let proposers = if use_optional {
            let key_elements = self.grove_get_raw_path_query_with_optional(
                &path_query,
                true,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?;

            key_elements
                .into_iter()
                .map(|(_, pro_tx_hash, element)| {
                    let block_count = match element {
                        None => 0,
                        Some(Element::Item(encoded_block_count, _)) => u64::from_be_bytes(
                            encoded_block_count.as_slice().try_into().map_err(|_| {
                                Error::Drive(DriveError::CorruptedSerialization(String::from(
                                    "epochs proposer block count must be u64",
                                )))
                            })?,
                        ),
                        _ => {
                            return Err(Error::Drive(DriveError::UnexpectedElementType(
                                "epochs proposer block count must be an item",
                            )));
                        }
                    };

                    Ok((pro_tx_hash, block_count))
                })
                .collect::<Result<_, _>>()
        } else {
            let key_elements = self
                .grove
                .query_raw(
                    &path_query,
                    transaction.is_some(),
                    true,
                    true,
                    QueryKeyElementPairResultType,
                    transaction,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(Error::GroveDB)?
                .0
                .to_key_elements();

            key_elements
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
                .collect::<Result<_, _>>()
        }?;

        Ok(proposers)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;

    use crate::query::proposer_block_count_query::ProposerQueryType;
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
            .fetch_epoch_proposers(
                &epoch,
                ProposerQueryType::ByRange(Some(100), None),
                Some(&transaction),
                platform_version,
            )
            .expect("should get proposers");

        assert_eq!(result, vec!((pro_tx_hash.to_vec(), block_count)));
    }
}
