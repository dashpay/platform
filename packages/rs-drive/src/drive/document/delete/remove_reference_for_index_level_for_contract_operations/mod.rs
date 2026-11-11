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
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::estimated_layers::EstimatedLayers;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::document_type::{DocumentTypeRef, IndexLevelTypeInfo};
    use dpp::data_contract::DataContract;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::platform_value::platform_value;
    use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
    use grovedb::EstimatedLayerInformation;
    use grovedb::EstimatedLayerSizes::{AllReference, AllReferencesWithSumItem, AllSubtrees};
    use grovedb::EstimatedSumTrees::NoSumTrees;
    use grovedb::TreeType;

    /// Removes `document`'s reference from the index of `document_type_name` on `properties`
    /// for an estimate, every layer the delete climbs through described before, as the index
    /// walkers do before they reach the reference. `adjust_index_type` may change the index's
    /// level info first. Returns the estimate, the removal's operations, and the index path the
    /// reference sits under.
    fn estimate_reference_removal(
        contract: &DataContract,
        document_type_name: &str,
        properties: &[&str],
        document: impl Fn(DocumentTypeRef) -> Document,
        adjust_index_type: impl Fn(&mut IndexLevelTypeInfo),
        platform_version: &PlatformVersion,
    ) -> (EstimatedLayers, Vec<LowLevelDriveOperation>, Vec<Vec<u8>>) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        let document = document(document_type);

        let mut path =
            contract_document_type_path_vec(contract.id_ref().as_bytes(), document_type_name);
        let mut index_level = document_type.index_structure();
        let mut estimate = EstimatedLayers::new();
        let described = EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(1, false),
            estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
        };
        estimate.insert(KeyInfoPath::from_known_owned_path(path.clone()), described);
        for property in properties {
            index_level = index_level
                .sub_levels()
                .get(*property)
                .expect("expected the index level");
            path.push(property.as_bytes().to_vec());
            estimate.insert(KeyInfoPath::from_known_owned_path(path.clone()), described);
            path.push(
                document
                    .get_raw_for_document_type(property, document_type, None, platform_version)
                    .expect("expected the property's value")
                    .expect("expected the property to be set"),
            );
            estimate.insert(KeyInfoPath::from_known_owned_path(path.clone()), described);
        }
        let mut index_type = index_level
            .has_index_with_type()
            .expect("expected the index to end here")
            .clone();
        adjust_index_type(&mut index_type);

        let mut estimate = Some(estimate);
        let mut operations = vec![];
        drive
            .remove_reference_for_index_level_for_contract_operations(
                &DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                PathInfo::PathAsVec(path.clone()),
                &index_type,
                false,
                false,
                &None,
                &None,
                &mut estimate,
                false,
                [0; 32],
                None,
                &mut operations,
                platform_version,
            )
            .expect("expected the reference removal");
        (estimate.expect("expected the estimate"), operations, path)
    }

    /// The description a delete's estimate gives the `[0]` layer of the index of
    /// `document_type_name` on `properties` (its entries for `document`'s values, keyed by
    /// document id).
    fn entries_layer(
        contract_path: &str,
        document_type_name: &str,
        properties: &[&str],
        document: impl Fn(DocumentTypeRef) -> Document,
        platform_version: &PlatformVersion,
    ) -> EstimatedLayerInformation {
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected the contract");
        let (estimate, _, mut path) = estimate_reference_removal(
            &contract,
            document_type_name,
            properties,
            document,
            |_| {},
            platform_version,
        );
        path.push(vec![0]);
        estimate
            .get(&KeyInfoPath::from_known_owned_path(path))
            .copied()
            .expect("expected the entries layer to be described")
    }

    /// A delete's estimate describes the `[0]` layer of the `person` type's non-unique
    /// `firstName`, `middleName` index as the references the insert writes there from protocol
    /// version 15, and as subtrees at protocol version 14.
    #[test]
    fn should_describe_an_index_value_entries_as_references_from_protocol_version_15() {
        for (platform_version, entries) in [
            (
                PlatformVersion::get(14).expect("expected protocol version 14"),
                AllSubtrees(32, NoSumTrees, None),
            ),
            // a reference to a document of a type keeping no history is 42 bytes
            (PlatformVersion::latest(), AllReference(32, 42, None)),
        ] {
            let layer = entries_layer(
                "tests/supporting_files/contract/family/family-contract-reduced.json",
                "person",
                &["firstName", "middleName"],
                |document_type| {
                    json_document_to_document(
                        "tests/supporting_files/contract/family/person0.json",
                        Some([3u8; 32].into()),
                        document_type,
                        platform_version,
                    )
                    .expect("expected the person document")
                },
                platform_version,
            );
            assert_eq!(
                layer,
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: entries,
                },
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    /// Under the tip jar's summable `byRecipient` index the entries are references with sum
    /// items, which GroveDB prices 10 bytes dearer per node than plain references: a delete's
    /// estimate describes them so from protocol version 15, and as subtrees at protocol version
    /// 14.
    #[test]
    fn should_describe_a_summable_index_value_entries_with_their_sum_items_from_protocol_version_15(
    ) {
        for (platform_version, entries) in [
            (
                PlatformVersion::get(14).expect("expected protocol version 14"),
                AllSubtrees(32, NoSumTrees, None),
            ),
            (
                PlatformVersion::latest(),
                AllReferencesWithSumItem(32, 42, None),
            ),
        ] {
            let layer = entries_layer(
                "tests/supporting_files/contract/tip-jar/tip-jar-contract.json",
                "tip",
                &["recipient"],
                |document_type| {
                    document_type
                        .random_document(Some(5), platform_version)
                        .expect("expected a tip")
                },
                platform_version,
            );
            assert_eq!(
                layer,
                EstimatedLayerInformation {
                    tree_type: TreeType::SumTree,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: entries,
                },
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    /// A unique index keeps a document's reference at its value's key `[0]`, under a summable
    /// index as a reference with a sum item, 10 bytes larger. From protocol version 15 a
    /// delete's estimate reads the deleted reference at that size, 10 bytes more than the same
    /// delete with the index's `summable` cleared; protocol version 14 reads it as a plain
    /// reference either way.
    #[test]
    fn should_read_a_unique_summable_index_deleted_reference_with_its_sum_item_from_protocol_version_15(
    ) {
        for (platform_version, sum_item_bytes) in [
            (
                PlatformVersion::get(14).expect("expected protocol version 14"),
                0,
            ),
            (PlatformVersion::latest(), 10),
        ] {
            let contract = DataContractFactory::new(platform_version.protocol_version)
                .expect("expected a contract factory")
                .create_with_value_config(
                    Identifier::from([7; 32]),
                    0,
                    platform_value!({
                        "receipt": {
                            "type": "object",
                            "indices": [{
                                "name": "byTxId",
                                "properties": [{ "txId": "asc" }],
                                "unique": true,
                                "summable": "amount",
                            }],
                            "properties": {
                                "txId": {
                                    "type": "array",
                                    "byteArray": true,
                                    "minItems": 32,
                                    "maxItems": 32,
                                    "position": 0,
                                },
                                "amount": {
                                    "type": "integer",
                                    "minimum": 1,
                                    "maximum": 4294967295u64,
                                    "position": 1,
                                },
                            },
                            "required": ["txId", "amount"],
                            "additionalProperties": false,
                        },
                    }),
                    None,
                    None,
                )
                .expect("expected the contract")
                .data_contract_owned();
            let loaded_bytes = |summable: bool| -> u64 {
                let (_, operations, _) = estimate_reference_removal(
                    &contract,
                    "receipt",
                    &["txId"],
                    |document_type| {
                        document_type
                            .random_document(Some(5), platform_version)
                            .expect("expected a receipt")
                    },
                    |index_type| {
                        assert_eq!(index_type.summable.as_deref(), Some("amount"));
                        if !summable {
                            index_type.summable = None;
                        }
                    },
                    platform_version,
                );
                operations
                    .iter()
                    .filter_map(|operation| match operation {
                        LowLevelDriveOperation::CalculatedCostOperation(cost) => {
                            Some(cost.storage_loaded_bytes)
                        }
                        _ => None,
                    })
                    .sum()
            };
            assert_eq!(
                loaded_bytes(true) - loaded_bytes(false),
                sum_item_bytes,
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }
}
