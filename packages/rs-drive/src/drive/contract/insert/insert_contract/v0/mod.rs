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
            // for each type we should insert the indices that are top level
            for index in document_type.as_ref().top_level_indices() {
                // toDo: change this to be a reference by index
                let index_bytes = index.name.as_bytes();
                if !index_cache.contains(index_bytes) {
                    self.batch_insert_empty_tree(
                        type_path,
                        KeyRef(index_bytes),
                        storage_flags.as_ref(),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?;
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
    use grovedb::Element;

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
}
