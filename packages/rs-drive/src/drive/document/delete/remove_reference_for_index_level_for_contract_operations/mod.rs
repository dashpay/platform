mod v0;
mod v1;
mod v2;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};
use crate::util::storage_flags::StorageFlags;

use dpp::version::PlatformVersion;

use dpp::data_contract::document_type::IndexLevelTypeInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Removes the terminal reference.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: The document and contract info.
    /// * `index_path_info`: Path info for the index.
    /// * `unique`: Whether the reference is unique.
    /// * `any_fields_null`: Whether any fields are null.
    /// * `storage_flags`: Optional storage flags.
    /// * `previous_batch_operations`: Previous batch operations to include.
    /// * `estimated_costs_only_with_layer_info`: Estimated costs with layer info.
    /// * `event_id`: The event ID.
    /// * `transaction`: The transaction argument.
    /// * `batch_operations`: The batch operations to include.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(())` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        // Borrow rather than owned — see the matching insert-path
        // wrapper for the rationale (IndexLevelTypeInfo dropped Copy
        // when `summable: Option<String>` was added in v3).
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        skip_missing_expired_entry: bool,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .delete
            .remove_reference_for_index_level_for_contract_operations
        {
            0 => self.remove_reference_for_index_level_for_contract_operations_v0(
                document_and_contract_info,
                index_path_info,
                index_type,
                any_fields_null,
                all_fields_null,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            // v1 (protocol version 14): the pruning climb stops at the
            // member level on `preallocated` indexOnly indexes.
            1 => self.remove_reference_for_index_level_for_contract_operations_v1(
                document_and_contract_info,
                index_path_info,
                index_type,
                any_fields_null,
                all_fields_null,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                skip_missing_expired_entry,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            // v2 (protocol version 15): an estimate describes the index's entries for a value
            // as the references they are.
            2 => self.remove_reference_for_index_level_for_contract_operations_v2(
                document_and_contract_info,
                index_path_info,
                index_type,
                any_fields_null,
                all_fields_null,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                skip_missing_expired_entry,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_reference_for_index_level_for_contract_operations".to_string(),
                known_versions: vec![0, 1, 2],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::util::estimated_layers::EstimatedLayers;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::EstimatedLevel;
    use grovedb::EstimatedLayerInformation;
    use grovedb::EstimatedLayerSizes::{AllReference, AllSubtrees};
    use grovedb::EstimatedSumTrees::NoSumTrees;
    use grovedb::TreeType;

    /// A delete's estimate describes the `[0]` layer of the `person` type's non-unique
    /// `firstName`, `middleName` index (its entries for the document's values, keyed by
    /// document id) as the references the insert writes there from protocol version 15, and as
    /// subtrees at protocol version 14.
    #[test]
    fn should_describe_an_index_value_entries_as_references_from_protocol_version_15() {
        for (platform_version, as_references) in [
            (
                PlatformVersion::get(14).expect("expected protocol version 14"),
                false,
            ),
            (PlatformVersion::latest(), true),
        ] {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));
            let contract = json_document_to_contract(
                "tests/supporting_files/contract/family/family-contract-reduced.json",
                false,
                platform_version,
            )
            .expect("expected the contract");
            let document_type = contract
                .document_type_for_name("person")
                .expect("expected the person document type");
            let document = json_document_to_document(
                "tests/supporting_files/contract/family/person0.json",
                Some([3u8; 32].into()),
                document_type,
                platform_version,
            )
            .expect("expected the person document");

            let mut path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "person");
            let mut index_level = document_type.index_structure();
            // Every layer the delete climbs through is described, as the index walkers do
            // before they reach the reference.
            let mut estimate = EstimatedLayers::new();
            estimate.insert(
                KeyInfoPath::from_known_owned_path(path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: EstimatedLevel(1, false),
                    estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
                },
            );
            for property in ["firstName", "middleName"] {
                index_level = index_level
                    .sub_levels()
                    .get(property)
                    .expect("expected the index level");
                path.push(property.as_bytes().to_vec());
                estimate.insert(
                    KeyInfoPath::from_known_owned_path(path.clone()),
                    EstimatedLayerInformation {
                        tree_type: TreeType::NormalTree,
                        estimated_layer_count: EstimatedLevel(1, false),
                        estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
                    },
                );
                path.push(
                    document
                        .get_raw_for_document_type(property, document_type, None, platform_version)
                        .expect("expected the property's value")
                        .expect("expected the property to be set"),
                );
                estimate.insert(
                    KeyInfoPath::from_known_owned_path(path.clone()),
                    EstimatedLayerInformation {
                        tree_type: TreeType::NormalTree,
                        estimated_layer_count: EstimatedLevel(1, false),
                        estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
                    },
                );
            }
            let index_type = index_level
                .has_index_with_type()
                .expect("expected the index to end here");

            let mut estimate = Some(estimate);
            drive
                .remove_reference_for_index_level_for_contract_operations(
                    &DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    PathInfo::PathAsVec(path.clone()),
                    index_type,
                    false,
                    false,
                    &None,
                    &None,
                    &mut estimate,
                    false,
                    [0; 32],
                    None,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected the reference removal");

            path.push(vec![0]);
            let entries = estimate
                .expect("expected the estimate")
                .get(&KeyInfoPath::from_known_owned_path(path))
                .copied()
                .expect("expected the entries layer to be described");
            assert_eq!(
                matches!(entries.estimated_layer_sizes, AllReference(..)),
                as_references,
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }
}
