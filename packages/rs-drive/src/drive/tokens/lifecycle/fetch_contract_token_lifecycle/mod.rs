mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::tokens::contract_lifecycle::ContractTokenLifecycle;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches a contract's token lifecycle record: its supply rollup and, once the issuer
    /// is destroyed, its wipe marker.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract whose record is read.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(None)` when the contract issues no tokens (or the ledger predates it).
    /// * `Err(DriveError::VersionNotActive)` on a platform version without the ledger.
    pub fn fetch_contract_token_lifecycle(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractTokenLifecycle>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .lifecycle
            .fetch_contract_token_lifecycle
        {
            Some(0) => {
                self.fetch_contract_token_lifecycle_v0(contract_id, transaction, platform_version)
            }
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_contract_token_lifecycle".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_token_lifecycle".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches a contract's token lifecycle record and accumulates the cost of the read.
    /// With `apply = false` the read is priced without touching state and returns `None`.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The contract whose record is read.
    /// * `apply` - Whether to read state (`true`) or only estimate the cost (`false`).
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The operations vector to accumulate the cost into.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The record, or `None` when absent or when only estimating.
    pub fn fetch_contract_token_lifecycle_operations(
        &self,
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractTokenLifecycle>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .lifecycle
            .fetch_contract_token_lifecycle
        {
            Some(0) => self.fetch_contract_token_lifecycle_operations_v0(
                contract_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_contract_token_lifecycle_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_token_lifecycle_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
