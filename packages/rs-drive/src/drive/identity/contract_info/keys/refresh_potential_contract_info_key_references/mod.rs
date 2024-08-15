use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::identity::IdentityPublicKey;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

mod v0;

impl Drive {
    /// Adds potential contract information for a contract-bounded key.
    ///
    /// This function considers the contract bounds associated with an identity key and forms the operations needed to process the contract information.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - An array of bytes representing the identity id.
    /// * `identity_key` - A reference to the `IdentityPublicKey` associated with the contract.
    /// * `epoch` - The current epoch.
    /// * `estimated_costs_only_with_layer_info` - A mutable reference to an optional `HashMap` that may contain estimated layer information.
    /// * `transaction` - The transaction arguments.
    /// * `drive_operations` - A mutable reference to a vector of `LowLevelDriveOperation` objects.
    /// * `platform_version` - A reference to the platform version information.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If successful, returns unit (`()`). If an error occurs during the operation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` if the operation creation process fails or if the platform version does not match any of the implemented method versions.
    pub(crate) fn refresh_potential_contract_info_key_references(
        &self,
        identity_id: [u8; 32],
        identity_key: &IdentityPublicKey,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .contract_info
            .refresh_potential_contract_info_key_references
        {
            0 => self.refresh_potential_contract_info_key_references_v0(
                identity_id,
                identity_key,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "refresh_potential_contract_info_key_references".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
