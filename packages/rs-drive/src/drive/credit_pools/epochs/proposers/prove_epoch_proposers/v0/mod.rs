use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::Error;
use crate::query::proposer_block_count_query::ProposerQueryType;
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Generates a GroveDB proof of the block proposers for a given epoch.
    ///
    /// This function retrieves the list of block proposers for a specific epoch and returns
    /// a proof that can be verified externally. The query can be limited by either a range or a set of proposer IDs.
    ///
    /// # Parameters
    ///
    /// - `epoch_tree`: A reference to an `Epoch` instance representing the epoch for which to retrieve proposers.
    /// - `query_type`: The type of query to perform (`ProposerQueryType`), which can either be a range query with
    ///   an optional limit and starting point or a query by specific proposer IDs.
    /// - `transaction`: A `TransactionArg` representing the transaction context for the query.
    /// - `platform_version`: The version of the platform, represented by a `PlatformVersion` instance, to ensure compatibility.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// - `Vec<u8>`: A byte vector representing the GroveDB proof of the proposers for the epoch.
    /// - `Error`: If the proof generation fails due to invalid data, unsupported platform version, or other errors.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// - The platform version is unknown or unsupported.
    /// - There is an issue generating the GroveDB proof for the epoch proposers.
    /// - The epoch or query type is invalid.
    pub(super) fn prove_epoch_proposers_v0(
        &self,
        epoch_tree: &Epoch,
        query_type: ProposerQueryType,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = query_type.into_path_query(epoch_tree);

        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::drive::Drive;
    use crate::query::proposer_block_count_query::ProposerQueryType;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
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

        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to be able to commit a transaction");

        let limit = 100;

        let proof = drive
            .prove_epoch_proposers(
                &epoch,
                ProposerQueryType::ByRange(Some(limit), None),
                None,
                platform_version,
            )
            .expect("should get proposers");

        let (_, epoch_proposers): (_, Vec<([u8; 32], u64)>) = Drive::verify_epoch_proposers(
            &proof,
            epoch.index,
            ProposerQueryType::ByRange(Some(100), None),
            platform_version,
        )
        .expect("expected to verify epoch proposers");

        assert_eq!(epoch_proposers, vec!((pro_tx_hash, block_count)));
    }
}
