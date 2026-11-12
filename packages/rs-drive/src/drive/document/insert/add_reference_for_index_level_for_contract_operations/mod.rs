mod v0;
mod v1;

use crate::util::estimated_layers::EstimatedLayers;
use crate::util::storage_flags::StorageFlags;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::{DocumentAndContractInfo, PathInfo};
use dpp::version::PlatformVersion;

use dpp::data_contract::document_type::IndexLevelTypeInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Adds the terminal reference.
    #[allow(clippy::too_many_arguments)]
    pub fn add_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        // Takes `&IndexLevelTypeInfo` (was `IndexLevelTypeInfo` by
        // value back when the struct was `Copy`). The `summable:
        // Option<String>` field added in v3 forced dropping `Copy`,
        // and the call sites all hand us a borrow from
        // `IndexLevel::has_index_with_type()` — pass it through
        // without cloning.
        index_type: &IndexLevelTypeInfo,
        any_fields_null: bool,
        all_fields_null: bool,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        // A full `PlatformVersion` (was `&DriveVersion`): the indexOnly
        // terminal branch reads the member key off the document via
        // `Document::get_raw_for_document_type`, which is
        // platform-versioned. Pure signature widening — the drive-version
        // dispatch below is unchanged.
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_reference_for_index_level_for_contract_operations
        {
            0 => self.add_reference_for_index_level_for_contract_operations_v0(
                document_and_contract_info,
                index_path_info,
                index_type,
                any_fields_null,
                all_fields_null,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            // v1 (protocol version 15): an estimate describes a summable index's entries for a
            // value as references with sum items; history references follow the primary pointer.
            1 => self.add_reference_for_index_level_for_contract_operations_v1(
                document_and_contract_info,
                index_path_info,
                index_type,
                any_fields_null,
                all_fields_null,
                previous_batch_operations,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_reference_for_index_level_for_contract_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::document::index_level_tree_types::terminal_member_tree_type;
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::estimated_layers::EstimatedLayers;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::platform_value::platform_value;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb::batch::key_info::KeyInfo::KnownKey;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
    use grovedb::EstimatedLayerInformation;
    use grovedb::EstimatedLayerSizes::{AllReference, AllReferencesWithSumItem};
    use grovedb::{GroveDb, TreeType};

    /// Under the tip jar's summable `byRecipient` index the entries for a recipient are
    /// references with sum items, which GroveDB prices 10 bytes dearer per node than plain
    /// references: an insert's estimate describes them so from protocol version 15, and as plain
    /// references at protocol version 14.
    #[test]
    fn should_describe_a_summable_index_value_entries_with_their_sum_items_from_protocol_version_15(
    ) {
        for (platform_version, entries) in [
            // a reference to a document of a type keeping no history is 42 bytes
            (
                PlatformVersion::get(14).expect("expected protocol version 14"),
                AllReference(32, 42, None),
            ),
            (
                PlatformVersion::latest(),
                AllReferencesWithSumItem(32, 42, None),
            ),
        ] {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));
            let contract = json_document_to_contract(
                "tests/supporting_files/contract/tip-jar/tip-jar-contract.json",
                false,
                platform_version,
            )
            .expect("expected the contract");
            let document_type = contract
                .document_type_for_name("tip")
                .expect("expected the tip document type");
            let document = document_type
                .random_document(Some(5), platform_version)
                .expect("expected a tip");

            let mut path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "tip");
            path.push(b"recipient".to_vec());
            path.push(
                document
                    .get_raw_for_document_type("recipient", document_type, None, platform_version)
                    .expect("expected the recipient")
                    .expect("expected the recipient to be set"),
            );
            let index_type = document_type
                .index_structure()
                .sub_levels()
                .get("recipient")
                .expect("expected the recipient level")
                .has_index_with_type()
                .expect("expected the byRecipient index to end here");

            let mut estimate = Some(EstimatedLayers::new());
            drive
                .add_reference_for_index_level_for_contract_operations(
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
                    &mut None,
                    &None,
                    &mut estimate,
                    None,
                    &mut vec![],
                    platform_version,
                )
                .expect("expected the reference insert");

            path.push(vec![0]);
            assert_eq!(
                estimate
                    .expect("expected the estimate")
                    .get(&KeyInfoPath::from_known_owned_path(path))
                    .copied(),
                Some(EstimatedLayerInformation {
                    tree_type: TreeType::SumTree,
                    estimated_layer_count: PotentiallyAtMaxElements,
                    estimated_layer_sizes: entries,
                }),
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }

    /// The history layout changes the unique-index lookup allowance from 44 to
    /// 42 bytes. A sum item adds no allowance for a create's existence check:
    /// the applied lookup is on an absent key and loads zero bytes.
    #[test]
    fn should_size_unique_index_create_lookups_for_the_active_history_layout() {
        for version in [PlatformVersion::get(14).unwrap(), PlatformVersion::latest()] {
            for history in [false, true] {
                let mut loaded = Vec::new();
                for summable in [false, true] {
                    let mut index = platform_value!({
                        "name": "byTag", "properties": [{"tag": "asc"}], "unique": true
                    });
                    if summable {
                        index.insert("summable".into(), "amount".into()).unwrap();
                    }
                    let contract = DataContractFactory::new(version.protocol_version)
                        .unwrap()
                        .create_with_value_config(
                            [7; 32].into(), 0,
                            platform_value!({"note": {
                                "type": "object", "documentsKeepHistory": history,
                                "documentsMutable": true, "canBeDeleted": false,
                                "indices": [index],
                                "properties": {
                                    "tag": {"type": "string", "maxLength": 20, "position": 0},
                                    "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 1}
                                },
                                "required": ["tag", "amount"], "additionalProperties": false
                            }}), None, None,
                        ).unwrap().data_contract_owned();
                    let dt = contract.document_type_for_name("note").unwrap();
                    let document = dt.random_document(Some(5), version).unwrap();
                    let index_type = dt
                        .index_structure()
                        .sub_levels()
                        .get("tag")
                        .unwrap()
                        .has_index_with_type()
                        .unwrap();
                    let mut path =
                        contract_document_type_path_vec(contract.id_ref().as_bytes(), "note");
                    path.push(b"tag".to_vec());
                    path.push(
                        document
                            .get_raw_for_document_type("tag", dt, None, version)
                            .unwrap()
                            .unwrap(),
                    );
                    // Sum trees encode aggregate bytes in their nodes independently
                    // of the reference. Remove that envelope to compare the allowance.
                    let node_bytes = GroveDb::average_case_for_has_raw(
                        &KeyInfoPath::from_known_owned_path(path.clone()),
                        &KnownKey(vec![0]),
                        0,
                        terminal_member_tree_type(index_type),
                        &version.drive.grove_version,
                    )
                    .unwrap()
                    .storage_loaded_bytes;
                    let drive = setup_drive_with_initial_state_structure(Some(version));
                    let mut operations = vec![];
                    drive
                        .add_reference_for_index_level_for_contract_operations(
                            &DocumentAndContractInfo {
                                owned_document_info: OwnedDocumentInfo {
                                    document_info: DocumentRefInfo((&document, None)),
                                    owner_id: None,
                                },
                                contract: &contract,
                                document_type: dt,
                            },
                            PathInfo::PathAsVec(path),
                            index_type,
                            false,
                            false,
                            &mut None,
                            &None,
                            &mut Some(EstimatedLayers::new()),
                            None,
                            &mut operations,
                            version,
                        )
                        .unwrap();
                    let loaded_bytes: u64 = operations
                        .iter()
                        .filter_map(|op| match op {
                            LowLevelDriveOperation::CalculatedCostOperation(cost) => {
                                Some(cost.storage_loaded_bytes)
                            }
                            _ => None,
                        })
                        .sum();
                    let reference_bytes = loaded_bytes - node_bytes;
                    let expected = if history && version.protocol_version == 14 {
                        44
                    } else {
                        42
                    };
                    assert_eq!(reference_bytes, expected,
                        "unique lookup reference allowance at PV{} history={history} summable={summable}", version.protocol_version);
                    loaded.push(reference_bytes);
                }
                assert_eq!(
                    loaded[1], loaded[0],
                    "unique-check sum allowance delta at PV{} history={history}",
                    version.protocol_version
                );
            }
        }
    }
}
