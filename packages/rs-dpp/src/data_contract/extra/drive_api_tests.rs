mod test {
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::config::v0::{
        DataContractConfigGettersV0, DataContractConfigSettersV0, DataContractConfigV0,
    };
    use crate::data_contract::config::DataContractConfig;
    #[cfg(feature = "data-contract-cbor-conversion")]
    use crate::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;
    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::DataContract;
    use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
    use crate::serialization::PlatformSerializableWithPlatformVersion;
    use crate::tests::json_document::json_document_to_contract;
    use platform_version::version::PlatformVersion;

    type IndexName = &'static str;
    type IsIndexUnique = bool;
    type IndexPropertyName = &'static str;
    type IndexOrderDirection = &'static str;
    type IndexProperties = &'static [(IndexPropertyName, IndexOrderDirection)];

    #[derive(Default)]
    struct ExpectedDocumentsData {
        document_name: &'static str,
        required_properties: &'static [&'static str],
        indexes: &'static [(IndexName, IsIndexUnique, IndexProperties)],
    }

    fn expected_documents() -> Vec<ExpectedDocumentsData> {
        vec![
            ExpectedDocumentsData {
                document_name: "profile",
                required_properties: &["$cratedAt"],
                indexes: &[
                    ("index1", true, &[("$ownerId", "asc")]),
                    (
                        "index2",
                        false,
                        &[("$ownerId", "asc"), ("$updatedAt", "asc")],
                    ),
                ],
                ..Default::default()
            },
            ExpectedDocumentsData {
                document_name: "contactInfo",
                required_properties: &["lastName", "$cratedAt"],
                indexes: &[
                    ("index1", true, &[("$ownerId", "asc")]),
                    ("index2", false, &[("$ownerId", "asc"), ("lastName", "asc")]),
                ],
                ..Default::default()
            },
            ExpectedDocumentsData {
                document_name: "contactRequest",
                required_properties: &["firstName", "$createdAt", "$updatedAt", "lastName"],
                indexes: &[
                    (
                        "index1",
                        true,
                        &[("$ownerId", "asc"), ("firstName", "desc")],
                    ),
                    (
                        "index2",
                        true,
                        &[("$ownerId", "asc"), ("$lastName", "desc")],
                    ),
                    ("index3", false, &[("lastName", "asc")]),
                    (
                        "index4",
                        false,
                        &[("$createdAt", "asc"), ("$updatedAt", "asc")],
                    ),
                ],
            },
        ]
    }

    #[test]
    #[cfg(feature = "data-contract-cbor-conversion")]
    fn deserialize_from_cbor_with_contract_inner() {
        // Uncomment and run to regenerate

        // let platform_version = PlatformVersion::latest();
        // let dashpay_contract = json_document_to_contract(
        //     "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
        //     false,
        //     platform_version,
        // )
        //     .expect("expected to get cbor document");
        //
        // let cbor = dashpay_contract.to_cbor(platform_version).expect("expected cbor");
        //
        // std::fs::write("../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-cbor.bin",cbor);

        let cbor_bytes = std::fs::read(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-cbor.bin",
        )
        .unwrap();
        let expect_id_base58 = "AcYUCSvAmUwryNsQqkqqD1o3BnFuzepGtR3Mhh2swLk6";
        let expect_owner_id_base58 = "AcYUCSvAmUwryNsQqkqqD1o3BnFuzepGtR3Mhh2swLk6";
        let expect_id = bs58::decode(expect_id_base58).into_vec().unwrap();
        let expect_owner_id = bs58::decode(expect_owner_id_base58).into_vec().unwrap();

        let platform_version = PlatformVersion::latest();

        let data_contract = DataContract::from_cbor(cbor_bytes, true, platform_version)
            .expect("contract should be deserialized");

        assert_eq!(0, data_contract.feature_version());
        assert_eq!(expect_id, data_contract.id().as_bytes());
        assert_eq!(expect_owner_id, data_contract.owner_id().as_bytes());

        assert_eq!(3, data_contract.document_types().len());

        for expect in expected_documents() {
            assert!(
                data_contract.has_document_type_for_name(expect.document_name),
                "'{}' document should be defined",
                expect.document_name
            );
            assert!(
                data_contract.has_document_type_for_name(expect.document_name),
                "'{}' document type should be defined",
                expect.document_name
            );

            // document_type  - Drive API
            let document_type = data_contract
                .document_type_for_name(expect.document_name)
                .unwrap();
            assert_eq!(expect.indexes.len(), document_type.indices().len());
        }
    }

    #[test]
    fn should_drive_api_methods_contain_contract_data_v0() {
        let platform_version = PlatformVersion::latest();

        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get a contract")
        .into_v0()
        .unwrap();

        assert!(contract.config.documents_mutable_contract_default());
        assert!(!contract.config.keeps_history());
        assert!(!contract.config().readonly()); // the contract shouldn't be readonly
        assert!(!contract.config.documents_keep_history_contract_default());
        assert_eq!(contract.document_types.len(), 3);
        assert!(contract.document_types.get("profile").is_some());
        assert!(contract
            .document_types
            .get("profile")
            .unwrap()
            .documents_mutable());
        assert!(contract.document_types.get("contactInfo").is_some());
        assert!(contract
            .document_types
            .get("contactInfo")
            .unwrap()
            .documents_mutable());
        assert!(contract.document_types.get("contactRequest").is_some());
        assert!(!contract
            .document_types
            .get("contactRequest")
            .unwrap()
            .documents_mutable());
        assert!(contract.document_types.get("non_existent_key").is_none());

        let contact_info_indices = &contract
            .document_types
            .get("contactInfo")
            .unwrap()
            .indices();
        assert_eq!(contact_info_indices.len(), 2);
        assert!(contact_info_indices[0].unique);
        assert!(!contact_info_indices[1].unique);
        assert_eq!(contact_info_indices[0].properties.len(), 3);

        assert_eq!(contact_info_indices[0].properties[0].name, "$ownerId");
        assert_eq!(
            contact_info_indices[0].properties[1].name,
            "rootEncryptionKeyIndex"
        );
        assert_eq!(
            contact_info_indices[0].properties[2].name,
            "derivationEncryptionKeyIndex"
        );

        assert!(contact_info_indices[0].properties[0].ascending);
    }

    #[test]
    #[cfg(feature = "data-contract-cbor-conversion")]
    fn mutability_properties_should_be_stored_and_restored_during_cbor_serialization() {
        let platform_version = PlatformVersion::latest();

        let mut contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to get a cbor document")
        .into_v0()
        .unwrap();

        assert!(!contract.config().readonly());
        assert!(!contract.config.keeps_history());
        assert!(contract.config.documents_mutable_contract_default());
        assert!(!contract.config.documents_keep_history_contract_default());

        contract.config.set_readonly(true);
        contract.config.set_keeps_history(true);
        contract
            .config
            .set_documents_mutable_contract_default(false);
        contract
            .config
            .set_documents_can_be_deleted_contract_default(false);
        contract
            .config
            .set_documents_keep_history_contract_default(true);

        let contract_cbor = contract
            .to_cbor(platform_version)
            .expect("serialization shouldn't fail");
        let deserialized_contract = DataContract::from_cbor(contract_cbor, true, platform_version)
            .expect("deserialization shouldn't fail");

        assert!(matches!(
            deserialized_contract.config(),
            DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: true,
                keeps_history: true,
                documents_mutable_contract_default: false,
                documents_keep_history_contract_default: true,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            })
        ));
    }

    #[test]
    fn mutability_properties_should_be_stored_and_restored_during_serialization() {
        let platform_version = PlatformVersion::latest();

        let mut contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to decode a contract");

        let contract_v0 = contract.as_v0_mut().unwrap();

        assert!(!contract_v0.config().readonly());
        assert!(!contract_v0.config.keeps_history());
        assert!(contract_v0.config.documents_mutable_contract_default());
        assert!(!contract_v0.config.documents_keep_history_contract_default());

        contract_v0.config.set_readonly(true);
        contract_v0.config.set_keeps_history(true);
        contract_v0
            .config
            .set_documents_mutable_contract_default(false);
        contract_v0
            .config
            .set_documents_can_be_deleted_contract_default(false);
        contract_v0
            .config
            .set_documents_keep_history_contract_default(true);

        let contract = contract
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("serialization shouldn't fail");
        let deserialized_contract =
            DataContract::versioned_deserialize(contract.as_slice(), false, platform_version)
                .expect("deserialization shouldn't fail");

        assert_eq!(
            deserialized_contract.as_v0().unwrap().config,
            DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: true,
                keeps_history: true,
                documents_mutable_contract_default: false,
                documents_keep_history_contract_default: true,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
                documents_can_be_deleted_contract_default: false,
            })
        );
    }
}
