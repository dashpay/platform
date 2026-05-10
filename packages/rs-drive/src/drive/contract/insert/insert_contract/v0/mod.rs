use crate::drive::contract::paths;

use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::{contract_documents_path, votes, Drive, RootTree};
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::storage_flags::StorageFlags;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::serialization::PlatformSerializableWithPlatformVersion;

use crate::drive::votes::paths::{
    CONTESTED_DOCUMENT_INDEXES_TREE_KEY, CONTESTED_DOCUMENT_STORAGE_TREE_KEY,
};
use crate::error::contract::DataContractError;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::{HashMap, HashSet};

impl Drive {
    /// Insert a contract.
    #[inline(always)]
    pub(super) fn insert_contract_v0(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = if contract.config().can_be_deleted() || !contract.config().readonly() {
            Some(StorageFlags::new_single_epoch(
                block_info.epoch.index,
                Some(contract.owner_id().to_buffer()),
            ))
        } else {
            None
        };

        let serialized_contract =
            contract.serialize_to_bytes_with_platform_version(platform_version)?;

        if serialized_contract.len() as u64 > u32::MAX as u64
            || serialized_contract.len() as u32
                > platform_version.dpp.contract_versions.max_serialized_size
        {
            // This should normally be caught by DPP, but there is a rare possibility that the
            // re-serialized size is bigger than the original serialized data contract.
            return Err(Error::DataContract(DataContractError::ContractTooBig(format!("Trying to insert a data contract of size {} that is over the max allowed insertion size {}", serialized_contract.len(), platform_version.dpp.contract_versions.max_serialized_size))));
        }

        let contract_element = Element::Item(
            serialized_contract,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        self.insert_contract_element_v0(
            contract_element,
            contract,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// Adds a contract to storage using `add_contract_to_storage`
    /// and inserts the empty trees which will be necessary to later insert documents.
    #[allow(clippy::too_many_arguments)]
    fn insert_contract_element_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.insert_contract_operations_v0(
            contract_element,
            contract,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    #[inline(always)]
    pub(super) fn insert_contract_add_operations_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v0(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    pub(in crate::drive::contract::insert::insert_contract) fn insert_contract_operations_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = StorageFlags::map_some_element_flags_ref(contract_element.get_flags())?;

        self.batch_insert_empty_tree(
            [Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).as_slice()],
            KeyRef(contract.id_ref().as_bytes()),
            storage_flags.as_ref(),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        self.add_contract_to_storage(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
            true,
            None, // we are not inserting into history, hence the transaction will not be used, we can pass None
            &platform_version.drive,
        )?;

        // the documents
        let contract_root_path = paths::contract_root_path(contract.id_ref().as_bytes());
        let key_info = Key(vec![1]);
        self.batch_insert_empty_tree(
            contract_root_path,
            key_info,
            storage_flags.as_ref(),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        // If the contract happens to contain any contested indexes then we add the contract to the
        //  contested contracts

        let document_types_with_contested_indexes =
            contract.document_types_with_contested_indexes();

        if !document_types_with_contested_indexes.is_empty() {
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_contested_document_tree_levels_up_to_contract(
                    contract,
                    None,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            let contested_contract_root_path =
                votes::paths::vote_contested_resource_active_polls_tree_path();

            self.batch_insert_empty_tree(
                contested_contract_root_path,
                KeyRef(contract.id_ref().as_bytes()),
                storage_flags.as_ref(),
                &mut batch_operations,
                &platform_version.drive,
            )?;

            let contested_unique_index_contract_document_types_path =
                votes::paths::vote_contested_resource_active_polls_contract_tree_path(
                    contract.id_ref().as_bytes(),
                );

            for (type_key, _document_type) in document_types_with_contested_indexes.into_iter() {
                self.batch_insert_empty_tree(
                    contested_unique_index_contract_document_types_path,
                    KeyRef(type_key.as_bytes()),
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                let type_path = [
                    contested_unique_index_contract_document_types_path[0],
                    contested_unique_index_contract_document_types_path[1],
                    contested_unique_index_contract_document_types_path[2],
                    contested_unique_index_contract_document_types_path[3],
                    type_key.as_bytes(),
                ];

                // primary key tree
                let key_info_storage = Key(vec![CONTESTED_DOCUMENT_STORAGE_TREE_KEY]);
                self.batch_insert_empty_tree(
                    type_path,
                    key_info_storage,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                // index key tree
                let key_info_indexes = Key(vec![CONTESTED_DOCUMENT_INDEXES_TREE_KEY]);
                self.batch_insert_empty_tree(
                    type_path,
                    key_info_indexes,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        // next we should store each document type
        // right now we are referring them by name
        // todo: maybe change this to be a reference by index
        let contract_documents_path = contract_documents_path(contract.id_ref().as_bytes());

        for (type_key, document_type) in contract.document_types().iter() {
            self.batch_insert_empty_tree(
                contract_documents_path,
                KeyRef(type_key.as_bytes()),
                storage_flags.as_ref(),
                &mut batch_operations,
                &platform_version.drive,
            )?;

            let type_path = [
                contract_documents_path[0],
                contract_documents_path[1],
                contract_documents_path[2],
                type_key.as_bytes(),
            ];

            // primary key tree — route through the centralized
            // primary_key_tree_type() so contract creation, document inserts,
            // deletes, and estimation paths all see the same tree-variant
            // selection (under whichever drive method version is active).
            let key_info = Key(vec![0]);
            match document_type
                .as_ref()
                .primary_key_tree_type(platform_version)?
            {
                TreeType::ProvableCountTree => self.batch_insert_empty_provable_count_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                TreeType::CountTree => self.batch_insert_empty_count_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                _ => self.batch_insert_empty_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
            }

            let mut index_cache: HashSet<&[u8]> = HashSet::new();
            let document_type_ref = document_type.as_ref();
            let index_structure = document_type_ref.index_structure();
            // for each type we should insert the indices that are top level
            for index in document_type.as_ref().top_level_indices() {
                // toDo: change this to be a reference by index
                let index_bytes = index.name.as_bytes();
                if !index_cache.contains(index_bytes) {
                    // If a range_countable index terminates at this top
                    // level (i.e. a single-property index over `index.name`
                    // with range_countable: true), the property-name tree
                    // must be a `ProvableCountTree` so range-count queries
                    // over the property's distinct values can use grovedb's
                    // `AggregateCountOnRange`. Otherwise it's a NormalTree.
                    let property_name_is_range_countable_terminator = index_structure
                        .sub_levels()
                        .get(index.name.as_str())
                        .and_then(|level| level.has_index_with_type())
                        .map(|info| info.range_countable)
                        .unwrap_or(false);
                    if property_name_is_range_countable_terminator {
                        self.batch_insert_empty_provable_count_tree(
                            type_path,
                            KeyRef(index_bytes),
                            storage_flags.as_ref(),
                            &mut batch_operations,
                            &platform_version.drive,
                        )?;
                    } else {
                        self.batch_insert_empty_tree(
                            type_path,
                            KeyRef(index_bytes),
                            storage_flags.as_ref(),
                            &mut batch_operations,
                            &platform_version.drive,
                        )?;
                    }
                    index_cache.insert(index_bytes);
                }
            }
        }

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_insertion(
                contract,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(batch_operations)
    }
}

#[cfg(test)]
mod countable_e2e_tests {
    //! End-to-end coverage for `documentsCountable` / `rangeCountable`.
    //!
    //! These tests exercise the full feature path:
    //!   - Build a v12 contract with the flag set in the schema.
    //!   - Apply it to a real Drive (grovedb).
    //!   - Read the primary-key tree element back from grove and assert the
    //!     concrete tree variant (NormalTree / CountTree / ProvableCountTree)
    //!     matches what the schema requested.
    //!   - For the count variants, insert and delete documents and assert the
    //!     tree's internal count moves accordingly.

    use crate::drive::Drive;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::{platform_value, Value};
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::version::PlatformVersion;
    use grovedb::{Element, GroveDb, PathTrunkChunkQuery};

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// Builds a v12 `DataContract` whose single `widget` document type has
    /// `documentsCountable` / `rangeCountable` set to the requested values.
    fn build_widget_contract(
        documents_countable: bool,
        range_countable: bool,
    ) -> dpp::prelude::DataContract {
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

        let mut document_schema = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "position": 0,
                    "maxLength": 64,
                }
            },
            "additionalProperties": false,
        });
        if documents_countable {
            document_schema.as_map_mut().unwrap().push((
                Value::Text("documentsCountable".to_string()),
                Value::Bool(true),
            ));
        }
        if range_countable {
            document_schema
                .as_map_mut()
                .unwrap()
                .push((Value::Text("rangeCountable".to_string()), Value::Bool(true)));
        }

        let schemas = platform_value!({ "widget": document_schema });
        let owner_id = generate_random_identifier_struct();

        factory
            .create_with_value_config(owner_id, 0, schemas, None, None)
            .expect("expected to create data contract")
            .data_contract_owned()
    }

    /// Reads the primary-key tree element directly from grove and returns it.
    fn read_primary_key_tree(
        drive: &Drive,
        contract: &dpp::prelude::DataContract,
        document_type_name: &str,
    ) -> Element {
        let pv = PlatformVersion::latest();
        let contract_id = contract.id();
        let path: [&[u8]; 4] = [
            &[crate::drive::RootTree::DataContractDocuments as u8],
            contract_id.as_bytes(),
            &[1],
            document_type_name.as_bytes(),
        ];
        drive
            .grove_get_raw(
                (&path).into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected grove_get_raw to succeed")
            .expect("primary key tree element should exist")
    }

    fn primary_key_tree_path(
        contract: &dpp::prelude::DataContract,
        document_type_name: &str,
    ) -> Vec<Vec<u8>> {
        vec![
            vec![crate::drive::RootTree::DataContractDocuments as u8],
            contract.id().as_bytes().to_vec(),
            vec![1],
            document_type_name.as_bytes().to_vec(),
            vec![0],
        ]
    }

    #[test]
    fn default_contract_creates_normal_tree_for_primary_key() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(false, false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let elem = read_primary_key_tree(&drive, &contract, "widget");
        assert!(
            matches!(elem, Element::Tree(..)),
            "default (non-countable) contract should use a NormalTree primary key tree, got {:?}",
            elem
        );
    }

    #[test]
    fn documents_countable_contract_creates_count_tree_for_primary_key() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(true, false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let elem = read_primary_key_tree(&drive, &contract, "widget");
        match &elem {
            Element::CountTree(_, count, _) => {
                assert_eq!(*count, 0, "freshly inserted CountTree should have count 0");
            }
            other => panic!(
                "documentsCountable contract should use a CountTree primary key tree, got {:?}",
                other
            ),
        }

        // Sanity: the parsed DocumentTypeV2 also reports the flag.
        let dt = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let dt_owned = dt.to_owned_document_type();
        match dt_owned {
            dpp::data_contract::document_type::DocumentType::V2(v2) => {
                assert!(v2.documents_countable());
                assert!(!v2.range_countable());
            }
            other => panic!("expected DocumentType::V2 on protocol v12, got {:?}", other),
        }
    }

    #[test]
    fn range_countable_contract_creates_provable_count_tree_for_primary_key() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(false, true);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let elem = read_primary_key_tree(&drive, &contract, "widget");
        assert!(
            matches!(elem, Element::ProvableCountTree(..)),
            "rangeCountable contract should use a ProvableCountTree primary key tree, got {:?}",
            elem
        );

        // rangeCountable implies documents_countable in the parser.
        let dt = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let dt_owned = dt.to_owned_document_type();
        match dt_owned {
            dpp::data_contract::document_type::DocumentType::V2(v2) => {
                assert!(v2.range_countable());
                assert!(v2.documents_countable());
            }
            other => panic!("expected DocumentType::V2 on protocol v12, got {:?}", other),
        }
    }

    #[test]
    fn count_tree_count_grows_and_shrinks_with_document_inserts_and_deletes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(true, false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // Insert 3 documents.
        let mut doc_ids = vec![];
        for seed in 1u64..=3 {
            let document = document_type
                .random_document(Some(seed), pv)
                .expect("random document");
            doc_ids.push(document.id());

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let elem_after_inserts = read_primary_key_tree(&drive, &contract, "widget");
        match elem_after_inserts {
            Element::CountTree(_, count, _) => {
                assert_eq!(count, 3, "count tree should track 3 inserted documents");
            }
            other => panic!("expected CountTree, got {:?}", other),
        }

        // Delete one.
        drive
            .delete_document_for_contract(
                doc_ids[0],
                &contract,
                "widget",
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to delete document");

        let elem_after_delete = read_primary_key_tree(&drive, &contract, "widget");
        match elem_after_delete {
            Element::CountTree(_, count, _) => {
                assert_eq!(count, 2, "count tree should drop to 2 after one delete");
            }
            other => panic!("expected CountTree, got {:?}", other),
        }
    }

    #[test]
    fn provable_count_tree_count_grows_with_document_inserts() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(false, true);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for seed in 1u64..=5 {
            let document = document_type
                .random_document(Some(seed), pv)
                .expect("random document");

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let elem = read_primary_key_tree(&drive, &contract, "widget");
        match elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(count, 5, "provable count tree should track 5 documents");
            }
            other => panic!("expected ProvableCountTree, got {:?}", other),
        }
    }

    #[test]
    fn range_countable_primary_key_tree_supports_trunk_proof() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(false, true);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for seed in 1u64..=20 {
            let document = document_type
                .random_document(Some(seed), pv)
                .expect("random document");

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let elem = read_primary_key_tree(&drive, &contract, "widget");
        match elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(count, 20, "provable count tree should track inserted docs");
            }
            other => panic!("expected ProvableCountTree, got {:?}", other),
        }

        let query = PathTrunkChunkQuery::new(primary_key_tree_path(&contract, "widget"), 3);
        let proof = drive
            .grove
            .prove_trunk_chunk(&query, &pv.drive.grove_version)
            .value
            .expect("expected trunk proof call to succeed");
        let (root_hash, result) =
            GroveDb::verify_trunk_chunk_proof(&proof, &query, &pv.drive.grove_version)
                .expect("expected trunk proof to verify");

        assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
        assert!(
            !result.elements.is_empty(),
            "trunk proof should return primary-key tree elements"
        );
        assert!(
            result
                .leaf_keys
                .values()
                .any(|leaf_info| leaf_info.count.is_some()),
            "rangeCountable trunk proof should expose subtree counts"
        );
    }

    /// Sanity: existing document fetch + count APIs still work for a CountTree
    /// contract — i.e. switching the underlying primary-key tree variant
    /// does not break document iteration.
    #[test]
    fn count_tree_contract_supports_document_fetch() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(true, false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        let document = document_type
            .random_document(Some(42), pv)
            .expect("random document");
        let inserted_id = document.id();

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document");

        let query =
            crate::query::DriveDocumentQuery::all_items_query(&contract, document_type, None);
        let (docs, _, _) = query
            .execute_raw_results_no_proof(&drive, None, None, pv)
            .expect("expected query to succeed");
        assert_eq!(docs.len(), 1, "should fetch exactly the inserted document");
        let decoded = dpp::document::Document::from_bytes(&docs[0], document_type, pv)
            .expect("expected to decode document");
        assert_eq!(decoded.id(), inserted_id);
    }

    /// Apply a contract with the given countable flags and return the fees
    /// reported by `insert_contract`. Used to compare fee profiles across
    /// the three primary-key tree variants.
    fn fees_for_contract_with(
        documents_countable: bool,
        range_countable: bool,
    ) -> dpp::fee::fee_result::FeeResult {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(documents_countable, range_countable);
        drive
            .insert_contract(&contract, BlockInfo::default(), true, None, pv)
            .expect("expected insert_contract to succeed and return fees")
    }

    /// Switching the primary-key tree variant from NormalTree to CountTree
    /// changes the underlying grovedb element shape (CountTree carries an
    /// extra count value). The reported fees must therefore differ — if they
    /// don't, the contract insert path silently degraded back to the
    /// NormalTree branch and the documentsCountable feature is dead.
    #[test]
    fn count_tree_contract_apply_produces_different_fees_than_normal_tree() {
        let normal_fees = fees_for_contract_with(false, false);
        let count_fees = fees_for_contract_with(true, false);

        assert!(normal_fees.storage_fee > 0, "normal tree storage fee");
        assert!(normal_fees.processing_fee > 0, "normal tree processing fee");
        assert!(count_fees.storage_fee > 0, "count tree storage fee");
        assert!(count_fees.processing_fee > 0, "count tree processing fee");

        assert_ne!(
            (normal_fees.storage_fee, normal_fees.processing_fee),
            (count_fees.storage_fee, count_fees.processing_fee),
            "documentsCountable: true must produce a different fee profile than the default \
             NormalTree contract — equal fees mean the count-tree branch was never exercised"
        );
    }

    /// Same invariant for the rangeCountable / ProvableCountTree branch:
    /// switching from CountTree to ProvableCountTree changes both the grove
    /// element type and the proof shape, so fees must differ.
    #[test]
    fn provable_count_tree_contract_apply_produces_different_fees_than_count_tree() {
        let count_fees = fees_for_contract_with(true, false);
        let provable_fees = fees_for_contract_with(false, true);

        assert!(provable_fees.storage_fee > 0, "provable count storage fee");
        assert!(
            provable_fees.processing_fee > 0,
            "provable count processing fee"
        );

        assert_ne!(
            (count_fees.storage_fee, count_fees.processing_fee),
            (provable_fees.storage_fee, provable_fees.processing_fee,),
            "rangeCountable: true must produce a different fee profile than documentsCountable: \
             true alone — equal fees mean the provable-count-tree branch was never exercised"
        );
    }

    /// Document insert into a CountTree contract should produce positive fees
    /// without error. This exercises the document-insert code paths
    /// (add_document_for_contract_operations, primary-key-tree dispatch in
    /// add_document_to_primary_storage) under the count-tree branch.
    #[test]
    fn document_insert_into_count_tree_produces_positive_fees() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_contract(true, false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let document = document_type
            .random_document(Some(7), pv)
            .expect("random document");

        let fee = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document into count tree");

        assert!(
            fee.storage_fee > 0,
            "document insert into a CountTree contract must produce a positive storage fee"
        );
        assert!(
            fee.processing_fee > 0,
            "document insert into a CountTree contract must produce a positive processing fee"
        );
    }
}

#[cfg(test)]
mod range_countable_index_e2e_tests {
    //! End-to-end coverage for an *indexed* `rangeCountable` property.
    //!
    //! Where `countable_e2e_tests` only checks the document-type-level flag
    //! (`documentsCountable` / `rangeCountable` on the document type, which
    //! drives the primary-key tree variant), this module builds a contract
    //! whose `indices` section contains a `rangeCountable: true` index over
    //! a property and verifies the *index storage tree shape*:
    //!
    //!   - `[contract_doc, doctype, "color"]` is a `ProvableCountTree`
    //!     (created at contract setup).
    //!   - `[..., "color", <c1>]` is a `CountTree` (created on document
    //!     insert by the index walker), whose count tracks how many docs
    //!     have that color value.
    //!   - Sibling continuations under that `CountTree` (compound index
    //!     suffixes) are wrapped with `Element::NonCounted` so they
    //!     contribute 0 to the parent count.

    use crate::drive::Drive;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value::{platform_value, Value};
    use dpp::prelude::DataContract;
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// Build a v12 contract whose `widget` document type has a
    /// `rangeCountable: true` single-property index over `color`. The
    /// optional `compound_index` adds a non-range-countable compound
    /// `[color, size]` index so we can verify NonCounted-wrapping of the
    /// sibling continuation.
    fn build_widget_with_color_index(compound_index: bool) -> DataContract {
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

        let mut indices = vec![platform_value!({
            "name": "byColor",
            "properties": [{"color": "asc"}],
            "countable": "countable",
            "rangeCountable": true,
        })];
        if compound_index {
            indices.push(platform_value!({
                "name": "byColorSize",
                "properties": [{"color": "asc"}, {"size": "asc"}],
            }));
        }

        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {
                    "type": "string",
                    "position": 0,
                    "maxLength": 32,
                },
                "size": {
                    "type": "string",
                    "position": 1,
                    "maxLength": 32,
                },
            },
            "indices": Value::Array(indices),
            "additionalProperties": false,
        });

        let schemas = platform_value!({ "widget": document_schema });
        let owner_id = generate_random_identifier_struct();

        factory
            .create_with_value_config(owner_id, 0, schemas, None, None)
            .expect("expected to create data contract")
            .data_contract_owned()
    }

    /// Two `range_countable` indexes sharing the `color` prefix:
    /// `byColor [color]` and `byColorSize [color, size]`. The shared
    /// prefix exercises the `NonCounted<*>` wrapping rule (book:
    /// indexes.md §"Compound interaction with range_countable") on a
    /// configuration where the wrapped tree itself is a
    /// `ProvableCountTree` rather than a plain `NormalTree` —
    /// stressing the walker's `parent_value_tree_is_range_countable`
    /// flag against a wrapper-target type that the existing single-
    /// doc layout test doesn't reach.
    #[allow(dead_code)]
    fn build_widget_with_two_range_countable_indexes() -> DataContract {
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

        let indices = vec![
            platform_value!({
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }),
            platform_value!({
                "name": "byColorSize",
                "properties": [{"color": "asc"}, {"size": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }),
        ];

        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {
                    "type": "string",
                    "position": 0,
                    "maxLength": 32,
                },
                "size": {
                    "type": "string",
                    "position": 1,
                    "maxLength": 32,
                },
            },
            "indices": Value::Array(indices),
            "additionalProperties": false,
        });

        let schemas = platform_value!({ "widget": document_schema });
        let owner_id = generate_random_identifier_struct();

        factory
            .create_with_value_config(owner_id, 0, schemas, None, None)
            .expect("expected to create data contract")
            .data_contract_owned()
    }

    fn property_name_tree_path(
        contract: &DataContract,
        document_type_name: &str,
        property_name: &str,
    ) -> Vec<Vec<u8>> {
        vec![
            vec![crate::drive::RootTree::DataContractDocuments as u8],
            contract.id().as_bytes().to_vec(),
            vec![1],
            document_type_name.as_bytes().to_vec(),
            property_name.as_bytes().to_vec(),
        ]
    }

    fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Option<Element> {
        let pv = PlatformVersion::latest();
        let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
        drive
            .grove_get_raw(
                path_refs.as_slice().into(),
                key,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("grove_get_raw should succeed")
    }

    fn build_widget_doc(contract: &DataContract, color: &str, size: &str, seed: u64) -> Document {
        let pv = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let mut doc = document_type
            .random_document(Some(seed), pv)
            .expect("random document");
        let mut props = std::collections::BTreeMap::new();
        props.insert("color".to_string(), Value::Text(color.to_string()));
        props.insert("size".to_string(), Value::Text(size.to_string()));
        doc.set_properties(props);
        doc
    }

    /// The top-level property-name tree at `[contract_doc, doctype, "color"]`
    /// must be a `ProvableCountTree` for a contract with a `rangeCountable`
    /// single-property index over `color`. This is the layer that
    /// `AggregateCountOnRange` walks for O(log n) range counts.
    #[test]
    fn property_name_tree_for_range_countable_index_is_provable_count_tree() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let path = property_name_tree_path(&contract, "widget", "color");
        let parent_path: Vec<Vec<u8>> = path[..path.len() - 1].to_vec();
        let key = path.last().unwrap().clone();
        let elem = read_grove_element(&drive, &parent_path, &key)
            .expect("color property-name tree must exist");
        match elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(
                    count, 0,
                    "freshly created property-name ProvableCountTree should have aggregate 0"
                );
            }
            other => panic!(
                "rangeCountable index property-name tree should be ProvableCountTree, got {:?}",
                other
            ),
        }
    }

    /// Inserting a document whose indexed property has value `c1` creates
    /// the value tree at `[contract_doc, doctype, "color", "c1"]`. With
    /// `rangeCountable: true` the walker must lay this down as a
    /// `CountTree` so the parent property-name `ProvableCountTree`'s
    /// aggregate sums per-value counts cleanly.
    #[test]
    fn value_tree_for_range_countable_index_is_count_tree_after_insert() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let doc = build_widget_doc(&contract, "red", "small", 1);

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document");

        // Property-name aggregate should now reflect the inserted doc.
        let property_path = property_name_tree_path(&contract, "widget", "color");
        let prop_parent: Vec<Vec<u8>> = property_path[..property_path.len() - 1].to_vec();
        let prop_key = property_path.last().unwrap().clone();
        let prop_elem = read_grove_element(&drive, &prop_parent, &prop_key)
            .expect("color property-name tree must exist");
        match prop_elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(
                    count, 1,
                    "ProvableCountTree aggregate should be 1 after inserting one doc"
                );
            }
            other => panic!("expected ProvableCountTree, got {:?}", other),
        }

        // Value tree at <c1> should be a CountTree counting the docs with
        // color="red".
        let value_elem = read_grove_element(&drive, &property_path, b"red")
            .expect("value tree for color=red must exist");
        match value_elem {
            Element::CountTree(_, count, _) => {
                assert_eq!(count, 1, "value-tree CountTree should count 1 doc");
            }
            other => panic!(
                "rangeCountable value tree should be a CountTree, got {:?}",
                other
            ),
        }
    }

    /// Walking the same property's IndexLevel for a *compound* sibling
    /// index `[color, size]` requires the walker to insert a continuation
    /// property-name tree under the `CountTree` value tree. That
    /// continuation must be wrapped with `Element::NonCounted` so it
    /// contributes 0 to the value tree's count — otherwise the count
    /// would be `1 (reference) + 1 (continuation NormalTree) = 2` per
    /// inserted doc instead of the correct `1`.
    #[test]
    fn count_tree_value_count_excludes_compound_continuation_via_non_counted() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(true);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let doc = build_widget_doc(&contract, "red", "small", 1);

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&doc, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to insert document");

        // CountTree count must be exactly 1 (the doc reference), even
        // though there's a compound continuation tree inserted as a
        // sibling. If NonCounted-wrapping is broken, count will be 2 (or
        // more, depending on how the [0] tree contributes).
        let property_path = property_name_tree_path(&contract, "widget", "color");
        let value_elem = read_grove_element(&drive, &property_path, b"red")
            .expect("value tree for color=red must exist");
        match value_elem {
            Element::CountTree(_, count, _) => {
                assert_eq!(
                    count, 1,
                    "CountTree count should equal exactly the number of docs with color=red, \
                     not including the compound-index continuation tree (NonCounted wrapping \
                     check)"
                );
            }
            other => panic!("expected CountTree, got {:?}", other),
        }

        // The compound continuation property-name tree at [..., "color",
        // "red", "size"] should exist and be wrapped with NonCounted.
        let mut size_path = property_path.clone();
        size_path.push(b"red".to_vec());
        let size_elem = read_grove_element(&drive, &size_path, b"size")
            .expect("compound continuation tree at 'size' must exist");
        match size_elem {
            Element::NonCounted(inner) => match inner.as_ref() {
                Element::Tree(_, _) => {} // expected: NonCounted<NormalTree>
                other => panic!(
                    "expected NonCounted<NormalTree>, got NonCounted<{:?}>",
                    other
                ),
            },
            other => panic!(
                "compound continuation under a CountTree must be NonCounted-wrapped, got {:?}",
                other
            ),
        }
    }

    /// Deleting a document under a `range_countable` index must decrement
    /// the value tree's `CountTree` and the parent property-name tree's
    /// `ProvableCountTree` aggregate. If the delete walker doesn't see
    /// the right tree variants in cost estimation, removals can leave
    /// stale references or over-bill the operation; this test pins the
    /// observable outcome (counts after delete).
    #[test]
    fn delete_decrements_count_tree_and_provable_count_aggregate() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // Insert two docs at color="red" so we can delete one and watch
        // the count drop from 2 → 1 (instead of 1 → 0, which is also
        // correct but doesn't distinguish "decrement" from "tree
        // collapsed").
        let doc1 = build_widget_doc(&contract, "red", "small", 1);
        let doc2 = build_widget_doc(&contract, "red", "large", 2);
        for doc in [&doc1, &doc2] {
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let property_path = property_name_tree_path(&contract, "widget", "color");

        // Sanity: 2 docs, both red.
        let value_elem =
            read_grove_element(&drive, &property_path, b"red").expect("value tree exists");
        match value_elem {
            Element::CountTree(_, count, _) => assert_eq!(count, 2),
            other => panic!("expected CountTree, got {:?}", other),
        }

        drive
            .delete_document_for_contract(
                doc1.id(),
                &contract,
                "widget",
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to delete document");

        let prop_parent: Vec<Vec<u8>> = property_path[..property_path.len() - 1].to_vec();
        let prop_key = property_path.last().unwrap().clone();
        let prop_elem =
            read_grove_element(&drive, &prop_parent, &prop_key).expect("property-name tree exists");
        match prop_elem {
            Element::ProvableCountTree(_, count, _) => assert_eq!(
                count, 1,
                "ProvableCountTree aggregate should drop to 1 after one delete"
            ),
            other => panic!("expected ProvableCountTree, got {:?}", other),
        }
        let value_elem =
            read_grove_element(&drive, &property_path, b"red").expect("value tree exists");
        match value_elem {
            Element::CountTree(_, count, _) => assert_eq!(
                count, 1,
                "CountTree count should drop to 1 after one delete"
            ),
            other => panic!("expected CountTree, got {:?}", other),
        }
    }

    /// Inserting multiple docs at the same color value increments the
    /// CountTree, and the aggregate at the property-name
    /// `ProvableCountTree` reflects the total across all values.
    #[test]
    fn aggregate_count_grows_across_distinct_values() {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for (i, color) in ["red", "red", "blue", "green", "green", "green"]
            .iter()
            .enumerate()
        {
            let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let property_path = property_name_tree_path(&contract, "widget", "color");

        // 6 inserts total → ProvableCountTree aggregate = 6
        let prop_parent: Vec<Vec<u8>> = property_path[..property_path.len() - 1].to_vec();
        let prop_key = property_path.last().unwrap().clone();
        let prop_elem =
            read_grove_element(&drive, &prop_parent, &prop_key).expect("property-name tree exists");
        match prop_elem {
            Element::ProvableCountTree(_, count, _) => assert_eq!(count, 6),
            other => panic!("expected ProvableCountTree, got {:?}", other),
        }

        // Per-value counts: red=2, blue=1, green=3
        for (color, expected) in [("red", 2u64), ("blue", 1), ("green", 3)] {
            let value_elem = read_grove_element(&drive, &property_path, color.as_bytes())
                .unwrap_or_else(|| panic!("value tree for color={} must exist", color));
            match value_elem {
                Element::CountTree(_, count, _) => {
                    assert_eq!(count, expected, "color={} CountTree count mismatch", color)
                }
                other => panic!("expected CountTree at color={}, got {:?}", color, other),
            }
        }
    }

    /// End-to-end exercise of the range count executor:
    /// `DriveDocumentCountQuery::execute_range_count_no_proof`. With six
    /// docs at three distinct color values, a `> "blue"` range
    /// should hit `green` (3 docs) and `red` (2 docs) for a total of 5,
    /// and `distinct = true` returns one entry per matching value.
    #[test]
    fn range_count_executor_sums_and_splits_correctly() {
        use crate::query::{
            DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator,
        };

        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for (i, color) in ["red", "red", "blue", "green", "green", "green"]
            .iter()
            .enumerate()
        {
            let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        // Find the range_countable index via the picker so the test
        // doesn't depend on any particular index name.
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: dpp::platform_value::Value::Text("blue".to_string()),
        }];
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("range_countable index should be picked");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: where_clauses.clone(),
        };

        // distinct=false: single summed entry. green(3) + red(2) = 5.
        let summed = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: false,
                    limit: None,
                    start_after_split_key: None,
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(summed.len(), 1);
        assert!(summed[0].key.is_empty(), "summed entry has empty key");
        assert_eq!(
            summed[0].count, 5,
            "color > 'blue' should sum to 3 (green) + 2 (red) = 5"
        );

        // distinct=true: per-value entries, ascending. Should be
        // [(green, 3), (red, 2)] — `blue` is excluded by the
        // exclusive lower bound.
        let split = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: None,
                    start_after_split_key: None,
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].key, b"green".to_vec());
        assert_eq!(split[0].count, 3);
        assert_eq!(split[1].key, b"red".to_vec());
        assert_eq!(split[1].count, 2);

        // distinct=true with limit=1: only the first entry.
        let limited = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: Some(1),
                    start_after_split_key: None,
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].key, b"green".to_vec());

        // distinct=true with start_after_split_key=green: only red.
        let after = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: None,
                    start_after_split_key: Some(b"green".to_vec()),
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].key, b"red".to_vec());

        // distinct=true descending: [(red, 2), (green, 3)].
        let desc = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: None,
                    start_after_split_key: None,
                    order_by_ascending: false,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(desc.len(), 2);
        assert_eq!(desc[0].key, b"red".to_vec());
        assert_eq!(desc[1].key, b"green".to_vec());
    }

    /// `Between [a, b]` is inclusive on both ends — a value at
    /// exactly the lower or upper bound must be counted.
    #[test]
    fn range_count_executor_between_is_inclusive_on_both_bounds() {
        use crate::query::{
            DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator,
        };

        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for (i, color) in ["aaa", "bbb", "ccc", "ddd"].iter().enumerate() {
            let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Between,
            value: dpp::platform_value::Value::Array(vec![
                dpp::platform_value::Value::Text("bbb".to_string()),
                dpp::platform_value::Value::Text("ccc".to_string()),
            ]),
        }];
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("range_countable index should be picked");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses,
        };

        let split = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: None,
                    start_after_split_key: None,
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].key, b"bbb".to_vec());
        assert_eq!(split[0].count, 1);
        assert_eq!(split[1].key, b"ccc".to_vec());
        assert_eq!(split[1].count, 1);
    }

    /// `execute_aggregate_count_with_proof` should produce a grovedb
    /// `AggregateCountOnRange` proof that verifies to the same total
    /// count as the no-proof range walk. This is the prove-path
    /// counterpart of [`range_count_executor_sums_and_splits_correctly`].
    ///
    /// The verification step uses
    /// `GroveDb::verify_aggregate_count_query` directly — proves the
    /// returned bytes are a real proof, not just any blob — and asserts
    /// the recovered count matches the no-proof sum.
    #[test]
    fn aggregate_count_proof_verifies_and_returns_correct_count() {
        use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
        use grovedb::{GroveDb, PathQuery};

        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("expected to apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // Same six-doc fixture as the no-proof test.
        for (i, color) in ["red", "red", "blue", "green", "green", "green"]
            .iter()
            .enumerate()
        {
            let doc = build_widget_doc(&contract, color, "small", (i + 1) as u64);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: dpp::platform_value::Value::Text("blue".to_string()),
        }];
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("range_countable index should be picked");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: where_clauses.clone(),
        };

        let proof_bytes = query
            .execute_aggregate_count_with_proof(&drive, None, pv)
            .expect("should generate aggregate count proof");
        assert!(!proof_bytes.is_empty(), "proof must not be empty");

        // Reconstruct the same path query the prover used, verify the
        // proof against it, and check the recovered count.
        let path = vec![
            vec![crate::drive::RootTree::DataContractDocuments as u8],
            contract.id().as_bytes().to_vec(),
            vec![1u8],
            b"widget".to_vec(),
            b"color".to_vec(),
        ];
        let query_item = grovedb::QueryItem::RangeAfter(b"blue".to_vec()..);
        let path_query = PathQuery::new_aggregate_count_on_range(path, query_item);

        let (root_hash, count) = GroveDb::verify_aggregate_count_query(
            &proof_bytes,
            &path_query,
            &pv.drive.grove_version,
        )
        .expect("aggregate-count proof should verify");
        assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
        assert_eq!(
            count, 5,
            "verified count should match no-proof sum: 3 (green) + 2 (red) = 5"
        );
    }

    /// Range count with an `In` clause on the prefix forks the walk
    /// into one path per prefix value and merges per-key entries.
    /// Uses a compound `[brand, color]` range_countable index — Equal
    /// would also work for one brand value, but `In` exercises the
    /// cartesian fork path that's not covered elsewhere.
    #[test]
    fn range_count_with_in_on_prefix_forks_and_merges() {
        use crate::query::{
            DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator,
        };
        use dpp::platform_value::Value;

        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();

        // Build a contract with `[brand, color]` range_countable.
        let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
            .expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "brand": { "type": "string", "position": 0, "maxLength": 32 },
                "color": { "type": "string", "position": 1, "maxLength": 32 },
            },
            "indices": [{
                "name": "byBrandColor",
                "properties": [{"brand": "asc"}, {"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
            .expect("create contract")
            .data_contract_owned();

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // 3 acme + red, 2 acme + blue, 2 contoso + red, 1 contoso + green.
        let docs: Vec<(&str, &str)> = vec![
            ("acme", "red"),
            ("acme", "red"),
            ("acme", "red"),
            ("acme", "blue"),
            ("acme", "blue"),
            ("contoso", "red"),
            ("contoso", "red"),
            ("contoso", "green"),
        ];
        for (i, (brand, color)) in docs.iter().enumerate() {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), pv)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("brand".to_string(), Value::Text(brand.to_string()));
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("insert");
        }

        // brand IN (acme, contoso) AND color > "blue"
        // Match: acme+red(3), contoso+red(2), contoso+green(1) = 6
        // (Excluded: acme+blue, contoso+blue — but there's no
        //  contoso+blue, just acme+blue which doesn't match.)
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![
                    Value::Text("acme".to_string()),
                    Value::Text("contoso".to_string()),
                ]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("blue".to_string()),
            },
        ];
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("range_countable index should be picked");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses,
        };

        // Distinct mode: per-color entries, summed across both brands.
        // green: 1 (only contoso). red: 3 + 2 = 5. So [(green, 1), (red, 5)].
        let split = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: None,
                    start_after_split_key: None,
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].key, b"green".to_vec());
        assert_eq!(split[0].count, 1);
        assert_eq!(split[1].key, b"red".to_vec());
        assert_eq!(split[1].count, 5);

        // Sum mode: 6 docs total.
        let summed = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: false,
                    limit: None,
                    start_after_split_key: None,
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("range count should succeed");
        assert_eq!(summed.len(), 1);
        assert_eq!(summed[0].count, 6);
    }

    /// `StartsWith` is in the picker's range-operator set but the
    /// executor rejects it because the upper-bound encoding is
    /// key-dependent. The error must surface clearly rather than
    /// silently using a wrong range.
    #[test]
    fn range_count_executor_rejects_starts_with() {
        use crate::query::{
            DriveDocumentCountQuery, RangeCountOptions, WhereClause, WhereOperator,
        };

        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::StartsWith,
            value: dpp::platform_value::Value::Text("re".to_string()),
        }];
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("picker accepts StartsWith");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses,
        };

        let result = query.execute_range_count_no_proof(
            &drive,
            &RangeCountOptions {
                distinct: false,
                limit: None,
                start_after_split_key: None,
                order_by_ascending: true,
            },
            None,
            pv,
        );
        assert!(
            matches!(
                result,
                Err(crate::error::Error::Query(
                    crate::error::query::QuerySyntaxError::InvalidWhereClauseComponents(msg)
                )) if msg.contains("startsWith")
            ),
            "expected startsWith rejection, got {:?}",
            result
        );
    }

    // -------- Aggregate-count prove-path coverage helpers ----------
    //
    // The existing `aggregate_count_proof_verifies_and_returns_correct_count`
    // tests exactly one operator (`>` → grovedb's `RangeAfter`). The
    // remaining 7 mapped operator shapes
    // (`>=`/`<`/`<=`/`between`/`betweenExcludeBounds`/
    // `betweenExcludeLeft`/`betweenExcludeRight`) all generate
    // structurally different `QueryItem` variants and exercise
    // different `Disjoint`/`Contained`/`Boundary` classifications in
    // grovedb's `prove_aggregate_count_on_range` walk. Each is its own
    // potential regression site even though all share the same
    // platform-side path-builder. The helpers + per-operator tests
    // below close that gap.

    /// Single-byColor fixture with 5 distinct color values
    /// (`a`..`e`, two docs each — 10 docs total) so range tests can
    /// land Disjoint, Contained, and Boundary classifications across
    /// the AVL tree without carrying contract setup duplication.
    fn setup_widget_with_5_colors_2_docs_each() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();
        let contract = build_widget_with_color_index(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        let mut seed = 1u64;
        for color in ["a", "b", "c", "d", "e"] {
            for _ in 0..2 {
                let doc = build_widget_doc(&contract, color, "small", seed);
                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentRefInfo((&doc, None)),
                                owner_id: None,
                            },
                            contract: &contract,
                            document_type,
                        },
                        false,
                        BlockInfo::default(),
                        true,
                        None,
                        pv,
                        None,
                    )
                    .expect("expected to insert document");
                seed += 1;
            }
        }

        (drive, contract)
    }

    /// Prove-path roundtrip helper: builds the path query via the
    /// shared `aggregate_count_path_query` (the same path the prover
    /// internally uses), generates the proof, verifies it via
    /// grovedb's `verify_aggregate_count_query`, and asserts the
    /// recovered count equals `expected_count`. Reusing the
    /// path-builder rather than hand-coding the path matches the SDK's
    /// runtime flow — a divergence between prover and verifier
    /// path-construction would surface here as a verification failure.
    fn assert_aggregate_count_proof_returns(
        drive: &Drive,
        contract: &DataContract,
        document_type_name: &str,
        where_clauses: Vec<crate::query::WhereClause>,
        expected_count: u64,
    ) {
        use crate::query::DriveDocumentCountQuery;
        use grovedb::GroveDb;

        let pv = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("document type exists");
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("range_countable index should be picked");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: document_type_name.to_string(),
            index,
            where_clauses,
        };

        let proof_bytes = query
            .execute_aggregate_count_with_proof(drive, None, pv)
            .expect("should generate aggregate count proof");
        assert!(!proof_bytes.is_empty(), "proof must not be empty");

        let path_query = query
            .aggregate_count_path_query(pv)
            .expect("aggregate_count_path_query should build");

        let (root_hash, count) = GroveDb::verify_aggregate_count_query(
            &proof_bytes,
            &path_query,
            &pv.drive.grove_version,
        )
        .expect("aggregate-count proof should verify");
        assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
        assert_eq!(
            count, expected_count,
            "verified count should equal expected count"
        );
    }

    /// `>=` → grovedb `RangeFrom`. Lower bound inclusive, no upper
    /// bound. Differs from `>` (RangeAfter) in whether the bound key
    /// itself contributes — both share the same one-sided-from-below
    /// AVL walk shape so this also serves as the regression for the
    /// inclusivity bit.
    #[test]
    fn aggregate_count_proof_verifies_lower_bound_inclusive_ge() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThanOrEquals,
            value: dpp::platform_value::Value::Text("c".to_string()),
        }];
        // c, d, e each have 2 docs; a, b excluded → 6.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 6);
    }

    /// `<` → grovedb `RangeTo`. Upper bound strict, no lower bound.
    /// Pins the one-sided-from-above walk shape; without this we'd
    /// only ever exercise the symmetric `RangeAfter` half.
    #[test]
    fn aggregate_count_proof_verifies_upper_bound_strict_lt() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::LessThan,
            value: dpp::platform_value::Value::Text("c".to_string()),
        }];
        // a, b each have 2 docs; c, d, e excluded → 4.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
    }

    /// `<=` → grovedb `RangeToInclusive`. Pins the upper-bound
    /// inclusivity bit on the from-above shape.
    #[test]
    fn aggregate_count_proof_verifies_upper_bound_inclusive_le() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::LessThanOrEquals,
            value: dpp::platform_value::Value::Text("c".to_string()),
        }];
        // a, b, c each have 2 docs; d, e excluded → 6.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 6);
    }

    /// `between` → grovedb `RangeInclusive` (closed-closed). The most
    /// common two-sided range shape; both bounds are matched.
    #[test]
    fn aggregate_count_proof_verifies_between_closed_closed() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Between,
            value: dpp::platform_value::Value::Array(vec![
                dpp::platform_value::Value::Text("b".to_string()),
                dpp::platform_value::Value::Text("d".to_string()),
            ]),
        }];
        // b, c, d each have 2 docs → 6.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 6);
    }

    /// `betweenExcludeBounds` → grovedb `RangeAfterTo` (open-open).
    /// Both bounds are excluded — the only `between*` variant where
    /// neither bound key contributes.
    #[test]
    fn aggregate_count_proof_verifies_between_open_open() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::BetweenExcludeBounds,
            value: dpp::platform_value::Value::Array(vec![
                dpp::platform_value::Value::Text("a".to_string()),
                dpp::platform_value::Value::Text("d".to_string()),
            ]),
        }];
        // b, c each have 2 docs (a excluded as lower, d excluded as
        // upper) → 4.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
    }

    /// `betweenExcludeLeft` → grovedb `RangeAfterToInclusive`
    /// (open-closed). Lower excluded, upper included.
    #[test]
    fn aggregate_count_proof_verifies_between_open_closed() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::BetweenExcludeLeft,
            value: dpp::platform_value::Value::Array(vec![
                dpp::platform_value::Value::Text("a".to_string()),
                dpp::platform_value::Value::Text("c".to_string()),
            ]),
        }];
        // b, c each have 2 docs (a excluded as lower) → 4.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
    }

    /// `betweenExcludeRight` → grovedb `Range` (closed-open). Lower
    /// included, upper excluded — the conventional half-open range.
    #[test]
    fn aggregate_count_proof_verifies_between_closed_open() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::BetweenExcludeRight,
            value: dpp::platform_value::Value::Array(vec![
                dpp::platform_value::Value::Text("b".to_string()),
                dpp::platform_value::Value::Text("d".to_string()),
            ]),
        }];
        // b, c each have 2 docs (d excluded as upper) → 4.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 4);
    }

    /// Empty range: zero matching keys must still produce a valid
    /// proof with count = 0. This is the boundary case where every
    /// subtree is `Disjoint` from the inner range — grovedb's prover
    /// short-circuits at every link without descending. The verifier
    /// must accept this proof shape and recover count = 0 (not error
    /// "no items in range"). Without this test a regression that made
    /// empty proofs fail would only surface at customer time.
    #[test]
    fn aggregate_count_proof_verifies_empty_range_returns_zero() {
        use crate::query::{WhereClause, WhereOperator};

        let (drive, contract) = setup_widget_with_5_colors_2_docs_each();
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: dpp::platform_value::Value::Text("z".to_string()),
        }];
        // No colors > "z" — count = 0.
        assert_aggregate_count_proof_returns(&drive, &contract, "widget", where_clauses, 0);
    }

    /// Compound `[brand, color]` range_countable index, prove path:
    /// the `Equal`-on-brand prefix becomes path bytes (not a query
    /// shape), and only the terminator `color > X` becomes the merk
    /// `AggregateCountOnRange` walk. This exercises grovedb#658's
    /// multi-layer envelope where the verifier must walk through one
    /// non-leaf layer (the `brand=acme` value tree's existence proof)
    /// before reaching the leaf merk's count proof. The single-
    /// property tests above all run at the top property-name layer
    /// directly so they don't reach this code path.
    #[test]
    fn aggregate_count_proof_verifies_on_compound_index_with_equal_prefix() {
        use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
        use dpp::platform_value::Value;
        use grovedb::GroveDb;

        let drive = setup_drive_with_initial_state_structure(None);
        let pv = PlatformVersion::latest();

        // Build a contract with `[brand, color]` range_countable.
        // Same shape as `range_count_with_in_on_prefix_forks_and_merges`
        // uses, but here we exercise the prove path instead of the
        // no-proof executor.
        let factory = dpp::data_contract::DataContractFactory::new(PROTOCOL_VERSION_V12)
            .expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "brand": { "type": "string", "position": 0, "maxLength": 32 },
                "color": { "type": "string", "position": 1, "maxLength": 32 },
            },
            "indices": [{
                "name": "byBrandColor",
                "properties": [{"brand": "asc"}, {"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(generate_random_identifier_struct(), 0, schemas, None, None)
            .expect("create contract")
            .data_contract_owned();

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                pv,
            )
            .expect("apply contract");

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        // acme: red×3, blue×2; contoso: red×2, green×1, blue×1.
        // Query: brand = acme AND color > "blue" → 3 (acme reds).
        let docs: &[(&str, &str)] = &[
            ("acme", "red"),
            ("acme", "red"),
            ("acme", "red"),
            ("acme", "blue"),
            ("acme", "blue"),
            ("contoso", "red"),
            ("contoso", "red"),
            ("contoso", "green"),
            ("contoso", "blue"),
        ];
        for (i, (brand, color)) in docs.iter().enumerate() {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), pv)
                .expect("random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("brand".to_string(), Value::Text(brand.to_string()));
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert document");
        }

        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("acme".to_string()),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("blue".to_string()),
            },
        ];
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .expect("compound range_countable index should be picked");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses,
        };

        let proof_bytes = query
            .execute_aggregate_count_with_proof(&drive, None, pv)
            .expect("should generate aggregate count proof");
        assert!(!proof_bytes.is_empty(), "proof must not be empty");

        let path_query = query
            .aggregate_count_path_query(pv)
            .expect("compound aggregate_count_path_query should build");

        let (root_hash, count) = GroveDb::verify_aggregate_count_query(
            &proof_bytes,
            &path_query,
            &pv.drive.grove_version,
        )
        .expect(
            "compound aggregate-count proof should verify (multi-layer \
             envelope walk through brand=acme to color leaf merk)",
        );
        assert_ne!(root_hash, [0u8; 32], "root hash should not be zero");
        assert_eq!(
            count, 3,
            "verified count should be 3 (acme reds; acme blues excluded by `> blue`)"
        );
    }
}
