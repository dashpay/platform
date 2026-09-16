mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Destroys a token issuer: marks its lifecycle record wiped and moves its supply rollup
    /// into the destroyed supply ledger, in one batch that touches neither a holder nor a
    /// token. A contract that issues no tokens gets a wiped record with a zero rollup, so its
    /// destruction is recorded the same way. Destroying an issuer twice is an error.
    ///
    /// This is the storage primitive only. No state transition and no block event calls it:
    /// the governance outcome that applies a wipe arrives with the token path validation,
    /// the query and proof binding and the bounded cleanup that make a destroyed issuer's
    /// tokens unusable, so on a network at this version no issuer is destroyed until every
    /// path honours the record.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The issuer to destroy.
    /// * `block_info` - The block the destruction happens in; recorded in the wipe marker.
    /// * `apply` - Whether to write state (`true`) or only estimate the cost (`false`).
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The fee of the write.
    /// * `Err(DriveError::TokenIssuerAlreadyDestroyed)` when the record is already wiped.
    /// * `Err(DriveError::VersionNotActive)` on a platform version without the ledger.
    pub fn destroy_token_issuer(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .token
            .lifecycle
            .destroy_token_issuer
        {
            Some(0) => self.destroy_token_issuer_v0(
                contract_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "destroy_token_issuer".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "destroy_token_issuer".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Applies the destruction and appends its cost to `drive_operations`.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The issuer to destroy.
    /// * `block_info` - The block the destruction happens in.
    /// * `apply` - Whether to write state (`true`) or only estimate the cost (`false`).
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The accumulator the cost is appended to.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the batch is applied or priced.
    pub fn destroy_token_issuer_add_to_operations(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .token
            .lifecycle
            .destroy_token_issuer
        {
            Some(0) => self.destroy_token_issuer_add_to_operations_v0(
                contract_id,
                block_info,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "destroy_token_issuer_add_to_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "destroy_token_issuer_add_to_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations of the destruction without applying them.
    ///
    /// # Parameters
    ///
    /// * `contract_id` - The issuer to destroy.
    /// * `block_info` - The block the destruction happens in.
    /// * `estimated_costs_only_with_layer_info` - `Some` to price the write without state.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * The batch operations.
    pub fn destroy_token_issuer_operations(
        &self,
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .lifecycle
            .destroy_token_issuer
        {
            Some(0) => self.destroy_token_issuer_operations_v0(
                contract_id,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "destroy_token_issuer_operations".to_string(),
                known_versions: vec![0],
            })),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "destroy_token_issuer_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
