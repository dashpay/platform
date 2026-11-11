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
            // value as references with sum items.
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
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::util::estimated_layers::EstimatedLayers;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
    use grovedb::EstimatedLayerInformation;
    use grovedb::EstimatedLayerSizes::{AllReference, AllReferencesWithSumItem};
    use grovedb::TreeType;

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
}
