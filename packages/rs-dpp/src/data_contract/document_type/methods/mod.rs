#[cfg(feature = "validation")]
mod validate_update;
mod versioned_methods;

use std::collections::BTreeMap;

use crate::data_contract::document_type::index::{Index, IndexProperty};
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::document::Document;
use crate::document::INITIAL_REVISION;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};
use crate::version::PlatformVersion;
use crate::ProtocolError;

use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::versioned_methods::DocumentTypeV0MethodsVersioned;
use crate::fee::Credits;
use crate::voting::vote_polls::VotePoll;
use platform_value::{Identifier, Value};

pub trait DocumentTypeBasicMethods: DocumentTypeV0Getters {
    fn unique_id_for_storage(&self) -> [u8; 32] {
        rand::random::<[u8; 32]>()
    }

    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8> {
        let mut bytes = index_level.identifier().to_be_bytes().to_vec();
        bytes.extend_from_slice(&base_event);
        bytes
    }

    fn initial_revision(&self) -> Option<Revision> {
        if self.requires_revision() {
            Some(INITIAL_REVISION)
        } else {
            None
        }
    }

    fn requires_revision(&self) -> bool {
        self.documents_mutable()
            || self.documents_transferable().is_transferable()
            || self.trade_mode().seller_sets_price()
    }

    fn top_level_indices(&self) -> Vec<&IndexProperty> {
        self.indexes()
            .values()
            .filter_map(|index| index.properties.first())
            .collect()
    }

    // This should normally just be 1 item, however we keep a vec in case we want to change things
    //  in the future.
    fn top_level_indices_of_contested_unique_indexes(&self) -> Vec<&IndexProperty> {
        self.indexes()
            .values()
            .filter_map(|index| {
                if index.contested_index.is_some() {
                    index.properties.first()
                } else {
                    None
                }
            })
            .collect()
    }
}

// TODO: Some of those methods are only for tests. Hide under feature
pub trait DocumentTypeV0Methods: DocumentTypeV0Getters + DocumentTypeV0MethodsVersioned {
    fn index_for_types(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .index_for_types
        {
            0 => Ok(self.index_for_types_v0(index_names, in_field_name, order_by)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "store_ephemeral_state".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// [`Self::index_for_types`] restricted to the indexes `filter` admits.
    ///
    /// Candidates rejected by `filter` are skipped before they are scored, so
    /// they can never be returned. This is the only correct way to require a
    /// property of the selected index: because several indexes can cover the
    /// same fields and ties are broken by the index map's name ordering,
    /// checking the property after an unrestricted search can reject the
    /// winner but cannot surface the index the caller actually needed.
    ///
    /// Shares the `index_for_types` feature-version gate — the filter narrows
    /// the candidate set, it does not change how a candidate is scored.
    fn index_for_types_matching(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
        filter: impl Fn(&Index) -> bool,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .index_for_types
        {
            0 => Ok(self.index_for_types_matching_v0(index_names, in_field_name, order_by, filter)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "index_for_types_matching".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn serialize_value_for_key(
        &self,
        key: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .serialize_value_for_key
        {
            0 => self.serialize_value_for_key_v0(key, value),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "serialize_value_for_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
    fn deserialize_value_for_key(
        &self,
        key: &str,
        serialized_value: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Value, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .deserialize_value_for_key
        {
            0 => self.deserialize_value_for_key_v0(key, serialized_value),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "deserialize_value_for_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .max_size
        {
            0 => self.max_size_v0(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "max_size".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn estimated_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .estimated_size
        {
            0 => self.estimated_size_v0(platform_version),
            1 => self.estimated_size_v1(platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "estimated_size".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn create_document_from_data(
        &self,
        data: Value,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .create_document_from_data
        {
            0 => self.create_document_from_data_v0(
                data,
                owner_id,
                block_height,
                core_block_height,
                document_entropy,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_from_data".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Creates a document at the current time based on specified document type information.
    /// This function requires that all properties provided are pre-validated according to
    /// the document's schema requirements.
    ///
    /// # Parameters:
    /// - `id`: An identifier for the document. Unique within the context of the document's type.
    /// - `owner_id`: The identifier of the entity that will own this document.
    /// - `block_height`: The block height at which this document is considered to have been created.
    ///   While this value is recorded in the document, it is ignored when the document is broadcasted
    ///   to the network. This is because the actual block height at the time of broadcast may differ.
    ///   This parameter is included to fulfill schema requirements that specify a block height; you may
    ///   use the current block height, a placeholder value of 0, or any other value as necessary.
    /// - `core_block_height`: Similar to `block_height`, this represents the core network's block height
    ///   at the document's creation time. It is handled the same way as `block_height` regarding broadcast
    ///   and schema requirements.
    /// - `properties`: A collection of properties for the document, structured as a `BTreeMap<String, Value>`.
    ///   These must be pre-validated to match the document's schema definitions.
    /// - `platform_version`: A reference to the current version of the platform for which the document is created.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` if the document was successfully created, or an error
    /// indicating what went wrong during the creation process.
    ///
    /// # Note:
    /// The `block_height` and `core_block_height` are primarily included for schema compliance and local record-keeping.
    /// These values are not used when the document is broadcasted to the network, as the network assigns its own block
    /// heights upon receipt and processing of the document. After broadcasting, it is recommended to update these fields
    /// in their created_at/updated_at variants as well as the base created_at/updated_at in the client-side
    /// representation of the document to reflect the values returned by the network. The base created_at/updated_at
    /// uses current time when creating the local document and is also ignored as it is also set network side.
    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .create_document_with_prevalidated_properties
        {
            0 => self.create_document_with_prevalidated_properties_v0(
                id,
                owner_id,
                block_height,
                core_block_height,
                properties,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_with_prevalidated_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Figures out the minimum prefunded voting balance needed for a document
    fn prefunded_voting_balance_for_document(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(String, Credits)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .prefunded_voting_balance_for_document
        {
            0 => Ok(self.prefunded_voting_balance_for_document_v0(document, platform_version)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "prefunded_voting_balances_for_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Gets the vote poll associated with a document
    fn contested_vote_poll_for_document(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Result<Option<VotePoll>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .contested_vote_poll_for_document
        {
            0 => Ok(self.contested_vote_poll_for_document_v0(document)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "contested_vote_poll_for_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
    /// Gets the vote poll associated with a document
    fn contested_vote_poll_for_document_properties(
        &self,
        document_properties: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<VotePoll>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
            .contested_vote_poll_for_document
        {
            0 => Ok(self.contested_vote_poll_for_document_properties_v0(document_properties)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "contested_vote_poll_for_document_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn sanitize_document_properties(&self, properties: &mut BTreeMap<String, Value>) {
        // Iterate through each property in the document
        for (field_name, field_value) in properties.iter_mut() {
            // Get the property definition from the document type schema
            if let Some(property_def) = self.properties().get(field_name) {
                // Sanitize the value based on its property type
                property_def.property_type.sanitize_value_mut(field_value);
            }
            // If the property is not in the schema, leave it as is
            // (validation will catch unknown properties later)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::document_type::DocumentType;
    use platform_value::{platform_value, Identifier};

    /// Build a document type from a schema using latest platform version.
    fn build_doc_type(name: &str, schema: Value) -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("should create default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            name,
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("should build doc type")
    }

    // --------------------------------------------------------------
    // DocumentTypeBasicMethods::requires_revision / initial_revision
    // --------------------------------------------------------------
    #[test]
    fn requires_revision_false_when_immutable_non_transferable_and_no_trade_mode() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "transferable": 0_u64,
            "tradeMode": 0_u64,
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("immutable_doc", schema);
        // Requires revision false => initial_revision must be None
        assert!(!dt.as_ref().requires_revision_ref());
        assert_eq!(dt.as_ref().initial_revision_ref(), None);
    }

    #[test]
    fn requires_revision_true_when_mutable() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("mutable_doc", schema);
        assert!(dt.as_ref().requires_revision_ref());
        assert_eq!(dt.as_ref().initial_revision_ref(), Some(INITIAL_REVISION));
    }

    #[test]
    fn requires_revision_true_when_transferable_even_if_immutable() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "transferable": 1_u64,
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("transferable_doc", schema);
        assert!(dt.as_ref().requires_revision_ref());
        assert_eq!(dt.as_ref().initial_revision_ref(), Some(INITIAL_REVISION));
    }

    #[test]
    fn requires_revision_true_when_trade_mode_seller_sets_price() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "tradeMode": 1_u64, // DirectPurchase -> seller_sets_price = true
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("nft_doc", schema);
        assert!(dt.as_ref().requires_revision_ref());
    }

    // --------------------------------------------------------------
    // DocumentTypeBasicMethods::top_level_indices and
    // top_level_indices_of_contested_unique_indexes
    // --------------------------------------------------------------
    #[test]
    fn top_level_indices_returns_first_property_of_each_index() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "first_name": {"type": "string", "position": 0, "maxLength": 60_u32},
                "last_name": {"type": "string", "position": 1, "maxLength": 60_u32}
            },
            "indices": [
                {
                    "name": "byFirst",
                    "properties": [{"first_name": "asc"}],
                },
                {
                    "name": "byLast",
                    "properties": [{"last_name": "asc"}],
                },
            ],
            "additionalProperties": false,
        });
        let dt = build_doc_type("person", schema);
        let dt_ref = dt.as_ref();
        let top: Vec<&IndexProperty> = dt_ref.top_level_indices_ref();
        // Two indices each contribute their first property
        assert_eq!(top.len(), 2);
        let names: Vec<&str> = top.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"first_name"));
        assert!(names.contains(&"last_name"));
    }

    #[test]
    fn top_level_indices_of_contested_unique_indexes_excludes_non_contested() {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": false,
            "properties": {
                "first_name": {"type": "string", "position": 0, "maxLength": 60_u32},
                "last_name": {"type": "string", "position": 1, "maxLength": 60_u32}
            },
            "indices": [
                {
                    "name": "byFirst",
                    "properties": [{"first_name": "asc"}],
                },
                {
                    "name": "byLast",
                    "properties": [{"last_name": "asc"}],
                },
            ],
            "additionalProperties": false,
        });
        let dt = build_doc_type("person_no_contested", schema);
        let dt_ref = dt.as_ref();
        let contested = dt_ref.top_level_indices_of_contested_unique_indexes_ref();
        // Neither index is contested
        assert!(contested.is_empty());
    }

    // --------------------------------------------------------------
    // DocumentTypeBasicMethods::unique_id_for_document_field
    // --------------------------------------------------------------
    #[test]
    fn unique_id_for_document_field_concatenates_identifier_and_base_event() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "field_a": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("events", schema);
        let dt_ref = dt.as_ref();
        let index_level = dt_ref.index_structure_ref();
        let base_event: [u8; 32] = [7; 32];
        let out = dt_ref.unique_id_for_document_field_ref(index_level, base_event);
        // Output must be 8 bytes (u64 identifier) + 32 bytes (base_event) = 40
        assert_eq!(out.len(), 8 + 32);
        // Last 32 bytes must match base_event exactly
        assert_eq!(&out[8..], &base_event);
        // First 8 bytes must be identifier BE bytes
        let id_bytes = index_level.identifier().to_be_bytes();
        assert_eq!(&out[..8], &id_bytes);
    }

    // --------------------------------------------------------------
    // DocumentTypeV0Methods::sanitize_document_properties
    // --------------------------------------------------------------
    #[test]
    fn sanitize_document_properties_converts_hex_bytearray_to_bytes() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 1_u32,
                    "maxItems": 64_u32,
                    "position": 0
                }
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("blob_doc", schema);
        // 4-byte hex string
        let mut props: BTreeMap<String, Value> = BTreeMap::new();
        props.insert("payload".to_string(), Value::Text("deadbeef".to_string()));

        dt.as_ref().sanitize_document_properties_ref(&mut props);

        let got = props.get("payload").unwrap();
        match got {
            Value::Bytes(bytes) => assert_eq!(bytes.as_slice(), &[0xde, 0xad, 0xbe, 0xef]),
            other => panic!("expected sanitized Bytes, got {:?}", other),
        }
    }

    #[test]
    fn sanitize_document_properties_converts_integer_array_bytearray_to_bytes() {
        // A binary property re-hydrated through a schemaless JSON layer (an edited
        // and replaced cached document) arrives as a plain array of numbers that
        // decode to wider Value integer variants (U64), not Value::U8. Sanitize must
        // normalize it to Value::Bytes so the strict binary serializer accepts it;
        // otherwise the replace fails with "not an array of bytes".
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 1_u32,
                    "maxItems": 64_u32,
                    "position": 0
                }
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("blob_doc", schema);
        let mut props: BTreeMap<String, Value> = BTreeMap::new();
        props.insert(
            "payload".to_string(),
            Value::Array(vec![
                Value::U64(0xde),
                Value::U64(0xad),
                Value::U64(0xbe),
                Value::U64(0xef),
            ]),
        );

        dt.as_ref().sanitize_document_properties_ref(&mut props);

        match props.get("payload").unwrap() {
            Value::Bytes(bytes) => assert_eq!(bytes.as_slice(), &[0xde, 0xad, 0xbe, 0xef]),
            other => panic!("expected sanitized Bytes, got {:?}", other),
        }
    }

    #[test]
    fn sanitize_document_properties_leaves_unknown_fields_untouched() {
        let schema = platform_value!({
            "type": "object",
            "properties": {
                "known": {"type": "string", "position": 0, "maxLength": 20_u32}
            },
            "additionalProperties": false,
        });
        let dt = build_doc_type("known_only", schema);
        let mut props: BTreeMap<String, Value> = BTreeMap::new();
        props.insert(
            "unknown_field".to_string(),
            Value::Text("abcdef".to_string()),
        );
        props.insert("known".to_string(), Value::Text("hello".to_string()));

        dt.as_ref().sanitize_document_properties_ref(&mut props);

        // unknown_field should be unchanged
        assert_eq!(
            props.get("unknown_field").unwrap(),
            &Value::Text("abcdef".to_string())
        );
        // known stays a string (no sanitization applies for String type)
        assert_eq!(
            props.get("known").unwrap(),
            &Value::Text("hello".to_string())
        );
    }

    // --------------------------------------------------------------
    // Helper extensions so we can dispatch to the underlying
    // DocumentTypeV0/V1 via the enum without exposing new API.
    // (Implemented inline for the tests.)
    // --------------------------------------------------------------
    trait DocumentTypeTestHelpers {
        fn requires_revision_ref(&self) -> bool;
        fn initial_revision_ref(&self) -> Option<Revision>;
        fn top_level_indices_ref(&self) -> Vec<&IndexProperty>;
        fn top_level_indices_of_contested_unique_indexes_ref(&self) -> Vec<&IndexProperty>;
        fn index_structure_ref(&self) -> &IndexLevel;
        fn unique_id_for_document_field_ref(
            &self,
            index_level: &IndexLevel,
            base_event: [u8; 32],
        ) -> Vec<u8>;
        fn sanitize_document_properties_ref(&self, properties: &mut BTreeMap<String, Value>);
    }

    impl<'a> DocumentTypeTestHelpers for crate::data_contract::document_type::DocumentTypeRef<'a> {
        fn requires_revision_ref(&self) -> bool {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.requires_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.requires_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.requires_revision()
                }
            }
        }

        fn initial_revision_ref(&self) -> Option<Revision> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.initial_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.initial_revision()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.initial_revision()
                }
            }
        }

        fn top_level_indices_ref(&self) -> Vec<&IndexProperty> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.top_level_indices()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.top_level_indices()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.top_level_indices()
                }
            }
        }

        fn top_level_indices_of_contested_unique_indexes_ref(&self) -> Vec<&IndexProperty> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.top_level_indices_of_contested_unique_indexes()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.top_level_indices_of_contested_unique_indexes()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.top_level_indices_of_contested_unique_indexes()
                }
            }
        }

        fn index_structure_ref(&self) -> &IndexLevel {
            use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.index_structure()
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.index_structure()
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.index_structure()
                }
            }
        }

        fn unique_id_for_document_field_ref(
            &self,
            index_level: &IndexLevel,
            base_event: [u8; 32],
        ) -> Vec<u8> {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.unique_id_for_document_field(index_level, base_event)
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.unique_id_for_document_field(index_level, base_event)
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.unique_id_for_document_field(index_level, base_event)
                }
            }
        }

        fn sanitize_document_properties_ref(&self, properties: &mut BTreeMap<String, Value>) {
            match self {
                crate::data_contract::document_type::DocumentTypeRef::V0(v0) => {
                    v0.sanitize_document_properties(properties)
                }
                crate::data_contract::document_type::DocumentTypeRef::V1(v1) => {
                    v1.sanitize_document_properties(properties)
                }
                crate::data_contract::document_type::DocumentTypeRef::V2(v2) => {
                    v2.sanitize_document_properties(properties)
                }
            }
        }
    }
}
