//! Drive Initialization

use crate::drive::Drive;
use crate::error::Error;

use crate::drive::system::{misc_path, misc_path_vec};
use crate::error::drive::DriveError;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::grove_operations::QueryType;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

/// Genesis Core Height Key
#[cfg(any(feature = "server", feature = "verify"))]
pub const GENESIS_CORE_HEIGHT_KEY: &[u8; 1] = b"C";

impl Drive {
    /// Stores the genesis core height in groveDB
    pub fn store_genesis_core_height(
        &self,
        genesis_core_height: CoreBlockHeight,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;
        let mut batch = GroveDbOpBatch::new();

        // In Misc
        batch.add_insert(
            misc_path_vec(),
            GENESIS_CORE_HEIGHT_KEY.to_vec(),
            Element::Item(genesis_core_height.to_be_bytes().to_vec(), None),
        );

        self.grove_apply_batch(batch, false, transaction, drive_version)?;

        Ok(())
    }

    /// Fetches the genesis core height in groveDB
    pub fn fetch_genesis_core_height(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<CoreBlockHeight, Error> {
        let drive_version = &platform_version.drive;

        let genesis_core_height_vec = self
            .grove_get(
                (&misc_path()).into(),
                GENESIS_CORE_HEIGHT_KEY,
                QueryType::StatefulQuery,
                transaction,
                &mut vec![],
                drive_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "genesis core height must exist",
            )))
            .and_then(|element| element.into_item_bytes().map_err(Error::GroveDB))?;

        let genesis_core_height =
            u32::from_be_bytes(genesis_core_height_vec.try_into().map_err(|_| {
                Error::Drive(DriveError::CorruptedSerialization(
                    "storage value must be a u32".to_string(),
                ))
            })?);

        Ok(genesis_core_height)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::prelude::CoreBlockHeight;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_initial_state_structure_proper_heights() {
        let drive = setup_drive_with_initial_state_structure();

        let db_transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let core_genesis_height: CoreBlockHeight = 1320;
        drive
            .store_genesis_core_height(
                core_genesis_height,
                Some(&db_transaction),
                &platform_version,
            )
            .expect("expected to store genesis core height");

        let read_core_genesis_height = drive
            .fetch_genesis_core_height(Some(&db_transaction), &platform_version)
            .expect("expected to fetch genesis core height");

        assert_eq!(core_genesis_height, read_core_genesis_height);
    }
}
