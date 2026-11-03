mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Reads the moderation action counts of the elected contract `contract_id`: for each
    /// member of the seated team who signed a counted moderation action since the moderators
    /// pot was last settled, how many. At most `limit` of them, in identity id order.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract.
    /// * `limit`: At most this many counts.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(BTreeMap<Identifier, u32>)` with the counts.
    /// * `Err(Error)` when the version is unknown, a read fails or a count is malformed.
    pub fn fetch_contract_moderation_action_counts(
        &self,
        contract_id: Identifier,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, u32>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_moderation_action_counts
        {
            0 => self.fetch_contract_moderation_action_counts_add_to_operations_v0(
                contract_id,
                limit,
                transaction,
                &mut vec![],
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_moderation_action_counts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// [`Drive::fetch_contract_moderation_action_counts`] with the fee of the read, so that
    /// consensus validation can bill it.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract.
    /// * `limit`: At most this many counts.
    /// * `epoch`: The epoch the fee is priced for.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, BTreeMap<Identifier, u32>))` with the fee and the counts.
    /// * `Err(Error)` when the version is unknown, a read fails or a count is malformed.
    pub fn fetch_contract_moderation_action_counts_with_fee(
        &self,
        contract_id: Identifier,
        limit: u16,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, BTreeMap<Identifier, u32>), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_moderation_action_counts
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let counts = self.fetch_contract_moderation_action_counts_add_to_operations_v0(
                    contract_id,
                    limit,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let fee = Drive::calculate_fee(
                    None,
                    Some(drive_operations),
                    epoch,
                    self.config.epochs_per_era,
                    platform_version,
                    None,
                )?;
                Ok((fee, counts))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_moderation_action_counts_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Reads one member's moderation action count on the elected contract `contract_id`, with
    /// the fee of the read: 0 when the member signed no counted action since the moderators pot
    /// was last settled, `None` when the contract has no counts tree (an elected contract
    /// stored before the counts existed, on a development network), in which case its team's
    /// actions are not counted. The read of every count likewise finds none there.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The elected contract.
    /// * `identity_id`: The member.
    /// * `epoch`: The epoch the fee is priced for.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, Option<u32>))` with the fee and the count, `None` without a counts
    ///   tree.
    /// * `Err(Error)` when the version is unknown, the read fails or the count is malformed.
    pub fn fetch_contract_moderation_action_count_with_fee(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<u32>), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_moderation_action_counts
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let count = self.fetch_contract_moderation_action_count_add_to_operations_v0(
                    contract_id,
                    identity_id,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let fee = Drive::calculate_fee(
                    None,
                    Some(drive_operations),
                    epoch,
                    self.config.epochs_per_era,
                    platform_version,
                    None,
                )?;
                Ok((fee, count))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_moderation_action_count_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
