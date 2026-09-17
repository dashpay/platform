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
    /// Deducts from a readiness fund, keeping `reserve` untouchable.
    ///
    /// The reserve is the cleanup reserve charged when the round retires; reports and
    /// membership pages may only spend what lies above it, so a round can always pay for its
    /// own deferred deletion.
    ///
    /// # Parameters
    ///
    /// * `fund_id` - The fund, derived from the round id.
    /// * `amount` - The credits to deduct.
    /// * `reserve` - The credits that must remain after the deduction.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The low level operations that perform the write.
    /// * `Err(DriveError::PrefundedSpecializedBalanceNotEnough)` when the fund minus the
    ///   reserve cannot cover the amount.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    pub fn deduct_from_readiness_fund_operations(
        &self,
        fund_id: Identifier,
        amount: Credits,
        reserve: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .readiness
            .deduct_from_fund
        {
            Some(0) => self.deduct_from_readiness_fund_operations_v0(
                fund_id,
                amount,
                reserve,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "deduct_from_readiness_fund_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "deduct_from_readiness_fund_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
