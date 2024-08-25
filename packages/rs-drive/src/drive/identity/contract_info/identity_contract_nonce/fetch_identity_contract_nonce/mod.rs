mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{IdentityNonce, Revision};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's contract document nonce from the backing store
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - Identity Id to fetch.
    /// * `contract_id` - For Contract Id to fetch.
    /// * `apply` - If `true`, the changes are applied, otherwise only the cost is estimated.
    /// * `transaction` - Transaction arguments.
    /// * `platform_version` - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option` for the Identity's revision, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn fetch_identity_contract_nonce(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityNonce>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .attributes
            .identity_contract_nonce
        {
            0 => self.fetch_identity_contract_nonce_v0(
                identity_id,
                contract_id,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contract_nonce".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates the operations to get Identity's contract nonce from the backing store.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - Identity Id to fetch.
    /// * `contract_id` - For Contract Id to fetch.
    /// * `apply` - If `true`, the changes are applied, otherwise only the cost is estimated.
    /// * `transaction` - Transaction arguments.
    /// * `drive_operations` - A mutable reference to a vector of low-level drive operations.
    /// * `platform_version` - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option` for the Identity's revision, otherwise an `Error` if the operation fails or the version is not supported.
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub(crate) fn fetch_identity_contract_nonce_operations(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Revision>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .attributes
            .identity_contract_nonce
        {
            0 => self.fetch_identity_contract_nonce_operations_v0(
                identity_id,
                contract_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contract_nonce_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Identity's contract nonce from the backing store with its associated fees.
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - Identity Id to fetch.
    /// * `contract_id` - For Contract Id to fetch.
    /// * `block_info` - Information about the block.
    /// * `apply` - If `true`, the changes are applied, otherwise only the cost is estimated.
    /// * `transaction` - Transaction arguments.
    /// * `platform_version` - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option` for the Identity's revision and the `FeeResult`, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn fetch_identity_contract_nonce_with_fees(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<IdentityNonce>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .attributes
            .identity_contract_nonce
        {
            0 => self.fetch_identity_contract_nonce_with_fees_v0(
                identity_id,
                contract_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_contract_nonce_with_fees".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
