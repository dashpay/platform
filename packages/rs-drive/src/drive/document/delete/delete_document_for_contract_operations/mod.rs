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

    /// Deletes `document`, a document a create consumes, which the create's `refersTo` lookup
    /// read with its element's `storage_flags`, without its type's `canBeDeleted` guard and
    /// without reading it again: the operations
    /// [`Self::force_delete_document_for_contract_operations`] gives, less its read. Only the
    /// create conversion 1 (protocol version 15) deletes through it, with what document
    /// reference validation 1 read.
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
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use crate::util::test_helpers::setup_contract;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::fee::fee_result::FeeResult;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerInformation;
    use std::borrow::Cow;
    use std::collections::HashMap;

    /// What a delete gives: its operations and, when it only estimates costs, the layer
    /// information it estimated
    type DeleteOutcome = (
        Vec<LowLevelDriveOperation>,
        Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
    );

    /// Deleting a stored `person` under `platform_version`: by its id, as the force delete
    /// does, and from the document and storage flags already read, each with a layer
    /// information map of its own when `estimate` is set. Gives the drive holding it too.
    fn force_delete_by_id_and_from_read(
        platform_version: &PlatformVersion,
        estimate: bool,
    ) -> (Drive, DeleteOutcome, DeleteOutcome) {
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

        let mut by_id_estimates = estimate.then(HashMap::new);
        let by_id = drive
            .force_delete_document_for_contract_operations(
                document.id(),
                &contract,
                document_type,
                None,
                &mut by_id_estimates,
                0,
                None,
                platform_version,
            )
            .expect("expected the force delete operations");
        let mut from_read_estimates = estimate.then(HashMap::new);
        let from_read = drive
            .force_delete_read_document_for_contract_operations(
                document.clone(),
                Some(storage_flags),
                &contract,
                document_type,
                None,
                &mut from_read_estimates,
                0,
                None,
                platform_version,
            )
            .expect("expected the read-document delete operations");
        (
            drive,
            (by_id, by_id_estimates),
            (from_read, from_read_estimates),
        )
    }

    /// How many layers `estimates` describes
    fn layer_count(estimates: &Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>) -> usize {
        estimates.as_ref().map_or(0, HashMap::len)
    }

    /// The fee of `operations`, costs estimated from `estimates` when it is set, as an
    /// estimated delete is charged
    fn fee_of(
        drive: &Drive,
        operations: Vec<LowLevelDriveOperation>,
        estimates: Option<HashMap<KeyInfoPath, EstimatedLayerInformation>>,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        let mut drive_operations = vec![];
        drive
            .apply_batch_low_level_drive_operations(
                estimates,
                None,
                operations,
                &mut drive_operations,
                &platform_version.drive,
            )
            .expect("expected to cost the operations");
        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &Epoch::new(0).expect("expected epoch 0"),
            drive.config.epochs_per_era,
            platform_version,
            None,
        )
        .expect("expected the fee")
    }

    /// A consumed document is deleted from what its lookup read
    #[test]
    fn should_delete_a_read_document_as_a_force_delete_does_less_its_read() {
        let (_, (by_id, _), (from_read, _)) =
            force_delete_by_id_and_from_read(PlatformVersion::latest(), false);
        // The force delete reads the document first; every operation after that read is
        // the same, the storage removed and refunded included
        assert_eq!(by_id.len(), from_read.len() + 1);
        assert_eq!(&by_id[1..], from_read.as_slice());
    }

    /// The estimate of a consumed document's delete is the force
    /// delete's, less its read: as many layers, and the type's estimated size stands for the
    /// document in both. The estimated operations and layers hold made-up keys, so they are
    /// compared by what they cost.
    #[test]
    fn should_estimate_a_read_document_delete_as_a_force_delete_less_its_read() {
        let platform_version = PlatformVersion::latest();
        let (drive, (mut by_id, by_id_estimates), (from_read, from_read_estimates)) =
            force_delete_by_id_and_from_read(platform_version, true);
        assert_eq!(by_id.len(), from_read.len() + 1);
        assert_matches!(
            by_id.remove(0),
            LowLevelDriveOperation::CalculatedCostOperation(_)
        );
        assert_eq!(
            layer_count(&by_id_estimates),
            layer_count(&from_read_estimates)
        );
        assert!(layer_count(&by_id_estimates) > 0);
        assert_eq!(
            fee_of(&drive, by_id, by_id_estimates, platform_version),
            fee_of(&drive, from_read, from_read_estimates, platform_version)
        );
    }

    /// Deletes a document of `document_type_name` in the contract at `contract_path` from
    /// what was read, at the latest protocol version, estimating costs only when `estimate`
    /// is set. The type is refused before anything is read, so the contract is not stored
    /// and the document is empty.
    fn force_delete_read_document_of_type(
        contract_path: &str,
        document_type_name: &str,
        estimate: bool,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected the contract");
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        drive.force_delete_read_document_for_contract_operations(
            Document::V0(DocumentV0::default()),
            None,
            &contract,
            document_type,
            None,
            &mut estimate.then(HashMap::new),
            0,
            None,
            platform_version,
        )
    }

    /// An indexOnly type has no stored document to delete, as for the force delete by id
    #[test]
    fn should_refuse_to_delete_a_read_index_only_document() {
        for estimate in [false, true] {
            assert_matches!(
                force_delete_read_document_of_type(
                    "tests/supporting_files/contract/index-only-scalar-terminal/index-only-scalar-terminal-contract.json",
                    "answer",
                    estimate,
                ),
                Err(Error::Drive(DriveError::CorruptedCodeExecution(_))),
                "estimating costs only: {estimate}"
            );
        }
    }

    /// A type that keeps history can not be deleted, as for the force delete by id
    #[test]
    fn should_refuse_to_delete_a_read_document_that_keeps_history() {
        for estimate in [false, true] {
            assert_matches!(
                force_delete_read_document_of_type(
                    "tests/supporting_files/contract/family/family-contract-with-history.json",
                    "person",
                    estimate,
                ),
                Err(Error::Drive(
                    DriveError::InvalidDeletionOfDocumentThatKeepsHistory(_)
                )),
                "estimating costs only: {estimate}"
            );
        }
    }
}
