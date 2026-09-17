mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Creates a readiness fund or adds to an existing one.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `amount` - The credits to add.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The low level operations that perform the write.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn add_readiness_fund_operations(
        &self,
        fund_id: Identifier,
        amount: u64,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.vote.readiness.add_fund {
            Some(0) => self.add_readiness_fund_operations_v0(
                fund_id,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_readiness_fund_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_readiness_fund_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates a readiness fund or adds to an existing one and applies the write.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `amount` - The credits to add.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the write is applied.
    pub fn add_readiness_fund(
        &self,
        fund_id: Identifier,
        amount: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations =
            self.add_readiness_fund_operations(fund_id, amount, &mut None, transaction, platform_version)?;
        let grove_db_operations =
            LowLevelDriveOperation::grovedb_operations_batch_consume(batch_operations);
        self.grove_apply_batch_with_add_costs(
            grove_db_operations,
            false,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
