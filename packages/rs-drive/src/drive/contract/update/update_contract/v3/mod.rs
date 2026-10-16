use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};

impl Drive {
    /// Generation 3 differs from generation 2 in one thing: when the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Earlier generations applied the batch (committing it on its own without a caller
    /// transaction) and priced it afterwards, so from protocol version 15, where pricing an
    /// owner-attributed storage removal without the fee history is an error, a call passing
    /// no history persisted the write and then failed. It now fails before anything is
    /// written, and the contract cache stays on its committed path: the rewritten copy
    /// replaces the global entry only after the commit. With a caller transaction nothing is
    /// committed by Drive in either generation and the block cache is used as before.
    #[inline(always)]
    pub(super) fn update_contract_v3(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        if !apply {
            return self.insert_contract(
                contract,
                block_info,
                false,
                transaction,
                platform_version,
            );
        }
        // The contract cache keys its block-versus-committed behaviour on whether a
        // transaction is present. An owned transaction is not block execution, so cache
        // reads and writes below stay on the committed path (`caller_transaction`), and
        // the rewritten copy is seeded only once the owned transaction has committed.
        let caller_transaction = transaction;
        let owned_transaction = transaction
            .is_none()
            .then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(caller_transaction);

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_bytes = contract.serialize_to_bytes_with_platform_version(platform_version)?;

        // Since we can update the contract by definition it already has storage flags
        let storage_flags = Some(StorageFlags::new_single_epoch(
            block_info.epoch.index,
            Some(contract.owner_id().to_buffer()),
        ));

        let contract_element = Element::Item(
            contract_bytes,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        let original_contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract.id().to_buffer(),
                Some(&block_info.epoch),
                true,
                caller_transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        if original_contract_fetch_info.contract.config().readonly() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                "original contract is readonly",
            )));
        }

        self.update_contract_element(
            contract_element,
            contract,
            &original_contract_fetch_info.contract,
            &block_info,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        // Update DataContracts cache with the new contract
        let updated_contract_fetch_info = self
            .fetch_contract_and_add_operations(
                contract.id().to_buffer(),
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        // A pricing error drops the owned transaction with everything it wrote, and the
        // cache never learns of the rewrite.
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }

        // Update DataContracts cache with the new contract
        self.cache
            .data_contracts
            .insert_rewritten(updated_contract_fetch_info, caller_transaction.is_some());
        Ok(fees)
    }
}
