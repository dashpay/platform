use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::identity::PartialIdentity;
use grovedb::TransactionArg;

use dpp::version::PlatformVersion;

impl Drive {
    /// Fetches the Identity's balance as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_with_balance_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PartialIdentity>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        Ok(self
            .fetch_identity_balance_operations(
                identity_id,
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .map(|balance| PartialIdentity {
                id: Identifier::new(identity_id),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,

                not_found_public_keys: Default::default(),
            }))
    }

    /// Fetches the Identity's balance as PartialIdentityInfo from the backing store
    /// Passing apply as false get the estimated cost instead
    pub(super) fn fetch_identity_with_balance_with_cost_v0(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<PartialIdentity>, FeeResult), Error> {
        let balance_cost = platform_version
            .fee_version
            .processing
            .fetch_identity_balance_processing_cost;
        if !apply {
            return Ok((None, FeeResult::new_from_processing_fee(balance_cost)));
        }
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        Ok((
            self.fetch_identity_balance_operations(
                identity_id,
                apply,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .map(|balance| PartialIdentity {
                id: Identifier::new(identity_id),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,

                not_found_public_keys: Default::default(),
            }),
            FeeResult::new_from_processing_fee(balance_cost),
        ))
    }
}
