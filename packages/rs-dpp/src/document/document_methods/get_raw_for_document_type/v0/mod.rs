use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::DocumentV0Getters;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;

pub trait DocumentGetRawForDocumentTypeV0: DocumentV0Getters {
    /// Return a value given the path to its key for a document type.
    fn get_raw_for_document_type_v0(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<u8>>, ProtocolError> {
        // todo: maybe merge with document_type.serialize_value_for_key() because we use different
        //   code paths for query and index creation
        // returns the owner id if the key path is $ownerId and an owner id is given
        if key_path == "$ownerId" {
            if let Some(owner_id) = owner_id {
                return Ok(Some(Vec::from(owner_id)));
            }
        }

        match key_path {
            // returns self.id or self.owner_id if key path is $id or $ownerId
            "$id" => return Ok(Some(self.id().to_vec())),
            "$ownerId" => return Ok(Some(self.owner_id().to_vec())),
            "$creatorId" => return Ok(self.creator_id().map(|id| id.to_vec())),
            "$createdAt" => {
                return Ok(self
                    .created_at()
                    .map(DocumentPropertyType::encode_date_timestamp))
            }
            "$createdAtBlockHeight" => {
                return Ok(self
                    .created_at_block_height()
                    .map(DocumentPropertyType::encode_u64))
            }
            "$createdAtCoreBlockHeight" => {
                return Ok(self
                    .created_at_core_block_height()
                    .map(DocumentPropertyType::encode_u32))
            }
            "$updatedAt" => {
                return Ok(self
                    .updated_at()
                    .map(DocumentPropertyType::encode_date_timestamp))
            }
            "$updatedAtBlockHeight" => {
                return Ok(self
                    .updated_at_block_height()
                    .map(DocumentPropertyType::encode_u64))
            }
            "$updatedAtCoreBlockHeight" => {
                return Ok(self
                    .updated_at_core_block_height()
                    .map(DocumentPropertyType::encode_u32))
            }
            "$transferredAt" => {
                return Ok(self
                    .transferred_at()
                    .map(DocumentPropertyType::encode_date_timestamp))
            }
            "$transferredAtBlockHeight" => {
                return Ok(self
                    .transferred_at_block_height()
                    .map(DocumentPropertyType::encode_u64))
            }
            "$transferredAtCoreBlockHeight" => {
                return Ok(self
                    .transferred_at_core_block_height()
                    .map(DocumentPropertyType::encode_u32))
            }
            _ => {}
        }
        self.properties()
            .get_optional_at_path(key_path)?
            .map(|value| document_type.serialize_value_for_key(key_path, value, platform_version))
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::DocumentV0;
    use crate::tests::json_document::json_document_to_contract;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_document_with_known_ids() -> DocumentV0 {
        DocumentV0 {
            contract_version: None,
            id: Identifier::new([0xAA; 32]),
            owner_id: Identifier::new([0xBB; 32]),
            properties: BTreeMap::new(),
            revision: None,
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_100_000),
            transferred_at: Some(1_700_000_200_000),
            created_at_block_height: Some(100),
            updated_at_block_height: Some(200),
            transferred_at_block_height: Some(300),
            created_at_core_block_height: Some(50),
            updated_at_core_block_height: Some(60),
            transferred_at_core_block_height: Some(70),
            creator_id: Some(Identifier::new([0xCC; 32])),
        }
    }

    // ================================================================
    //  System field extraction: $id, $ownerId, $creatorId
    // ================================================================

    #[test]
    fn get_raw_returns_id_for_dollar_id() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let raw = doc
            .get_raw_for_document_type_v0("$id", document_type, None, platform_version)
            .expect("should succeed");
        assert_eq!(
            raw,
            Some(doc.id.to_vec()),
            "$id should return the document id bytes"
        );
    }

    #[test]
    fn get_raw_returns_owner_id_for_dollar_owner_id() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let raw = doc
            .get_raw_for_document_type_v0("$ownerId", document_type, None, platform_version)
            .expect("should succeed");
        assert_eq!(raw, Some(doc.owner_id.to_vec()));
    }

    #[test]
    fn get_raw_returns_override_owner_id_when_provided() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let override_owner = [0xFF; 32];
        let raw = doc
            .get_raw_for_document_type_v0(
                "$ownerId",
                document_type,
                Some(override_owner),
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(
            raw,
            Some(Vec::from(override_owner)),
            "explicit owner_id should override the document's owner_id"
        );
    }

    #[test]
    fn get_raw_returns_creator_id() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let raw = doc
            .get_raw_for_document_type_v0("$creatorId", document_type, None, platform_version)
            .expect("should succeed");
        assert_eq!(raw, Some(Identifier::new([0xCC; 32]).to_vec()));
    }

    // ================================================================
    //  Timestamp fields
    // ================================================================

    #[test]
    fn get_raw_returns_encoded_created_at() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let raw = doc
            .get_raw_for_document_type_v0("$createdAt", document_type, None, platform_version)
            .expect("should succeed");
        assert!(raw.is_some(), "$createdAt should produce bytes");
        let expected = DocumentPropertyType::encode_date_timestamp(1_700_000_000_000);
        assert_eq!(raw.unwrap(), expected);
    }

    #[test]
    fn get_raw_returns_encoded_updated_at() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let raw = doc
            .get_raw_for_document_type_v0("$updatedAt", document_type, None, platform_version)
            .expect("should succeed");
        assert!(raw.is_some());
        let expected = DocumentPropertyType::encode_date_timestamp(1_700_000_100_000);
        assert_eq!(raw.unwrap(), expected);
    }

    #[test]
    fn get_raw_returns_encoded_block_heights() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();

        // $createdAtBlockHeight -> encode_u64(100)
        let raw = doc
            .get_raw_for_document_type_v0(
                "$createdAtBlockHeight",
                document_type,
                None,
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(raw, Some(DocumentPropertyType::encode_u64(100)));

        // $updatedAtBlockHeight -> encode_u64(200)
        let raw = doc
            .get_raw_for_document_type_v0(
                "$updatedAtBlockHeight",
                document_type,
                None,
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(raw, Some(DocumentPropertyType::encode_u64(200)));

        // $createdAtCoreBlockHeight -> encode_u32(50)
        let raw = doc
            .get_raw_for_document_type_v0(
                "$createdAtCoreBlockHeight",
                document_type,
                None,
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(raw, Some(DocumentPropertyType::encode_u32(50)));

        // $updatedAtCoreBlockHeight -> encode_u32(60)
        let raw = doc
            .get_raw_for_document_type_v0(
                "$updatedAtCoreBlockHeight",
                document_type,
                None,
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(raw, Some(DocumentPropertyType::encode_u32(60)));
    }

    #[test]
    fn get_raw_returns_encoded_transferred_fields() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();

        let raw = doc
            .get_raw_for_document_type_v0("$transferredAt", document_type, None, platform_version)
            .expect("should succeed");
        assert_eq!(
            raw,
            Some(DocumentPropertyType::encode_date_timestamp(
                1_700_000_200_000
            ))
        );

        let raw = doc
            .get_raw_for_document_type_v0(
                "$transferredAtBlockHeight",
                document_type,
                None,
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(raw, Some(DocumentPropertyType::encode_u64(300)));

        let raw = doc
            .get_raw_for_document_type_v0(
                "$transferredAtCoreBlockHeight",
                document_type,
                None,
                platform_version,
            )
            .expect("should succeed");
        assert_eq!(raw, Some(DocumentPropertyType::encode_u32(70)));
    }

    // ================================================================
    //  Non-existent property returns None
    // ================================================================

    #[test]
    fn get_raw_returns_none_for_missing_property() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let raw = doc
            .get_raw_for_document_type_v0("nonExistentField", document_type, None, platform_version)
            .expect("should succeed");
        assert_eq!(raw, None);
    }

    // ================================================================
    //  User-defined property serialization
    // ================================================================

    // ================================================================
    //  None-valued system fields should return None (not panic).
    // ================================================================

    fn minimal_doc() -> DocumentV0 {
        DocumentV0 {
            contract_version: None,
            id: Identifier::new([1u8; 32]),
            owner_id: Identifier::new([2u8; 32]),
            properties: BTreeMap::new(),
            revision: None,
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
        }
    }

    #[test]
    fn get_raw_returns_none_for_unset_optional_system_fields() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = minimal_doc();
        let keys = [
            "$createdAt",
            "$updatedAt",
            "$transferredAt",
            "$createdAtBlockHeight",
            "$updatedAtBlockHeight",
            "$transferredAtBlockHeight",
            "$createdAtCoreBlockHeight",
            "$updatedAtCoreBlockHeight",
            "$transferredAtCoreBlockHeight",
            "$creatorId",
        ];
        for k in keys {
            let raw = doc
                .get_raw_for_document_type_v0(k, document_type, None, platform_version)
                .expect("should succeed");
            assert_eq!(raw, None, "{k} should yield None when unset");
        }
    }

    // ================================================================
    //  get_raw_for_document_type_v0: $ownerId with owner_id override
    //  takes precedence even if the document's owner is different, but
    //  for $id the override is NOT applied (only $ownerId is gated).
    // ================================================================

    #[test]
    fn get_raw_owner_id_override_only_affects_dollar_owner_id_path() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let doc = make_document_with_known_ids();
        let override_owner = [0xFF; 32];

        // $ownerId path sees the override
        let raw_owner = doc
            .get_raw_for_document_type_v0(
                "$ownerId",
                document_type,
                Some(override_owner),
                platform_version,
            )
            .expect("owner should succeed");
        assert_eq!(raw_owner, Some(Vec::from(override_owner)));

        // $id path ignores the owner override
        let raw_id = doc
            .get_raw_for_document_type_v0(
                "$id",
                document_type,
                Some(override_owner),
                platform_version,
            )
            .expect("id should succeed");
        assert_eq!(
            raw_id,
            Some(doc.id.to_vec()),
            "$id path should not be affected by owner_id override"
        );
    }

    // ================================================================
    //  get_raw_for_contract with unknown document_type_name errors.
    // ================================================================

    #[test]
    fn get_raw_for_contract_with_unknown_document_type_errors() {
        use crate::document::document_methods::DocumentGetRawForContractV0;
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");

        let doc = make_document_with_known_ids();
        let err = doc
            .get_raw_for_contract_v0("$id", "nonExistentType", &contract, None, platform_version)
            .expect_err("unknown document type should fail");
        match err {
            crate::ProtocolError::DataContractError(
                crate::data_contract::errors::DataContractError::DocumentTypeNotFound(_),
            ) => {}
            other => panic!("expected DocumentTypeNotFound, got {:?}", other),
        }
    }

    #[test]
    fn get_raw_serializes_user_defined_property() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected contract");
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected document type");

        let document = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");

        let crate::document::Document::V0(doc_v0) = &document;

        // "displayName" is a required string property in dashpay profile
        let raw = doc_v0
            .get_raw_for_document_type_v0("displayName", document_type, None, platform_version)
            .expect("should succeed");
        assert!(raw.is_some(), "displayName should produce serialized bytes");
    }
}
