mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Empties a readiness fund, deleting its entry.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `error_if_does_not_exist` - Whether a missing fund is an error or an empty result.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The credits the fund held and the low level operations that delete it.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn empty_readiness_fund_operations(
        &self,
        fund_id: Identifier,
        error_if_does_not_exist: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Credits, Vec<LowLevelDriveOperation>), Error> {
        match platform_version.drive.methods.vote.readiness.empty_fund {
            Some(0) => self.empty_readiness_fund_operations_v0(
                fund_id,
                error_if_does_not_exist,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "empty_readiness_fund_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "empty_readiness_fund_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
