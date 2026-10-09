mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Prepares the operations for deleting a document.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract`: The contract that contains the document.
    /// * `document_type`: The type of the document.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn delete_document_for_contract_operations(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        if estimated_costs_only_with_layer_info.is_none() {
            self.prepare_document_time_range_ttl(
                contract,
                document_type,
                block_time_ms,
                transaction,
                platform_version,
            )?;
        }
        self.delete_document_for_contract_operations_without_ttl_drain(
            document_id,
            contract,
            document_type,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            block_time_ms,
            transaction,
            platform_version,
        )
    }

    /// Build against post-drain state. The caller must prepare the whole
    /// batch before invoking this method; no cleanup occurs during conversion.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn delete_document_for_contract_operations_without_ttl_drain(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_operations
        {
            0 => self.delete_document_for_contract_operations_v0(
                document_id,
                contract,
                document_type,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Prepares the operations for deleting a document and will delete even if the contract does
    /// not allow for deletion, this is reserved for system data contracts.
    ///
    /// # Parameters
    /// * `document_id`: The ID of the document to delete.
    /// * `contract`: The contract that contains the document.
    /// * `document_type`: The type of the document.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn force_delete_document_for_contract_operations(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_operations
        {
            0 => self.force_delete_document_for_contract_operations_v0(
                document_id,
                contract,
                document_type,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "force_delete_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Deletes `document`, already read with its element's `storage_flags`, without its
    /// type's `canBeDeleted` guard and without reading it again: the operations
    /// [`Self::force_delete_document_for_contract_operations`] gives, less its read. For a
    /// document a create consumes, which the create's `refersTo` lookup read.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn force_delete_read_document_for_contract_operations(
        &self,
        document: Document,
        storage_flags: Option<StorageFlags>,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .delete_document_for_contract_operations
        {
            0 => self.force_delete_read_document_for_contract_operations_v0(
                document,
                storage_flags,
                contract,
                document_type,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "force_delete_read_document_for_contract_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::setup_contract;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::document::DocumentV0Getters;
    use dpp::tests::json_document::json_document_to_document;
    use dpp::version::PlatformVersion;
    use std::borrow::Cow;

    #[test]
    fn should_delete_a_read_document_as_a_force_delete_does_less_its_read() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );
        let document_type = contract
            .document_type_for_name("person")
            .expect("expected the person document type");
        let owner_id = [3u8; 32];
        let document = json_document_to_document(
            "tests/supporting_files/contract/family/person0.json",
            Some(owner_id.into()),
            document_type,
            platform_version,
        )
        .expect("expected the person document");
        let storage_flags = StorageFlags::new_single_epoch(0, Some(owner_id));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            Some(Cow::Borrowed(&storage_flags)),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to store the document");

        let by_id = drive
            .force_delete_document_for_contract_operations(
                document.id(),
                &contract,
                document_type,
                None,
                &mut None,
                0,
                None,
                platform_version,
            )
            .expect("expected the force delete operations");
        let from_read = drive
            .force_delete_read_document_for_contract_operations(
                document.clone(),
                Some(storage_flags),
                &contract,
                document_type,
                None,
                &mut None,
                0,
                None,
                platform_version,
            )
            .expect("expected the read-document delete operations");

        // The force delete reads the document first; every operation after that read is
        // the same, the storage removed and refunded included
        assert_eq!(by_id.len(), from_read.len() + 1);
        assert_eq!(&by_id[1..], from_read.as_slice());
    }
}
