use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::identity::identity_contract_info_group_path;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::IdentityNonce;

use crate::drive::identity::contract_info::ContractInfoStructure::IdentityContractNonceKey;
use dpp::version::PlatformVersion;
use grovedb::Element::Item;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's contract revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_contract_nonce_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityNonce>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_contract_nonce_operations_v0(
            identity_id,
            contract_id,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Creates the operations to get Identity's contract revision from the backing store
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(super) fn fetch_identity_contract_nonce_operations_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<IdentityNonce>, Error> {
        let direct_query_type = if apply {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(1),
            }
        };
        let identity_contract_path =
            identity_contract_info_group_path(&identity_id, contract_id.as_slice());
        match self.grove_get_raw_optional(
            (&identity_contract_path).into(),
            &[IdentityContractNonceKey as u8],
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        ) {
            Ok(Some(Item(encoded_nonce, _))) => {
                let nonce =
                    IdentityNonce::from_be_bytes(encoded_nonce.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "identity contract nonce was not 8 bytes as expected",
                        ))
                    })?);

                Ok(Some(nonce))
            }

            Ok(None) => Ok(None),

            Ok(Some(..)) => Err(Error::Drive(DriveError::CorruptedElementType(
                "identity contract nonce was present but was not identified as an item",
            ))),

            Err(e) => Err(e),
        }
    }

    /// Fetches the Identity's contract revision from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_contract_nonce_with_fees_v0(
        &self,
        identity_id: [u8; 32],
        contract_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<IdentityNonce>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identity_contract_nonce_operations_v0(
            identity_id,
            contract_id,
            apply,
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
}
