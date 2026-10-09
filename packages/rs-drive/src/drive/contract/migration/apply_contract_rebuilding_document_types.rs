use crate::drive::{contract_documents_path, Drive};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::version::PlatformVersion;
use grovedb::operations::delete::DeleteOptions;
use grovedb::{Element, Transaction};
use std::borrow::Cow;

impl Drive {
    /// Re-stores `contract` over the stored version of the same contract, as `apply_contract`
    /// does, and rebuilds each document type `rebuilt_document_types` names from nothing:
    /// its whole subtree, every document and index entry under it, is deleted, and the update
    /// then creates the type as it creates a type a contract update adds, which is how a
    /// contract registration creates it too. The rebuilt types end up holding no documents,
    /// laid out exactly as on a chain that registered `contract` directly.
    ///
    /// For a protocol upgrade that changes a system contract's document type in a way a
    /// contract update cannot follow, such as replacing a unique index; the documents deleted
    /// are not refunded. Runs once, on the first block of the protocol version that ships the
    /// new contract (DPNS v3's `preorder` at protocol version 14).
    pub fn apply_contract_rebuilding_document_types(
        &self,
        contract: &DataContract,
        rebuilt_document_types: &[&str],
        block_info: &BlockInfo,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let contract_id = contract.id().to_buffer();
        let stored_contract = self
            .fetch_contract_and_add_operations(
                contract_id,
                None,
                Some(transaction),
                &mut vec![],
                platform_version,
            )?
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "a contract is rebuilt over its stored version, which must exist",
                ))
            })?
            .contract
            .clone();

        // The update compares against the stored contract without the rebuilt types, so it
        // creates each of them as a type the update adds
        let mut original_contract = stored_contract;
        for document_type_name in rebuilt_document_types {
            if original_contract
                .document_types_mut()
                .remove(*document_type_name)
                .is_none()
                || !contract.has_document_type_for_name(document_type_name)
            {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a rebuilt document type must be in both the stored and the new contract",
                )));
            }
            self.grove
                .delete(
                    contract_documents_path(&contract_id).as_ref(),
                    document_type_name.as_bytes(),
                    Some(DeleteOptions {
                        allow_deleting_non_empty_trees: true,
                        deleting_non_empty_trees_returns_error: false,
                        ..Default::default()
                    }),
                    Some(transaction),
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(Error::from)?;
        }

        let contract_element = Element::Item(
            contract.serialize_to_bytes_with_platform_version(platform_version)?,
            StorageFlags::map_cow_to_some_element_flags(storage_flags),
        );
        let mut drive_operations = vec![];
        self.update_contract_add_operations(
            contract_element,
            contract,
            &original_contract,
            block_info,
            &mut None,
            Some(transaction),
            &mut drive_operations,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            None,
            Some(transaction),
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
