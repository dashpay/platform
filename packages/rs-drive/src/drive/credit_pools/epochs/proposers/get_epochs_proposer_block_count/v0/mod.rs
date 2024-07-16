use grovedb::{Element, TransactionArg};

use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Returns the given proposer's block count
    pub(super) fn get_epochs_proposer_block_count_v0(
        &self,
        epoch: &Epoch,
        proposer_tx_hash: &[u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let element = self
            .grove
            .get(
                &epoch.get_proposers_path(),
                proposer_tx_hash,
                transaction,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .map_err(Error::GroveDB)?;

        let Element::Item(encoded_proposer_block_count, _) = element else {
            return Err(Error::Drive(DriveError::UnexpectedElementType(
                "epochs proposer block count must be an item",
            )));
        };

        let proposer_block_count =
            u64::from_be_bytes(encoded_proposer_block_count.as_slice().try_into().map_err(
                |_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "epochs proposer block count item have an invalid length",
                    )))
                },
            )?);

        Ok(proposer_block_count)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::epoch::Epoch;
    use grovedb::Element;

    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::drive::credit_pools::epochs::paths::EpochProposers;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;

    use dpp::version::PlatformVersion;

    #[test]
    fn test_error_if_value_has_invalid_length_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let pro_tx_hash: [u8; 32] = rand::random();

        let epoch = Epoch::new(0).unwrap();

        let mut batch = GroveDbOpBatch::new();

        batch.push(epoch.init_proposers_tree_operation());

        batch.add_insert(
            epoch.get_proposers_path_vec(),
            pro_tx_hash.to_vec(),
            Element::Item(u128::MAX.to_be_bytes().to_vec(), None),
        );

        drive
            .grove_apply_batch(batch, false, Some(&transaction), &platform_version.drive)
            .expect("should apply batch");

        let result = drive.get_epochs_proposer_block_count(
            &epoch,
            &pro_tx_hash,
            Some(&transaction),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedSerialization(_),))
        ));
    }

    #[test]
    fn test_error_if_epoch_tree_is_not_initiated_v0() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let pro_tx_hash: [u8; 32] = rand::random();

        let epoch = Epoch::new(7000).unwrap();

        let result = drive.get_epochs_proposer_block_count(
            &epoch,
            &pro_tx_hash,
            Some(&transaction),
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::GroveDB(grovedb::Error::PathParentLayerNotFound(_)))
        ));
    }
}
