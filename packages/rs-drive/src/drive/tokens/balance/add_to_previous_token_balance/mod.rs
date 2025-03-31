mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::balances::credits::TokenAmount;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations for adding a certain amount of credits to an identity's balance. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the Token.
    /// * `identity_id` - The ID of the Identity to which credits are to be added.
    /// * `balance_to_add` - The amount of credits to be added to the identity's balance.
    /// * `block_info` - Information about the current block.
    /// * `apply` - A boolean indicating whether the operation should be applied or not.
    /// * `transaction` - The transaction information related to the operation.
    /// * `platform_version` - The platform version.
    /// * `previous_fee_versions` - Cached fee versions, if any.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The resulting fee result if successful, or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn add_to_identity_token_balance(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_add: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .token
            .update
            .add_to_identity_token_balance
        {
            0 => self.add_to_identity_token_balance_v0(
                token_id,
                identity_id,
                balance_to_add,
                block_info,
                apply,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_identity_token_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds a specified amount of credits to an identity balance. This function checks for overflows and ensures the balance does not exceed `MAX_CREDITS`.
    /// Balances are stored under the identity's ID in the token balance tree.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the Token.
    /// * `identity_id` - The ID of the Identity to which credits are to be added.
    /// * `balance_to_add` - The amount of credits to be added to the identity's balance.
    /// * `estimated_costs_only_with_layer_info` - Estimated costs with layer information, if any.
    /// * `transaction` - The transaction information related to the operation.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - The resulting low level drive operations if successful, or an error.
    pub(crate) fn add_to_identity_token_balance_operations(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        balance_to_add: TokenAmount,
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
            .update
            .add_to_identity_token_balance
        {
            0 => self.add_to_identity_token_balance_operations_v0(
                token_id,
                identity_id,
                balance_to_add,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_identity_token_balance_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
