mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the token statuses from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs whose statuses are to be proved.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The version of the platform to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_token_statuses(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.token.prove.token_statuses {
            0 => self.prove_token_statuses_v0(token_ids, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_token_statuses".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves the token statuses with associated costs.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to prove the infos for.
    /// * `block_info` - Information about the current block for fee calculation.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_token_statuses_with_costs(
        &self,
        token_ids: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.prove_token_statuses_operations(
            token_ids,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok((value, fees))
    }

    /// Creates the low-level operations needed to prove the Token statuses from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to query the statuses for.
    /// * `transaction` - The current transaction context.
    /// * `drive_operations` - A vector to store the created low-level drive operations.
    /// * `platform_version` - The platform version to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_token_statuses_operations(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.token.prove.token_statuses {
            0 => self.prove_token_statuses_operations_v0(
                token_ids,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_token_statuses_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod dispatcher_tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    /// An empty token id list produces a PathQuery with limit=0, which GroveDB
    /// rejects as "proved path queries can not be for limit 0". This test
    /// pins that rejection so the downstream error-propagation branch is covered.
    #[test]
    fn prove_empty_token_list_errors_from_grovedb() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let err = drive
            .prove_token_statuses(&[], None, platform_version)
            .expect_err("empty list should bubble up a GroveDB InvalidQuery");
        // The exact variant is GroveDB; we just verify the error propagated and
        // did not silently return a proof of nothing.
        let _ = err;
    }

    /// prove_token_statuses_with_costs returns both proof bytes and a FeeResult
    /// against a real stateful query.
    #[test]
    fn prove_with_costs_returns_fee_result() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [0x99u8; 32];
        let (proof, _fees) = drive
            .prove_token_statuses_with_costs(
                &[token_id],
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("prove with costs");
        assert!(!proof.is_empty());
    }
}
