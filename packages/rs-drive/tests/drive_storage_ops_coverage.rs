//! Integration-style unit tests for drive storage forms, batch operations,
//! contract info helpers, and vote poll resolution.
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::explicit_into_iter_loop)]
#![allow(clippy::useless_conversion)]

mod contested_document_resource_storage_form_tests {
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use drive::drive::votes::paths::{
        ACTIVE_POLLS_TREE_KEY, RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
        RESOURCE_LOCK_VOTE_TREE_KEY_U8_32,
    };
    use drive::drive::votes::storage_form::contested_document_resource_storage_form::ContestedDocumentResourceVoteStorageForm;
    use drive::drive::votes::tree_path_storage_form::TreePathStorageForm;

    /// Build a valid 10-element path for `try_from_tree_path`.
    /// Layout: [root, sub, active_polls_key, contract_id(32), doc_type_name,
    ///          index_type, idx_val_0, vote_choice(32), voter_id, leaf]
    fn make_path(vote_choice_bytes: [u8; 32], index_values: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        let mut path: Vec<Vec<u8>> = Vec::new();
        // 0 - root
        path.push(vec![0u8]);
        // 1 - sub-tree
        path.push(vec![1u8]);
        // 2 - active polls key (must be the ACTIVE_POLLS_TREE_KEY as a single byte)
        path.push(vec![ACTIVE_POLLS_TREE_KEY as u8]);
        // 3 - contract id (32 bytes)
        path.push(vec![42u8; 32]);
        // 4 - document type name (valid utf8)
        path.push(b"myDocType".to_vec());
        // 5 - index type / another key
        path.push(vec![5u8]);
        // 6..len-3 - index values
        for iv in &index_values {
            path.push(iv.clone());
        }
        // len-3 - vote choice (32 bytes)
        path.push(vote_choice_bytes.to_vec());
        // len-2 - voter identity
        path.push(vec![99u8; 32]);
        // len-1 - leaf
        path.push(vec![0u8]);

        path
    }

    #[test]
    fn try_from_tree_path_with_lock_vote_choice() {
        let path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![10, 20, 30]]);
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path).unwrap();

        assert_eq!(result.contract_id.to_buffer(), [42u8; 32]);
        assert_eq!(result.document_type_name, "myDocType");
        assert_eq!(result.resource_vote_choice, ResourceVoteChoice::Lock);
        assert_eq!(result.index_values, vec![vec![10u8, 20, 30]]);
    }

    #[test]
    fn try_from_tree_path_with_abstain_vote_choice() {
        let path = make_path(
            RESOURCE_ABSTAIN_VOTE_TREE_KEY_U8_32,
            vec![vec![1, 2], vec![3, 4]],
        );
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path).unwrap();

        assert_eq!(result.resource_vote_choice, ResourceVoteChoice::Abstain);
        assert_eq!(result.index_values.len(), 2);
        assert_eq!(result.index_values[0], vec![1, 2]);
        assert_eq!(result.index_values[1], vec![3, 4]);
    }

    #[test]
    fn try_from_tree_path_with_towards_identity_vote_choice() {
        // A 32-byte key that is neither LOCK nor ABSTAIN is interpreted as TowardsIdentity
        let mut identity_bytes = [0u8; 32];
        identity_bytes[0] = 0xAA;
        identity_bytes[31] = 0xBB;

        let path = make_path(identity_bytes, vec![vec![7, 8, 9]]);
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path).unwrap();

        match result.resource_vote_choice {
            ResourceVoteChoice::TowardsIdentity(id) => {
                assert_eq!(id.to_buffer(), identity_bytes);
            }
            other => panic!("Expected TowardsIdentity, got {:?}", other),
        }
    }

    #[test]
    fn try_from_tree_path_with_no_index_values_requires_min_length() {
        // With no index values, the path has 9 elements (< 10), so it should fail
        let path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![]);
        assert_eq!(path.len(), 9);
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(
            result.is_err(),
            "Path with 0 index values has only 9 elements, below minimum 10"
        );
    }

    #[test]
    fn try_from_tree_path_with_one_index_value_exactly_ten() {
        // With exactly 1 index value, the path has 10 elements (= minimum)
        let path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![42]]);
        assert_eq!(path.len(), 10);
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path).unwrap();
        assert_eq!(result.index_values, vec![vec![42u8]]);
    }

    #[test]
    fn try_from_tree_path_error_path_too_short() {
        // Path with only 9 elements (< 10)
        let path: Vec<Vec<u8>> = (0..9).map(|i| vec![i as u8]).collect();
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("not long enough"),
            "Error should mention path not long enough, got: {}",
            err_msg
        );
    }

    #[test]
    fn try_from_tree_path_error_active_polls_key_empty() {
        // Element at index 2 is empty (no first byte)
        let mut path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![1]]);
        path[2] = vec![]; // empty
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("third element must be a byte"),
            "Error should mention third element, got: {}",
            err_msg
        );
    }

    #[test]
    fn try_from_tree_path_error_wrong_active_polls_key() {
        let mut path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![1]]);
        path[2] = vec![0xFF]; // wrong key
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("ACTIVE_POLLS_TREE_KEY"),
            "Error should mention ACTIVE_POLLS_TREE_KEY, got: {}",
            err_msg
        );
    }

    #[test]
    fn try_from_tree_path_error_contract_id_wrong_length() {
        let mut path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![1]]);
        path[3] = vec![1, 2, 3]; // 3 bytes, not 32
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("32 bytes") || err_msg.contains("contract id"),
            "Error should mention contract id or 32 bytes, got: {}",
            err_msg
        );
    }

    #[test]
    fn try_from_tree_path_error_invalid_utf8_document_type() {
        let mut path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![1]]);
        path[4] = vec![0xFF, 0xFE, 0xFD]; // invalid UTF-8
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("document type name") || err_msg.contains("string"),
            "Error should mention document type name conversion, got: {}",
            err_msg
        );
    }

    #[test]
    fn try_from_tree_path_error_vote_choice_wrong_length() {
        let mut path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, vec![vec![1]]);
        // The vote choice is at index path.len() - 3
        let vote_idx = path.len() - 3;
        path[vote_idx] = vec![1, 2, 3]; // 3 bytes, not 32
        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("identifier") || err_msg.contains("RESOURCE_ABSTAIN"),
            "Error should mention identifier or RESOURCE keys, got: {}",
            err_msg
        );
    }

    #[test]
    fn storage_form_fields_are_correct() {
        let form = ContestedDocumentResourceVoteStorageForm {
            contract_id: dpp::identifier::Identifier::new([1u8; 32]),
            document_type_name: "testDoc".to_string(),
            index_values: vec![vec![10, 20], vec![30, 40]],
            resource_vote_choice: ResourceVoteChoice::Abstain,
        };
        assert_eq!(form.contract_id.to_buffer(), [1u8; 32]);
        assert_eq!(form.document_type_name, "testDoc");
        assert_eq!(form.index_values.len(), 2);
        assert_eq!(form.resource_vote_choice, ResourceVoteChoice::Abstain);
    }

    #[test]
    fn storage_form_clone_and_partial_eq() {
        let form = ContestedDocumentResourceVoteStorageForm {
            contract_id: dpp::identifier::Identifier::new([5u8; 32]),
            document_type_name: "doc".to_string(),
            index_values: vec![],
            resource_vote_choice: ResourceVoteChoice::Lock,
        };
        let cloned = form.clone();
        assert_eq!(form, cloned);
    }

    #[test]
    fn try_from_tree_path_multiple_index_values() {
        // 3 index values: 6 fixed before + 3 index vals + 3 fixed after = 12 elements
        let index_vals = vec![vec![1], vec![2], vec![3]];
        let path = make_path(RESOURCE_LOCK_VOTE_TREE_KEY_U8_32, index_vals.clone());
        assert_eq!(path.len(), 12);

        let result = ContestedDocumentResourceVoteStorageForm::try_from_tree_path(path).unwrap();
        assert_eq!(result.index_values, index_vals);
    }
}

mod contract_info_tests {
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::system_data_contracts::load_system_data_contract;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::util::object_size_info::{
        DataContractInfo, DataContractOwnedResolvedInfo, DataContractResolvedInfo, DocumentTypeInfo,
    };
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;

    fn make_test_contract() -> DataContract {
        let platform_version = PlatformVersion::latest();
        load_system_data_contract(SystemDataContract::Dashpay, platform_version)
            .expect("should load dashpay contract")
    }

    // --- DataContractOwnedResolvedInfo tests ---

    #[test]
    fn owned_resolved_info_owned_contract_id() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractOwnedResolvedInfo::OwnedDataContract(contract);
        assert_eq!(info.id(), expected_id);
    }

    #[test]
    fn owned_resolved_info_fetch_info_id() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let expected_id = fetch_info.contract.id();
        let info = DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info);
        assert_eq!(info.id(), expected_id);
    }

    #[test]
    fn owned_resolved_info_as_ref_owned() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractOwnedResolvedInfo::OwnedDataContract(contract);
        let contract_ref: &DataContract = info.as_ref();
        assert_eq!(contract_ref.id(), expected_id);
    }

    #[test]
    fn owned_resolved_info_as_ref_fetch_info() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let expected_id = fetch_info.contract.id();
        let info = DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info);
        let contract_ref: &DataContract = info.as_ref();
        assert_eq!(contract_ref.id(), expected_id);
    }

    #[test]
    fn owned_resolved_info_into_owned_from_owned() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractOwnedResolvedInfo::OwnedDataContract(contract);
        let owned = info.into_owned();
        assert_eq!(owned.id(), expected_id);
    }

    #[test]
    fn owned_resolved_info_into_owned_from_fetch_info() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let expected_id = fetch_info.contract.id();
        let info = DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info);
        let owned = info.into_owned();
        assert_eq!(owned.id(), expected_id);
    }

    // --- DataContractResolvedInfo tests ---

    #[test]
    fn resolved_info_borrowed_id() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractResolvedInfo::BorrowedDataContract(&contract);
        assert_eq!(info.id(), expected_id);
    }

    #[test]
    fn resolved_info_owned_id() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractResolvedInfo::OwnedDataContract(contract);
        assert_eq!(info.id(), expected_id);
    }

    #[test]
    fn resolved_info_arc_data_contract_id() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractResolvedInfo::ArcDataContract(Arc::new(contract));
        assert_eq!(info.id(), expected_id);
    }

    #[test]
    fn resolved_info_arc_fetch_info_id() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let expected_id = fetch_info.contract.id();
        let info = DataContractResolvedInfo::ArcDataContractFetchInfo(fetch_info);
        assert_eq!(info.id(), expected_id);
    }

    #[test]
    fn resolved_info_as_ref_all_variants() {
        let contract = make_test_contract();
        let expected_id = contract.id();

        // BorrowedDataContract
        let info = DataContractResolvedInfo::BorrowedDataContract(&contract);
        assert_eq!(info.as_ref().id(), expected_id);

        // OwnedDataContract
        let contract2 = make_test_contract();
        let info = DataContractResolvedInfo::OwnedDataContract(contract2);
        assert_eq!(info.as_ref().id(), expected_id);

        // ArcDataContract
        let contract3 = make_test_contract();
        let info = DataContractResolvedInfo::ArcDataContract(Arc::new(contract3));
        assert_eq!(info.as_ref().id(), expected_id);

        // ArcDataContractFetchInfo
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let fetch_expected_id = fetch_info.contract.id();
        let info = DataContractResolvedInfo::ArcDataContractFetchInfo(fetch_info);
        assert_eq!(info.as_ref().id(), fetch_expected_id);
    }

    // --- From conversions ---

    #[test]
    fn resolved_info_from_owned_resolved_owned_contract() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let owned_info = DataContractOwnedResolvedInfo::OwnedDataContract(contract);
        let resolved: DataContractResolvedInfo = (&owned_info).into();
        assert_eq!(resolved.id(), expected_id);

        // Should be BorrowedDataContract variant
        match resolved {
            DataContractResolvedInfo::BorrowedDataContract(_) => {}
            other => panic!(
                "Expected BorrowedDataContract variant, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[test]
    fn resolved_info_from_owned_resolved_fetch_info() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let expected_id = fetch_info.contract.id();
        let owned_info = DataContractOwnedResolvedInfo::DataContractFetchInfo(fetch_info);
        let resolved: DataContractResolvedInfo = (&owned_info).into();
        assert_eq!(resolved.id(), expected_id);

        match resolved {
            DataContractResolvedInfo::ArcDataContractFetchInfo(_) => {}
            other => panic!(
                "Expected ArcDataContractFetchInfo variant, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    // --- DataContractInfo construction tests ---

    #[test]
    fn data_contract_info_data_contract_id_variant() {
        let id = Identifier::new([99u8; 32]);
        let info = DataContractInfo::DataContractId(id);
        match info {
            DataContractInfo::DataContractId(got_id) => {
                assert_eq!(got_id.to_buffer(), [99u8; 32]);
            }
            _ => panic!("Expected DataContractId variant"),
        }
    }

    #[test]
    fn data_contract_info_borrowed_contract_variant() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractInfo::BorrowedDataContract(&contract);
        match info {
            DataContractInfo::BorrowedDataContract(c) => {
                assert_eq!(c.id(), expected_id);
            }
            _ => panic!("Expected BorrowedDataContract variant"),
        }
    }

    #[test]
    fn data_contract_info_owned_contract_variant() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = DataContractInfo::OwnedDataContract(contract);
        match info {
            DataContractInfo::OwnedDataContract(c) => {
                assert_eq!(c.id(), expected_id);
            }
            _ => panic!("Expected OwnedDataContract variant"),
        }
    }

    #[test]
    fn data_contract_info_fetch_info_variant() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let expected_id = fetch_info.contract.id();
        let info = DataContractInfo::DataContractFetchInfo(fetch_info);
        match info {
            DataContractInfo::DataContractFetchInfo(fi) => {
                assert_eq!(fi.contract.id(), expected_id);
            }
            _ => panic!("Expected DataContractFetchInfo variant"),
        }
    }

    // --- DocumentTypeInfo tests ---

    #[test]
    fn document_type_info_document_type_name() {
        let info = DocumentTypeInfo::DocumentTypeName("myDoc".to_string());
        match info {
            DocumentTypeInfo::DocumentTypeName(name) => assert_eq!(name, "myDoc"),
            _ => panic!("Expected DocumentTypeName variant"),
        }
    }

    #[test]
    fn document_type_info_document_type_name_as_str() {
        let info = DocumentTypeInfo::DocumentTypeNameAsStr("myDoc");
        match info {
            DocumentTypeInfo::DocumentTypeNameAsStr(name) => assert_eq!(name, "myDoc"),
            _ => panic!("Expected DocumentTypeNameAsStr variant"),
        }
    }

    #[test]
    fn document_type_info_resolve_with_nonexistent_type_errors() {
        let contract = make_test_contract();
        let info = DocumentTypeInfo::DocumentTypeName("nonexistent".to_string());
        let result = info.resolve(&contract);
        assert!(
            result.is_err(),
            "Resolving a non-existent document type should fail"
        );
    }

    #[test]
    fn document_type_info_resolve_str_with_nonexistent_type_errors() {
        let contract = make_test_contract();
        let info = DocumentTypeInfo::DocumentTypeNameAsStr("nonexistent");
        let result = info.resolve(&contract);
        assert!(
            result.is_err(),
            "Resolving a non-existent document type should fail"
        );
    }
}

mod contested_document_resource_vote_poll_tests {
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::data_contracts::SystemDataContract;
    use dpp::platform_value::Value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
    use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::{
        ContestedDocumentResourceVotePollWithContractInfo,
        ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed,
    };
    use drive::util::object_size_info::{DataContractOwnedResolvedInfo, DataContractResolvedInfo};
    use platform_version::version::PlatformVersion;
    use std::sync::Arc;

    fn make_test_contract() -> DataContract {
        let platform_version = PlatformVersion::latest();
        load_system_data_contract(SystemDataContract::Dashpay, platform_version)
            .expect("should load dashpay contract")
    }

    fn make_vote_poll_for_contract(contract: &DataContract) -> ContestedDocumentResourceVotePoll {
        ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        }
    }

    fn make_different_contract() -> DataContract {
        let platform_version = PlatformVersion::latest();
        load_system_data_contract(SystemDataContract::DPNS, platform_version)
            .expect("should load dpns contract")
    }

    // --- From conversions ---

    #[test]
    fn from_owned_with_contract_info_to_vote_poll() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        };

        let poll: ContestedDocumentResourceVotePoll = info.into();
        assert_eq!(poll.contract_id, expected_id);
        assert_eq!(poll.document_type_name, "testDoc");
        assert_eq!(poll.index_name, "testIndex");
        assert_eq!(poll.index_values, vec![Value::Text("hello".to_string())]);
    }

    #[test]
    fn from_ref_with_contract_info_to_vote_poll() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "testDoc".to_string(),
            index_name: "idx".to_string(),
            index_values: vec![],
        };

        let poll: ContestedDocumentResourceVotePoll = (&info).into();
        assert_eq!(poll.contract_id, expected_id);
        assert_eq!(poll.document_type_name, "testDoc");
        assert_eq!(poll.index_name, "idx");
    }

    #[test]
    fn from_allow_borrowed_owned_to_vote_poll() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::OwnedDataContract(contract),
            document_type_name: "docA".to_string(),
            index_name: "idxA".to_string(),
            index_values: vec![Value::U64(42)],
        };

        let poll: ContestedDocumentResourceVotePoll = info.into();
        assert_eq!(poll.document_type_name, "docA");
        assert_eq!(poll.index_name, "idxA");
        assert_eq!(poll.index_values, vec![Value::U64(42)]);
    }

    #[test]
    fn from_ref_allow_borrowed_to_vote_poll() {
        let contract = make_test_contract();
        let expected_id = contract.id();
        let info = ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::BorrowedDataContract(&contract),
            document_type_name: "docB".to_string(),
            index_name: "idxB".to_string(),
            index_values: vec![],
        };

        let poll: ContestedDocumentResourceVotePoll = (&info).into();
        assert_eq!(poll.contract_id, expected_id);
        assert_eq!(poll.document_type_name, "docB");
    }

    #[test]
    fn from_owned_with_contract_info_to_allow_borrowed() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "docC".to_string(),
            index_name: "idxC".to_string(),
            index_values: vec![Value::Bool(true)],
        };

        let borrowed: ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed =
            (&info).into();
        assert_eq!(borrowed.document_type_name, "docC");
        assert_eq!(borrowed.index_name, "idxC");
        assert_eq!(borrowed.index_values, vec![Value::Bool(true)]);
    }

    // --- serialize / hash / unique_id ---

    #[test]
    fn serialize_to_bytes_round_trip() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        };

        let bytes = info.serialize_to_bytes().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn sha256_2_hash_produces_32_bytes() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![],
        };

        let hash = info.sha256_2_hash().unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn unique_id_and_specialized_balance_id_are_equal() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        };

        let contract2 = make_test_contract();
        let info2 = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract2),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        };

        let uid = info.unique_id().unwrap();
        let bid = info2.specialized_balance_id().unwrap();
        assert_eq!(uid, bid);
    }

    #[test]
    fn allow_borrowed_serialize_to_bytes() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::BorrowedDataContract(&contract),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![],
        };

        let bytes = info.serialize_to_bytes().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn allow_borrowed_unique_id_equals_specialized_balance_id() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::BorrowedDataContract(&contract),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![],
        };

        let uid = info.unique_id().unwrap();
        let bid = info.specialized_balance_id().unwrap();
        assert_eq!(uid, bid);
    }

    // --- resolve_with_provided_borrowed_contract ---

    #[test]
    fn resolve_with_provided_borrowed_contract_success() {
        let contract = make_test_contract();
        let poll = make_vote_poll_for_contract(&contract);

        let result = poll.resolve_with_provided_borrowed_contract(&contract);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.document_type_name, "testDoc");
        assert_eq!(resolved.index_name, "testIndex");
    }

    #[test]
    fn resolve_with_provided_borrowed_contract_mismatch() {
        let contract = make_test_contract();
        let wrong_contract = make_different_contract();
        let poll = make_vote_poll_for_contract(&contract);

        let result = poll.resolve_with_provided_borrowed_contract(&wrong_contract);
        assert!(result.is_err());
    }

    // --- resolve_with_provided_arc_contract_fetch_info ---

    #[test]
    fn resolve_with_provided_arc_contract_fetch_info_success() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        // Build poll matching the dashpay fixture contract id
        let poll = ContestedDocumentResourceVotePoll {
            contract_id: fetch_info.contract.id(),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        };

        let result = poll.resolve_with_provided_arc_contract_fetch_info(fetch_info);
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.document_type_name, "testDoc");
    }

    #[test]
    fn resolve_with_provided_arc_contract_fetch_info_mismatch() {
        let contract = make_test_contract();
        let poll = make_vote_poll_for_contract(&contract);
        // Use a different contract (DPNS) to cause a mismatch
        let fetch_info = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));

        let result = poll.resolve_with_provided_arc_contract_fetch_info(fetch_info);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_owned_with_provided_arc_contract_fetch_info_success() {
        let fetch_info = Arc::new(DataContractFetchInfo::dashpay_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));
        let poll = ContestedDocumentResourceVotePoll {
            contract_id: fetch_info.contract.id(),
            document_type_name: "testDoc".to_string(),
            index_name: "testIndex".to_string(),
            index_values: vec![Value::Text("hello".to_string())],
        };

        let result = poll.resolve_owned_with_provided_arc_contract_fetch_info(fetch_info);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_owned_with_provided_arc_contract_fetch_info_mismatch() {
        let contract = make_test_contract();
        let poll = make_vote_poll_for_contract(&contract);
        // Use a different contract (DPNS) to cause a mismatch
        let fetch_info = Arc::new(DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        ));

        let result = poll.resolve_owned_with_provided_arc_contract_fetch_info(fetch_info);
        assert!(result.is_err());
    }

    // --- document_type / index errors ---

    #[test]
    fn document_type_errors_on_nonexistent() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract),
            document_type_name: "nonexistent".to_string(),
            index_name: "idx".to_string(),
            index_values: vec![],
        };

        assert!(info.document_type().is_err());
        assert!(info.document_type_borrowed().is_err());
    }

    #[test]
    fn allow_borrowed_document_type_errors_on_nonexistent() {
        let contract = make_test_contract();
        let info = ContestedDocumentResourceVotePollWithContractInfoAllowBorrowed {
            contract: DataContractResolvedInfo::BorrowedDataContract(&contract),
            document_type_name: "nonexistent".to_string(),
            index_name: "idx".to_string(),
            index_values: vec![],
        };

        assert!(info.document_type().is_err());
        assert!(info.document_type_borrowed().is_err());
    }
}

mod document_operation_tests {
    use drive::util::batch::drive_op_batch::{
        DocumentOperation, DocumentOperationType, UpdateOperationInfo,
    };

    #[test]
    fn document_operation_type_variants_are_constructible() {
        // Verify that the enum variants can be matched
        let op = DocumentOperation::AddOperation {
            owned_document_info: drive::util::object_size_info::OwnedDocumentInfo {
                document_info:
                    drive::util::object_size_info::DocumentInfo::DocumentEstimatedAverageSize(100),
                owner_id: Some([1u8; 32]),
            },
            override_document: true,
        };

        match &op {
            DocumentOperation::AddOperation {
                owned_document_info,
                override_document,
            } => {
                assert!(override_document);
                assert_eq!(owned_document_info.owner_id, Some([1u8; 32]));
            }
            _ => panic!("Expected AddOperation"),
        }
    }

    #[test]
    fn document_operation_add_type_construction() {
        let id = dpp::prelude::Identifier::new([2u8; 32]);
        let _op = DocumentOperationType::DeleteDocument {
            document_id: id,
            contract_info: drive::util::object_size_info::DataContractInfo::DataContractId(
                dpp::prelude::Identifier::new([3u8; 32]),
            ),
            document_type_info: drive::util::object_size_info::DocumentTypeInfo::DocumentTypeName(
                "testDoc".to_string(),
            ),
        };

        // Just verifying it constructs without panic
    }

    #[test]
    fn update_operation_info_construction() {
        use dpp::document::Document;

        let doc = Document::V0(dpp::document::DocumentV0 {
            id: dpp::prelude::Identifier::new([1u8; 32]),
            owner_id: dpp::prelude::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let update_info = UpdateOperationInfo {
            document: &doc,
            serialized_document: None,
            owner_id: Some([2u8; 32]),
            storage_flags: None,
        };

        assert_eq!(update_info.owner_id, Some([2u8; 32]));
        assert!(update_info.serialized_document.is_none());
        assert!(update_info.storage_flags.is_none());
    }

    #[test]
    fn update_operation_info_with_serialized_document() {
        use dpp::document::Document;

        let doc = Document::V0(dpp::document::DocumentV0 {
            id: dpp::prelude::Identifier::new([1u8; 32]),
            owner_id: dpp::prelude::Identifier::new([2u8; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        });

        let serialized = vec![1, 2, 3, 4, 5];
        let update_info = UpdateOperationInfo {
            document: &doc,
            serialized_document: Some(&serialized),
            owner_id: None,
            storage_flags: None,
        };

        assert_eq!(update_info.serialized_document, Some(&serialized[..]));
        assert!(update_info.owner_id.is_none());
    }
}

mod grovedb_op_batch_tests {
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatch;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use grovedb::batch::{GroveOp, QualifiedGroveDbOp};
    use grovedb::{Element, TreeType};

    #[test]
    fn new_batch_is_empty() {
        let batch = GroveDbOpBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn push_increases_len() {
        let mut batch = GroveDbOpBatch::new();
        let op =
            QualifiedGroveDbOp::insert_or_replace_op(vec![vec![1]], vec![2], Element::empty_tree());
        batch.push(op);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn from_operations() {
        let ops = vec![
            QualifiedGroveDbOp::insert_or_replace_op(vec![vec![1]], vec![2], Element::empty_tree()),
            QualifiedGroveDbOp::insert_or_replace_op(vec![vec![3]], vec![4], Element::empty_tree()),
        ];
        let batch = GroveDbOpBatch::from_operations(ops);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn append_merges_batches() {
        let mut batch1 = GroveDbOpBatch::new();
        batch1.add_insert_empty_tree(vec![vec![1]], vec![2]);

        let mut batch2 = GroveDbOpBatch::new();
        batch2.add_insert_empty_tree(vec![vec![3]], vec![4]);
        batch2.add_insert_empty_tree(vec![vec![5]], vec![6]);

        batch1.append(&mut batch2);
        assert_eq!(batch1.len(), 3);
        assert_eq!(batch2.len(), 0);
    }

    #[test]
    fn extend_adds_operations() {
        let mut batch = GroveDbOpBatch::new();
        let ops = vec![
            QualifiedGroveDbOp::insert_or_replace_op(vec![vec![1]], vec![2], Element::empty_tree()),
            QualifiedGroveDbOp::insert_or_replace_op(vec![vec![3]], vec![4], Element::empty_tree()),
        ];
        batch.extend(ops);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn add_insert_empty_tree() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![1, 2]], vec![3, 4]);
        assert_eq!(batch.len(), 1);

        let result = batch.contains([&[1u8, 2][..]].into_iter(), &[3, 4]);
        assert!(result.is_some());
    }

    #[test]
    fn add_insert_empty_tree_with_flags() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree_with_flags(vec![vec![10]], vec![20], &None);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn add_insert_empty_sum_tree() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_sum_tree(vec![vec![1]], vec![2]);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn add_insert_empty_sum_tree_with_flags() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_sum_tree_with_flags(vec![vec![1]], vec![2], &None);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn add_delete() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_delete(vec![vec![1]], vec![2]);
        assert_eq!(batch.len(), 1);

        let result = batch.contains([&[1u8][..]].into_iter(), &[2]);
        assert!(result.is_some());
    }

    #[test]
    fn add_delete_tree() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_delete_tree(vec![vec![1]], vec![2], TreeType::NormalTree);
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn add_insert_with_element() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert(vec![vec![1]], vec![2], Element::new_item(vec![10, 20, 30]));
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn contains_returns_none_for_missing() {
        let batch = GroveDbOpBatch::new();
        let result = batch.contains([&[1u8][..]].into_iter(), &[2]);
        assert!(result.is_none());
    }

    #[test]
    fn contains_finds_existing_op() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![10], vec![20]], vec![30]);

        let result = batch.contains([&[10u8][..], &[20u8][..]].into_iter(), &[30]);
        assert!(result.is_some());

        // Should not find with different key
        let result = batch.contains([&[10u8][..], &[20u8][..]].into_iter(), &[99]);
        assert!(result.is_none());

        // Should not find with different path
        let result = batch.contains([&[10u8][..], &[99u8][..]].into_iter(), &[30]);
        assert!(result.is_none());
    }

    #[test]
    fn remove_existing_op() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![1]], vec![2]);
        batch.add_insert_empty_tree(vec![vec![3]], vec![4]);
        assert_eq!(batch.len(), 2);

        let removed = batch.remove([&[1u8][..]].into_iter(), &[2]);
        assert!(removed.is_some());
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![1]], vec![2]);

        let removed = batch.remove([&[99u8][..]].into_iter(), &[99]);
        assert!(removed.is_none());
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn remove_if_insert_removes_insert_op() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![1]], vec![2]);
        assert_eq!(batch.len(), 1);

        let result = batch.remove_if_insert(vec![vec![1]], &[2]);
        assert!(result.is_some());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn remove_if_insert_does_not_remove_delete_op() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_delete(vec![vec![1]], vec![2]);
        assert_eq!(batch.len(), 1);

        let result = batch.remove_if_insert(vec![vec![1]], &[2]);
        // Should return the op but NOT remove it (it's a delete, not insert)
        assert!(result.is_some());
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn remove_if_insert_returns_none_for_missing() {
        let mut batch = GroveDbOpBatch::new();
        let result = batch.remove_if_insert(vec![vec![1]], &[2]);
        assert!(result.is_none());
    }

    #[test]
    fn into_iter_works() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![1]], vec![2]);
        batch.add_insert_empty_tree(vec![vec![3]], vec![4]);

        let ops: Vec<_> = batch.into_iter().collect();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn display_formatting_for_insert_item() {
        let mut batch = GroveDbOpBatch::new();
        // Insert an item with 8 bytes (should display as u64)
        batch.add_insert(
            vec![vec![96u8]], // Balances root tree key
            vec![1; 32],      // 32-byte identity id
            Element::new_item(42u64.to_be_bytes().to_vec()),
        );

        let display = format!("{}", batch);
        assert!(display.contains("Path:"), "Display should contain 'Path:'");
        assert!(display.contains("Key:"), "Display should contain 'Key:'");
        assert!(
            display.contains("Operation:"),
            "Display should contain 'Operation:'"
        );
        assert!(
            display.contains("u64(42)"),
            "Display should show u64 value, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_for_insert_item_u32() {
        let mut batch = GroveDbOpBatch::new();
        // Insert an item with 4 bytes (should display as u32)
        batch.add_insert(
            vec![vec![104u8]], // Misc root tree key
            vec![5, 6, 7],
            Element::new_item(123u32.to_be_bytes().to_vec()),
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("u32(123)"),
            "Display should show u32 value, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_for_empty_tree() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_tree(vec![vec![32u8]], vec![1; 32]);

        let display = format!("{}", batch);
        assert!(
            display.contains("Insert Empty Tree"),
            "Display should mention empty tree, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_for_empty_sum_tree() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert_empty_sum_tree(vec![vec![96u8]], vec![1; 32]);

        let display = format!("{}", batch);
        assert!(
            display.contains("Insert Empty Sum Tree"),
            "Display should mention empty sum tree, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_for_delete() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_delete(vec![vec![1]], vec![2]);

        let display = format!("{}", batch);
        assert!(
            display.contains("Operation:"),
            "Display should contain operation info, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_root_tree_paths() {
        let mut batch = GroveDbOpBatch::new();
        // Use Identities root (32) as path, then identity root structure
        batch.add_insert_empty_tree(
            vec![vec![32u8]], // Identities
            vec![1; 32],
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Identities"),
            "Display should resolve root tree name, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_data_contract_documents_root() {
        let mut batch = GroveDbOpBatch::new();
        // DataContractDocuments = 64
        batch.add_insert_empty_tree(
            vec![vec![64u8]],
            vec![0u8], // DataContractStorage sub-key
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("DataContractAndDocumentsRoot"),
            "Display should resolve DataContractDocuments, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_pools_root() {
        let mut batch = GroveDbOpBatch::new();
        // Pools = 48
        batch.add_insert(
            vec![vec![48u8]],
            vec![b's'], // StorageFeePool
            Element::new_item(vec![1, 2, 3]),
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Pools"),
            "Display should show Pools, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_balances_root_with_identity() {
        let mut batch = GroveDbOpBatch::new();
        // Balances = 96, key is a 32-byte identity id
        batch.add_insert(
            vec![vec![96u8]],
            vec![0xAA; 32],
            Element::new_item(1000u64.to_be_bytes().to_vec()),
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Balances"),
            "Display should show Balances root, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_token_root_balances() {
        let mut batch = GroveDbOpBatch::new();
        // Tokens = 16
        batch.add_insert_empty_tree(
            vec![vec![16u8]],
            vec![128u8], // TOKEN_BALANCES_KEY = 128
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Token"),
            "Display should show Tokens root, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_identity_root_structure() {
        let mut batch = GroveDbOpBatch::new();
        // Identities = 32, then IdentityTreeRevision = 192
        batch.add_insert_empty_tree(
            vec![vec![32u8], vec![1; 32]], // identity tree
            vec![192u8],                   // IdentityTreeRevision
        );

        let display = format!("{}", batch);
        // The second path element is a 32-byte identity ID
        assert!(
            display.contains("IdentityId") || display.contains("Identities"),
            "Display should show identity path info, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_epochs_inside_pools() {
        let mut batch = GroveDbOpBatch::new();
        // Pools = 48, then epoch key (2 bytes for epoch 0 = [1, 0] because of 256 offset)
        batch.add_insert(
            vec![vec![48u8], vec![1, 0]], // Pools -> Epoch 0
            vec![b'p'],                   // KEY_POOL_PROCESSING_FEES
            Element::new_item(vec![0; 8]),
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Pools") || display.contains("Epoch"),
            "Display should show pools/epoch path info, got: {}",
            display
        );
    }

    #[test]
    fn display_formatting_misc_root() {
        let mut batch = GroveDbOpBatch::new();
        // Misc = 104
        batch.add_insert(
            vec![vec![104u8]],
            vec![1, 2, 3],
            Element::new_item(vec![10]),
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Misc"),
            "Display should resolve Misc root, got: {}",
            display
        );
    }

    #[test]
    fn verify_consistency_of_empty_batch() {
        let batch = GroveDbOpBatch::new();
        let results = batch.verify_consistency_of_operations();
        // An empty batch should have no inconsistencies
        assert!(
            results.is_empty(),
            "Empty batch should have no inconsistencies"
        );
    }

    #[test]
    fn default_batch_is_empty() {
        let batch = GroveDbOpBatch::default();
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn display_no_key_operation() {
        // Build an op with key = None to test the "(none)" branch
        use grovedb::batch::KeyInfoPath;

        let op = QualifiedGroveDbOp {
            path: KeyInfoPath(vec![]),
            key: None,
            op: GroveOp::Delete,
        };
        let mut batch = GroveDbOpBatch::new();
        batch.push(op);

        let display = format!("{}", batch);
        assert!(
            display.contains("(none)"),
            "Display should show '(none)' for missing key, got: {}",
            display
        );
    }

    #[test]
    fn display_data_contract_storage_subkey() {
        let mut batch = GroveDbOpBatch::new();
        // DataContractDocuments (64), then key [0] (DataContractStorage)
        batch.add_insert_empty_tree(vec![vec![64u8]], vec![0u8]);

        let display = format!("{}", batch);
        assert!(
            display.contains("DataContractStorage"),
            "Display should resolve DataContractStorage(0), got: {}",
            display
        );
    }

    #[test]
    fn display_data_contract_documents_subkey() {
        let mut batch = GroveDbOpBatch::new();
        // DataContractDocuments (64), then key [1] (DataContractDocuments sub)
        batch.add_insert_empty_tree(vec![vec![64u8]], vec![1u8]);

        let display = format!("{}", batch);
        assert!(
            display.contains("DataContractDocuments"),
            "Display should resolve DataContractDocuments(1), got: {}",
            display
        );
    }

    #[test]
    fn display_contract_id_key_in_data_contracts_root() {
        let mut batch = GroveDbOpBatch::new();
        // DataContractDocuments (64), then a 32-byte contract ID as key
        batch.add_insert_empty_tree(vec![vec![64u8]], vec![0xBB; 32]);

        let display = format!("{}", batch);
        assert!(
            display.contains("ContractId"),
            "Display should show ContractId for 32-byte key in data contracts root, got: {}",
            display
        );
    }

    #[test]
    fn display_pools_root_storage_fee_pool() {
        let mut batch = GroveDbOpBatch::new();
        // Pools (48), key = 's' (StorageFeePool)
        batch.add_insert(vec![vec![48u8]], vec![b's'], Element::new_item(vec![0; 8]));

        let display = format!("{}", batch);
        assert!(
            display.contains("StorageFeePool"),
            "Display should show StorageFeePool, got: {}",
            display
        );
    }

    #[test]
    fn display_pools_root_unpaid_epoch_index() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert(vec![vec![48u8]], vec![b'u'], Element::new_item(vec![0; 2]));

        let display = format!("{}", batch);
        assert!(
            display.contains("UnpaidEpochIndex"),
            "Display should show UnpaidEpochIndex, got: {}",
            display
        );
    }

    #[test]
    fn display_pools_root_pending_epoch_refunds() {
        let mut batch = GroveDbOpBatch::new();
        batch.add_insert(vec![vec![48u8]], vec![b'p'], Element::new_item(vec![0; 2]));

        let display = format!("{}", batch);
        assert!(
            display.contains("PendingEpochRefunds"),
            "Display should show PendingEpochRefunds, got: {}",
            display
        );
    }

    #[test]
    fn display_epoch_key_constants() {
        // Pools (48) -> Epoch 0 ([1, 0]) -> various keys
        let epoch_key = vec![1u8, 0]; // epoch 0

        let keys_and_expected = vec![
            (vec![b'p'], "PoolProcessingFees"),
            (vec![b's'], "PoolStorageFees"),
            (vec![b't'], "StartTime"),
            (vec![b'v'], "ProtocolVersion"),
            (vec![b'h'], "StartBlockHeight"),
            (vec![b'c'], "StartBlockCoreHeight"),
            (vec![b'm'], "Proposers"),
            (vec![b'x'], "FeeMultiplier"),
        ];

        for (key, expected) in keys_and_expected {
            let mut batch = GroveDbOpBatch::new();
            batch.add_insert(
                vec![vec![48u8], epoch_key.clone()],
                key,
                Element::new_item(vec![0]),
            );

            let display = format!("{}", batch);
            assert!(
                display.contains(expected),
                "Display should contain '{}', got: {}",
                expected,
                display
            );
        }
    }

    #[test]
    fn display_token_root_keys() {
        // Test each token root sub-key
        let test_cases = vec![
            (32u8, "Distribution"),  // TOKEN_DISTRIBUTIONS_KEY
            (92u8, "SellPrice"),     // TOKEN_DIRECT_SELL_PRICE_KEY
            (128u8, "Balances"),     // TOKEN_BALANCES_KEY
            (192u8, "IdentityInfo"), // TOKEN_IDENTITY_INFO_KEY
            (160u8, "ContractInfo"), // TOKEN_CONTRACT_INFO_KEY
            (64u8, "Status"),        // TOKEN_STATUS_INFO_KEY
        ];

        for (key, expected) in test_cases {
            let mut batch = GroveDbOpBatch::new();
            batch.add_insert_empty_tree(
                vec![vec![16u8]], // Tokens root
                vec![key],
            );

            let display = format!("{}", batch);
            assert!(
                display.contains(expected),
                "Token key {} should produce '{}', got: {}",
                key,
                expected,
                display
            );
        }
    }

    #[test]
    fn display_token_distribution_sub_keys() {
        // Test distribution sub-keys
        let test_cases = vec![
            (128u8, "TimedDistribution"),         // TOKEN_TIMED_DISTRIBUTIONS_KEY
            (64u8, "PerpetualDistribution"),      // TOKEN_PERPETUAL_DISTRIBUTIONS_KEY
            (192u8, "PreProgrammedDistribution"), // TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY
        ];

        for (key, expected) in test_cases {
            let mut batch = GroveDbOpBatch::new();
            batch.add_insert_empty_tree(
                vec![vec![16u8], vec![32u8]], // Tokens -> Distribution
                vec![key],
            );

            let display = format!("{}", batch);
            assert!(
                display.contains(expected),
                "Distribution key {} should produce '{}', got: {}",
                key,
                expected,
                display
            );
        }
    }

    #[test]
    fn display_timed_distribution_sub_keys() {
        let test_cases = vec![
            (128u8, "MillisecondTimedDistribution"), // TOKEN_MS_TIMED_DISTRIBUTIONS_KEY
            (64u8, "BlockTimedDistribution"),        // TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY
            (192u8, "EpochTimedDistribution"),       // TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY
        ];

        for (key, expected) in test_cases {
            let mut batch = GroveDbOpBatch::new();
            batch.add_insert_empty_tree(
                vec![vec![16u8], vec![32u8], vec![128u8]], // Tokens -> Distribution -> Timed
                vec![key],
            );

            let display = format!("{}", batch);
            assert!(
                display.contains(expected),
                "Timed distribution key {} should produce '{}', got: {}",
                key,
                expected,
                display
            );
        }
    }

    #[test]
    fn display_perpetual_distribution_sub_keys() {
        let test_cases = vec![
            (128u8, "PerpetualDistributionInfo"), // TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY
            (192u8, "PerpetualDistributionLastClaim"), // TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY
        ];

        for (key, expected) in test_cases {
            let mut batch = GroveDbOpBatch::new();
            batch.add_insert_empty_tree(
                vec![vec![16u8], vec![32u8], vec![64u8]], // Tokens -> Distribution -> Perpetual
                vec![key],
            );

            let display = format!("{}", batch);
            assert!(
                display.contains(expected),
                "Perpetual distribution key {} should produce '{}', got: {}",
                key,
                expected,
                display
            );
        }
    }

    #[test]
    fn display_identity_key_references_purpose() {
        let mut batch = GroveDbOpBatch::new();
        // Identities (32) -> 32-byte id -> IdentityTreeKeyReferences (160) -> Purpose::Authentication (0)
        batch.add_insert_empty_tree(
            vec![vec![32u8], vec![1; 32], vec![160u8]], // identity key references root
            vec![0u8],                                  // Purpose::Authentication = 0
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Purpose") || display.contains("Authentication"),
            "Display should show Purpose info, got: {}",
            display
        );
    }

    #[test]
    fn display_identity_key_references_security_level() {
        let mut batch = GroveDbOpBatch::new();
        // Identities (32) -> 32-byte id -> IdentityTreeKeyReferences (160) -> Purpose (0) -> SecurityLevel (0)
        batch.add_insert_empty_tree(
            vec![vec![32u8], vec![1; 32], vec![160u8], vec![0u8]], // up to purpose
            vec![0u8],                                             // SecurityLevel::Master = 0
        );

        let display = format!("{}", batch);
        // This tests the IdentityTreeKeyReferencesInPurpose -> SecurityLevel path
        assert!(
            display.contains("SecurityLevel") || display.contains("Purpose"),
            "Display should show security level info, got: {}",
            display
        );
    }

    #[test]
    fn display_item_with_flags() {
        let mut batch = GroveDbOpBatch::new();
        let flags = vec![1, 2, 3];
        batch.add_insert(
            vec![vec![1]],
            vec![2],
            Element::new_item_with_flags(vec![10, 20], Some(flags)),
        );

        let display = format!("{}", batch);
        assert!(
            display.contains("Flags are 0x"),
            "Display should show flags, got: {}",
            display
        );
    }
}
