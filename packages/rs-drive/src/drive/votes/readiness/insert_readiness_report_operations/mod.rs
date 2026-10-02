mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::readiness::report_record::ReadinessReportRecord;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Inserts one accepted readiness report into a round's count tree if absent. A report
    /// already present (a retransmission) is a no-op that returns `false`; the count tree's
    /// count therefore stays the number of distinct reporters.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract.
    /// * `round_id` - The round; the caller has checked it is the contract's current round.
    /// * `pro_tx_hash` - The reporting evonode.
    /// * `record` - What to store for the report.
    /// * `estimated_costs_only_with_layer_info` - `Some` to estimate instead of read state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * Whether the report was new and the low level operations that perform the insert.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without readiness.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_readiness_report_operations(
        &self,
        contract_id: [u8; 32],
        round_id: [u8; 32],
        pro_tx_hash: [u8; 32],
        record: &ReadinessReportRecord,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, Vec<LowLevelDriveOperation>), Error> {
        match platform_version.drive.methods.vote.readiness.insert_report {
            Some(0) => self.insert_readiness_report_operations_v0(
                contract_id,
                round_id,
                pro_tx_hash,
                record,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "insert_readiness_report_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_readiness_report_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
