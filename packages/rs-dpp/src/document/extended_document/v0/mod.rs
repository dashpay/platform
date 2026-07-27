#[cfg(feature = "value-conversion")]
mod platform_value_conversion;
mod serialize;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
#[cfg(any(feature = "value-conversion", feature = "json-conversion"))]
use crate::document::extended_document::fields::property_names;
use crate::document::{Document, DocumentV0Getters};
use crate::identity::TimestampMillis;
use crate::metadata::Metadata;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};

use crate::util::hash::hash_double_to_vec;
use crate::ProtocolError;

use platform_value::btreemap_extensions::{
    BTreeValueMapInsertionPathHelper, BTreeValueMapPathHelper,
};
#[cfg(feature = "value-conversion")]
use platform_value::btreemap_extensions::{
    BTreeValueMapReplacementPathHelper, BTreeValueRemoveFromMapHelper,
};
use platform_value::{Bytes32, Identifier, ReplacementType, Value};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
#[cfg(feature = "validation")]
use crate::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
#[cfg(feature = "value-conversion")]
use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;
use crate::tokens::token_payment_info::TokenPaymentInfo;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
#[cfg(feature = "json-conversion")]
use platform_value::converter::serde_json::BTreeValueJsonConverter;
use platform_version::version::PlatformVersion;
#[cfg(feature = "json-conversion")]
use serde_json::Value as JsonValue;

/// The `ExtendedDocumentV0` struct represents the data provided by the platform in response to a query.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub struct ExtendedDocumentV0 {
    /// The document type name, stored as a string.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$type"))]
    pub document_type_name: String,

    /// The identifier of the associated data contract.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$dataContractId"))]
    pub data_contract_id: Identifier,

    /// The actual document object containing the data.
    #[cfg_attr(feature = "serde-conversion", serde(flatten))]
    pub document: Document,

    // TODO: We should remove it from here, or at least keep a ref
    //  also there is no point to keep both contract and its ID
    /// The data contract associated with the document.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$dataContract"))]
    pub data_contract: DataContract,

    /// An optional field for metadata associated with the document.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$metadata", default))]
    pub metadata: Option<Metadata>,

    /// A field representing the entropy, stored as `Bytes32`.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$entropy"))]
    pub entropy: Bytes32,
    /// A field representing the token payment info.
    #[cfg_attr(feature = "serde-conversion", serde(rename = "$tokenPaymentInfo"))]
    pub token_payment_info: Option<TokenPaymentInfo>,
}

/// Ensures a Document `Value::Map` carries the canonical
/// `$formatVersion: "0"` tag before it's passed to canonical
/// `Document::from_object` (which is `tag = "$formatVersion"`).
///
/// Used by ExtendedDocument's `from_trusted_platform_value` /
/// `from_untrusted_platform_value` to accept legacy un-tagged shapes
/// (DPNS / DashPay JSON test fixtures, untrusted JS user input via
/// wasm-dpp legacy) and route them through the canonical deserializer.
/// V0 is the only Document structure version, so unconditionally
/// inserting `"0"` is correct today; if a future structure version is
/// introduced, this helper needs to grow a version-selection step.
#[cfg(feature = "value-conversion")]
fn ensure_document_format_version(document_value: &mut Value) -> Result<(), ProtocolError> {
    use platform_value::ValueMapHelper;
    if let Value::Map(map) = document_value {
        let has_tag = map
            .iter()
            .any(|(k, _)| k.as_text() == Some("$formatVersion"));
        if !has_tag {
            map.insert_string_key_value("$formatVersion".to_string(), Value::Text("0".to_string()));
        }
    }
    Ok(())
}

impl ExtendedDocumentV0 {
    #[cfg(feature = "json-conversion")]
    pub(super) fn properties_as_json_data(&self) -> Result<JsonValue, ProtocolError> {
        self.document
            .properties()
            .to_json_value()
            .map_err(ProtocolError::ValueError)
    }

    pub fn get_optional_value(&self, key: &str) -> Option<&Value> {
        self.document.properties().get(key)
    }

    pub fn properties(&self) -> &BTreeMap<String, Value> {
        self.document.properties()
    }

    pub fn properties_as_mut(&mut self) -> &mut BTreeMap<String, Value> {
        self.document.properties_mut()
    }

    pub fn id(&self) -> Identifier {
        self.document.id()
    }

    pub fn owner_id(&self) -> Identifier {
        self.document.owner_id()
    }

    pub fn document_type(&self) -> Result<DocumentTypeRef<'_>, ProtocolError> {
        // We can unwrap because the Document can not be created without a valid Document Type
        self.data_contract
            .document_type_for_name(self.document_type_name.as_str())
            .map_err(ProtocolError::DataContractError)
    }

    pub fn can_be_modified(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable())
    }

    pub fn requires_revision(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.requires_revision())
    }

    pub fn revision(&self) -> Option<Revision> {
        self.document.revision()
    }

    /// Returns an optional block timestamp at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at(&self) -> Option<TimestampMillis> {
        self.document.created_at()
    }

    /// Returns an optional block timestamp at which the document was updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at(&self) -> Option<TimestampMillis> {
        self.document.updated_at()
    }

    /// Returns an optional block height at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at_block_height(&self) -> Option<BlockHeight> {
        self.document.created_at_block_height()
    }

    /// Returns an optional block height at which the document was last updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at_block_height(&self) -> Option<BlockHeight> {
        self.document.updated_at_block_height()
    }

    /// Returns an optional core block height at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        self.document.created_at_core_block_height()
    }

    /// Returns an optional core block height at which the document was last updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        self.document.updated_at_core_block_height()
    }

    /// Create an extended document with additional information.
    ///
    /// # Arguments
    ///
    /// * `document` - A `Document` instance.
    /// * `data_contract` - A `DataContract` instance.
    /// * `document_type_name` - A `String` representing the document type name.
    pub fn from_document_with_additional_info(
        document: Document,
        data_contract: DataContract,
        document_type_name: String,
        token_payment_info: Option<TokenPaymentInfo>,
    ) -> Self {
        Self {
            document_type_name,
            data_contract_id: data_contract.id(),
            document,
            data_contract,
            metadata: None,
            entropy: Default::default(),
            token_payment_info,
        }
    }

    #[cfg(feature = "json-conversion")]
    /// Create an extended document from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `string` - A JSON string representing the extended document.
    /// * `contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error decoding the JSON string.
    pub fn from_json_string(
        string: &str,
        contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let json_value: JsonValue = serde_json::from_str(string).map_err(|_| {
            ProtocolError::StringDecodeError("error decoding from json string".to_string())
        })?;
        Self::from_untrusted_platform_value(json_value.into(), contract, platform_version)
    }

    #[cfg(feature = "json-conversion")]
    /// Create an extended document from a raw JSON document.
    ///
    /// # Arguments
    ///
    /// * `raw_document` - A `JsonValue` representing the raw document.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the raw JSON document.
    pub fn from_raw_json_document(
        raw_document: JsonValue,
        data_contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Self::from_untrusted_platform_value(raw_document.into(), data_contract, platform_version)
    }

    #[cfg(feature = "value-conversion")]
    /// Create an extended document from a trusted platform value object where fields are already in
    /// the proper format for the contract.
    ///
    /// # Arguments
    ///
    /// * `document_value` - A `Value` representing the document value.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the trusted platform value.
    pub fn from_trusted_platform_value(
        mut document_value: Value,
        data_contract: DataContract,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .clone()
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let token_payment_info = properties
            .remove_optional_map_as_btree_map_keep_values_as_platform_value(
                property_names::TOKEN_PAYMENT_INFO,
            )
            .ok()
            .flatten()
            .map(|map| map.try_into())
            .transpose()?;

        let document_type_name = properties
            .remove_string(property_names::DOCUMENT_TYPE_NAME)
            .map_err(ProtocolError::ValueError)?;

        // Insert the canonical `$formatVersion` tag (Phase D step 8 slice
        // B): the legacy `Document::from_platform_value` accepted
        // un-tagged shapes, but it has been deleted. Untrusted user
        // input lacks the tag, so we add it before routing through
        // canonical `Document::from_object`.
        ensure_document_format_version(&mut document_value)?;
        use crate::serialization::ValueConvertible;
        let document = Document::from_object(document_value)?;

        let data_contract_id = data_contract.id();
        let mut extended_document = Self {
            data_contract,
            document_type_name,
            document,
            data_contract_id,
            metadata: None,
            entropy: Default::default(),
            token_payment_info,
        };

        extended_document.data_contract_id = Identifier::new(
            properties
                .remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)?
                .unwrap_or(extended_document.data_contract.id().to_buffer()),
        );
        Ok(extended_document)
    }

    #[cfg(feature = "value-conversion")]
    /// Create an extended document from an untrusted platform value object where fields might not
    /// be in the proper format for the contract.
    ///
    /// # Arguments
    ///
    /// * `document_value` - A `Value` representing the document value.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the untrusted platform value.
    pub fn from_untrusted_platform_value(
        mut document_value: Value,
        data_contract: DataContract,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .clone()
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let document_type_name = properties
            .remove_string(property_names::DOCUMENT_TYPE_NAME)
            .map_err(ProtocolError::ValueError)?;

        let token_payment_info = properties
            .remove_optional_map_as_btree_map_keep_values_as_platform_value(
                property_names::TOKEN_PAYMENT_INFO,
            )
            .ok()
            .flatten()
            .map(|map| map.try_into())
            .transpose()?;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        let identifiers = document_type.identifier_paths().to_owned();
        let binary_paths = document_type.binary_paths();

        document_value.replace_at_paths(["$id", "$ownerId"], ReplacementType::Identifier)?;
        if document_value.has("$creatorId")? {
            document_value.replace_at_path("$creatorId", ReplacementType::Identifier)?;
        }
        document_value.replace_at_paths(
            identifiers.iter().map(|s| s.as_str()),
            ReplacementType::Identifier,
        )?;
        document_value.replace_at_paths(
            binary_paths.iter().map(|s| s.as_str()),
            ReplacementType::BinaryBytes,
        )?;

        ensure_document_format_version(&mut document_value)?;
        use crate::serialization::ValueConvertible;
        let document = Document::from_object(document_value)?;
        let data_contract_id = data_contract.id();
        let mut extended_document = Self {
            data_contract,
            document_type_name,
            document,
            data_contract_id,
            metadata: None,
            entropy: Default::default(),
            token_payment_info,
        };

        extended_document.data_contract_id = properties
            .remove_optional_identifier(property_names::DATA_CONTRACT_ID)?
            .unwrap_or(extended_document.data_contract.id());
        extended_document
            .document
            .properties_mut()
            .replace_at_paths(&identifiers, ReplacementType::Identifier)?;
        Ok(extended_document)
    }

    #[cfg(feature = "value-conversion")]
    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut object = self.document.to_map_value()?;
        object.insert(
            property_names::DOCUMENT_TYPE_NAME.to_string(),
            Value::Text(self.document_type_name.clone()),
        );
        object.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(self.data_contract_id.to_buffer()),
        );
        if let Some(token_payment_info) = self.token_payment_info {
            object.insert(
                property_names::TOKEN_PAYMENT_INFO.to_string(),
                token_payment_info.try_into()?,
            );
        }
        Ok(object)
    }

    #[cfg(feature = "value-conversion")]
    pub fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let ExtendedDocumentV0 {
            document_type_name,
            data_contract_id,
            document,
            token_payment_info,
            ..
        } = self;

        let mut object = document.into_map_value()?;
        object.insert(property_names::FEATURE_VERSION.to_string(), Value::U16(0));
        object.insert(
            property_names::DOCUMENT_TYPE_NAME.to_string(),
            Value::Text(document_type_name),
        );
        object.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(data_contract_id.to_buffer()),
        );
        if let Some(token_payment_info) = token_payment_info {
            object.insert(
                property_names::TOKEN_PAYMENT_INFO.to_string(),
                token_payment_info.try_into()?,
            );
        }
        Ok(object)
    }

    pub fn hash(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_double_to_vec(
            ExtendedDocumentPlatformConversionMethodsV0::serialize_to_bytes(
                self,
                platform_version,
            )?,
        ))
    }

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        Ok(self.document.properties_mut().insert_at_path(path, value)?)
    }

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set_untrusted(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        let document_type = self.document_type()?;

        let identifiers = document_type.identifier_paths();
        let binary_paths = document_type.binary_paths();

        if identifiers.contains(path) {
            let value =
                ReplacementType::Identifier.replace_for_bytes(value.to_identifier_bytes()?)?;
            self.set(path, value)
        } else if binary_paths.contains(path) {
            let value =
                ReplacementType::BinaryBytes.replace_for_bytes(value.to_identifier_bytes()?)?;
            self.set(path, value)
        } else {
            self.set(path, value)
        }
    }

    /// Retrieves field specified by path
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.properties().get_optional_at_path(path).ok().flatten()
    }

    // TODO: We probably should validate extended document on creation and during modification
    //  instead of have a dedicated method. That would be more Rust way approach
    #[cfg(feature = "validation")]
    /// Validate external document
    pub fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        self.data_contract.validate_document(
            &self.document_type_name,
            &self.document,
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::document_type::random_document::CreateRandomDocument;
    use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;
    use crate::document::DocumentV0Getters;
    use crate::tests::json_document::json_document_to_contract;
    use platform_version::version::PlatformVersion;

    fn load_dashpay_contract(platform_version: &PlatformVersion) -> crate::prelude::DataContract {
        json_document_to_contract(
            "../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract.json",
            false,
            platform_version,
        )
        .expect("expected to load dashpay contract")
    }

    fn make_extended_document(
        platform_version: &PlatformVersion,
    ) -> (ExtendedDocumentV0, crate::prelude::DataContract) {
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected profile document type");
        let document = document_type
            .random_document(Some(42), platform_version)
            .expect("expected random document");
        let ext_doc = ExtendedDocumentV0::from_document_with_additional_info(
            document,
            contract.clone(),
            "profile".to_string(),
            None,
        );
        (ext_doc, contract)
    }

    // ================================================================
    //  Construction: from_document_with_additional_info
    // ================================================================

    #[test]
    fn from_document_with_additional_info_sets_fields_correctly() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, contract) = make_extended_document(platform_version);

        assert_eq!(ext_doc.document_type_name, "profile");
        assert_eq!(ext_doc.data_contract_id, contract.id());
        assert!(ext_doc.metadata.is_none());
        assert_eq!(ext_doc.entropy, Bytes32::default());
        assert!(ext_doc.token_payment_info.is_none());
    }

    // ================================================================
    //  Property access methods
    // ================================================================

    #[test]
    fn get_optional_value_returns_existing_property() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        // Random dashpay profile documents have "displayName"
        let display_name = ext_doc.get_optional_value("displayName");
        assert!(
            display_name.is_some(),
            "displayName should exist in random profile document"
        );
    }

    #[test]
    fn get_optional_value_returns_none_for_missing_key() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let missing = ext_doc.get_optional_value("nonExistentField");
        assert!(missing.is_none());
    }

    #[test]
    fn properties_returns_the_document_properties() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let props = ext_doc.properties();
        assert!(
            !props.is_empty(),
            "random profile document should have properties"
        );
    }

    #[test]
    fn properties_as_mut_allows_modification() {
        let platform_version = PlatformVersion::latest();
        let (mut ext_doc, _) = make_extended_document(platform_version);

        ext_doc
            .properties_as_mut()
            .insert("newField".to_string(), Value::Text("newValue".to_string()));
        assert_eq!(
            ext_doc.get_optional_value("newField"),
            Some(&Value::Text("newValue".to_string()))
        );
    }

    // ================================================================
    //  ID delegation methods
    // ================================================================

    #[test]
    fn id_and_owner_id_delegate_to_inner_document() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        assert_eq!(ext_doc.id(), ext_doc.document.id());
        assert_eq!(ext_doc.owner_id(), ext_doc.document.owner_id());
    }

    // ================================================================
    //  document_type lookup
    // ================================================================

    #[test]
    fn document_type_returns_correct_type_ref() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let doc_type = ext_doc
            .document_type()
            .expect("document_type should succeed for valid profile");
        assert_eq!(doc_type.name(), "profile");
    }

    #[test]
    fn document_type_fails_for_invalid_type_name() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected profile type");
        let document = document_type
            .random_document(Some(1), platform_version)
            .expect("random document");

        let ext_doc = ExtendedDocumentV0 {
            document_type_name: "nonExistentType".to_string(),
            data_contract_id: contract.id(),
            document,
            data_contract: contract,
            metadata: None,
            entropy: Default::default(),
            token_payment_info: None,
        };

        let result = ext_doc.document_type();
        assert!(
            result.is_err(),
            "document_type should fail for unknown type name"
        );
    }

    // ================================================================
    //  can_be_modified and requires_revision
    // ================================================================

    #[test]
    fn can_be_modified_returns_value_from_document_type() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        // The profile document type in dashpay is mutable
        let can_modify = ext_doc
            .can_be_modified()
            .expect("can_be_modified should succeed");
        assert!(can_modify, "dashpay profile should be mutable");
    }

    #[test]
    fn requires_revision_returns_value_from_document_type() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        // Mutable documents require revision
        let requires_rev = ext_doc
            .requires_revision()
            .expect("requires_revision should succeed");
        assert!(
            requires_rev,
            "mutable dashpay profile should require revision"
        );
    }

    // ================================================================
    //  Timestamp delegation methods
    // ================================================================

    #[test]
    fn timestamp_methods_delegate_to_inner_document() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        assert_eq!(ext_doc.created_at(), ext_doc.document.created_at());
        assert_eq!(ext_doc.updated_at(), ext_doc.document.updated_at());
        assert_eq!(ext_doc.revision(), ext_doc.document.revision());
        assert_eq!(
            ext_doc.created_at_block_height(),
            ext_doc.document.created_at_block_height()
        );
        assert_eq!(
            ext_doc.updated_at_block_height(),
            ext_doc.document.updated_at_block_height()
        );
        assert_eq!(
            ext_doc.created_at_core_block_height(),
            ext_doc.document.created_at_core_block_height()
        );
        assert_eq!(
            ext_doc.updated_at_core_block_height(),
            ext_doc.document.updated_at_core_block_height()
        );
    }

    // ================================================================
    //  set and get for path-based property access
    // ================================================================

    #[test]
    fn set_and_get_inserts_and_retrieves_value() {
        let platform_version = PlatformVersion::latest();
        let (mut ext_doc, _) = make_extended_document(platform_version);

        ext_doc
            .set("customPath", Value::U64(999))
            .expect("set should succeed");
        let val = ext_doc.get("customPath");
        assert_eq!(val, Some(&Value::U64(999)));
    }

    #[test]
    fn get_returns_none_for_nonexistent_path() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        assert!(ext_doc.get("no.such.path").is_none());
    }

    // ================================================================
    //  to_map_value and into_map_value
    // ================================================================

    #[test]
    fn to_map_value_contains_type_and_contract_id() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, contract) = make_extended_document(platform_version);

        let map = ext_doc.to_map_value().expect("to_map_value should succeed");
        assert_eq!(
            map.get(property_names::DOCUMENT_TYPE_NAME),
            Some(&Value::Text("profile".to_string()))
        );
        assert_eq!(
            map.get(property_names::DATA_CONTRACT_ID),
            Some(&Value::Identifier(contract.id().to_buffer()))
        );
    }

    #[test]
    fn into_map_value_contains_feature_version() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let map = ext_doc
            .into_map_value()
            .expect("into_map_value should succeed");
        assert_eq!(
            map.get(property_names::FEATURE_VERSION),
            Some(&Value::U16(0))
        );
    }

    // ================================================================
    //  properties_as_json_data
    // ================================================================

    #[test]
    fn properties_as_json_data_returns_json_with_properties() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let json_data = ext_doc
            .properties_as_json_data()
            .expect("properties_as_json_data should succeed");
        assert!(
            json_data.is_object(),
            "properties_as_json_data should return a JSON object"
        );
    }

    // ================================================================
    //  hash
    // ================================================================

    #[test]
    fn hash_produces_consistent_output() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let hash1 = ext_doc.hash(platform_version).expect("hash should succeed");
        let hash2 = ext_doc.hash(platform_version).expect("hash should succeed");
        assert_eq!(hash1, hash2, "hash should be deterministic");
        assert!(!hash1.is_empty(), "hash should not be empty");
    }

    // ================================================================
    //  from_json_string
    // ================================================================

    #[test]
    fn from_json_string_parses_valid_json() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let contract_id = contract.id();

        // Build a minimal valid JSON string for a "profile" document
        let json_str = format!(
            r#"{{
                "$type": "profile",
                "$dataContractId": "{}",
                "$id": "{}",
                "$ownerId": "{}",
                "$revision": 1,
                "displayName": "TestUser",
                "publicMessage": "Hello",
                "avatarUrl": "https://example.com/avatar.png"
            }}"#,
            bs58::encode(contract_id.to_buffer()).into_string(),
            bs58::encode([1u8; 32]).into_string(),
            bs58::encode([2u8; 32]).into_string(),
        );

        let ext_doc = ExtendedDocumentV0::from_json_string(&json_str, contract, platform_version)
            .expect("from_json_string should succeed");

        assert_eq!(ext_doc.document_type_name, "profile");
        assert!(ext_doc.get_optional_value("displayName").is_some());
    }

    #[test]
    fn from_json_string_rejects_invalid_json() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);

        let result =
            ExtendedDocumentV0::from_json_string("not valid json {{{", contract, platform_version);
        assert!(
            result.is_err(),
            "from_json_string should fail on invalid JSON"
        );
    }

    // ================================================================
    //  Serialization round-trip
    // ================================================================

    #[test]
    fn extended_document_serialize_deserialize_round_trip() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let serialized = ext_doc
            .serialize_to_bytes(platform_version)
            .expect("serialize_to_bytes should succeed");

        let recovered = ExtendedDocumentV0::from_bytes(&serialized, platform_version)
            .expect("from_bytes should succeed");

        assert_eq!(ext_doc.document_type_name, recovered.document_type_name);
        assert_eq!(ext_doc.data_contract_id, recovered.data_contract_id);
        assert_eq!(ext_doc.document.id(), recovered.document.id());
        assert_eq!(ext_doc.document.owner_id(), recovered.document.owner_id());
    }

    // ================================================================
    //  validate
    // ================================================================

    #[cfg(feature = "validation")]
    #[test]
    fn validate_returns_result_without_error() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        // validate() should not return a ProtocolError (it may return
        // validation errors for random data, but should not panic)
        let result = ext_doc.validate(platform_version);
        assert!(result.is_ok(), "validate should not return a ProtocolError");
    }

    // ================================================================
    //  from_trusted_platform_value
    // ================================================================

    #[test]
    fn from_trusted_platform_value_round_trip() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, contract) = make_extended_document(platform_version);

        let map_val = ext_doc.to_map_value().expect("to_map_value should succeed");
        let platform_val: Value = map_val.into();

        let recovered = ExtendedDocumentV0::from_trusted_platform_value(
            platform_val,
            contract,
            platform_version,
        )
        .expect("from_trusted_platform_value should succeed");

        assert_eq!(recovered.document_type_name, "profile");
        assert_eq!(recovered.document.id(), ext_doc.document.id());
    }

    // ================================================================
    //  from_raw_json_document
    // ================================================================

    #[test]
    fn from_raw_json_document_parses_json_value() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let contract_id = contract.id();

        let json_val: JsonValue = serde_json::json!({
            "$type": "profile",
            "$dataContractId": bs58::encode(contract_id.to_buffer()).into_string(),
            "$id": bs58::encode([1u8; 32]).into_string(),
            "$ownerId": bs58::encode([2u8; 32]).into_string(),
            "$revision": 1,
            "displayName": "Bob",
            "publicMessage": "Hi",
            "avatarUrl": "https://example.com/bob.png"
        });

        let ext_doc =
            ExtendedDocumentV0::from_raw_json_document(json_val, contract, platform_version)
                .expect("from_raw_json_document should succeed");

        assert_eq!(ext_doc.document_type_name, "profile");
    }

    // ================================================================
    //  from_untrusted_platform_value: fails when document type missing
    // ================================================================

    #[test]
    fn from_untrusted_platform_value_fails_for_missing_type_name() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);

        // Value with no $type field — should error out of remove_string
        let val = Value::Map(vec![(
            Value::Text("$id".to_string()),
            Value::Identifier([1u8; 32]),
        )]);

        let result =
            ExtendedDocumentV0::from_untrusted_platform_value(val, contract, platform_version);
        assert!(
            result.is_err(),
            "missing $type should cause from_untrusted_platform_value to fail"
        );
    }

    #[test]
    fn from_untrusted_platform_value_fails_for_unknown_document_type() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);

        // Value with $type that's not in the contract — document_type_for_name fails
        let val = Value::Map(vec![
            (
                Value::Text("$type".to_string()),
                Value::Text("doesNotExist".to_string()),
            ),
            (Value::Text("$id".to_string()), Value::Identifier([1u8; 32])),
            (
                Value::Text("$ownerId".to_string()),
                Value::Identifier([2u8; 32]),
            ),
        ]);

        let result =
            ExtendedDocumentV0::from_untrusted_platform_value(val, contract, platform_version);
        assert!(
            result.is_err(),
            "unknown document type name should cause error"
        );
    }

    // ================================================================
    //  from_trusted_platform_value: fails when document type missing
    // ================================================================

    #[test]
    fn from_trusted_platform_value_fails_for_missing_type_name() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);

        let val = Value::Map(vec![(
            Value::Text("$id".to_string()),
            Value::Identifier([1u8; 32]),
        )]);

        let result =
            ExtendedDocumentV0::from_trusted_platform_value(val, contract, platform_version);
        assert!(result.is_err());
    }

    // ================================================================
    //  set_untrusted: exercises identifier-path, binary-path, and other branches
    // ================================================================

    #[test]
    fn set_untrusted_non_identifier_non_binary_path_sets_raw_value() {
        let platform_version = PlatformVersion::latest();
        let (mut ext_doc, _) = make_extended_document(platform_version);

        // "displayName" is neither an identifier path nor a binary path
        ext_doc
            .set_untrusted("displayName", Value::Text("Alice".to_string()))
            .expect("set_untrusted should succeed for plain string field");

        assert_eq!(
            ext_doc.get_optional_value("displayName"),
            Some(&Value::Text("Alice".to_string()))
        );
    }

    // ================================================================
    //  hash: differs when documents differ
    // ================================================================

    #[test]
    fn hash_differs_for_different_documents() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected profile document type");

        let doc_a = document_type
            .random_document(Some(1), platform_version)
            .expect("random doc a");
        let doc_b = document_type
            .random_document(Some(2), platform_version)
            .expect("random doc b");

        let ext_a = ExtendedDocumentV0::from_document_with_additional_info(
            doc_a,
            contract.clone(),
            "profile".to_string(),
            None,
        );
        let ext_b = ExtendedDocumentV0::from_document_with_additional_info(
            doc_b,
            contract,
            "profile".to_string(),
            None,
        );

        let hash_a = ext_a.hash(platform_version).expect("hash a");
        let hash_b = ext_b.hash(platform_version).expect("hash b");
        assert_ne!(hash_a, hash_b, "hashes of different documents must differ");
    }

    // ================================================================
    //  serialize_specific_version_to_bytes
    // ================================================================

    #[test]
    fn serialize_specific_version_to_bytes_v0_succeeds() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let bytes = ext_doc
            .serialize_specific_version_to_bytes(0, platform_version)
            .expect("v0 serialization should succeed");
        assert!(!bytes.is_empty());
        // First byte should be the varint for 0
        assert_eq!(bytes[0], 0);
    }

    #[test]
    fn serialize_specific_version_to_bytes_rejects_unknown_version() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);

        let result = ext_doc.serialize_specific_version_to_bytes(99, platform_version);
        assert!(
            matches!(
                result,
                Err(crate::ProtocolError::UnknownVersionMismatch { received: 99, .. })
            ),
            "unknown feature version should yield UnknownVersionMismatch, got {:?}",
            result
        );
    }

    // ================================================================
    //  from_bytes deserialization error paths
    // ================================================================

    #[test]
    fn from_bytes_empty_buffer_fails() {
        let platform_version = PlatformVersion::latest();
        let result = ExtendedDocumentV0::from_bytes(&[], platform_version);
        assert!(
            result.is_err(),
            "empty buffer should fail ExtendedDocumentV0::from_bytes"
        );
    }

    #[test]
    fn from_bytes_unknown_version_fails() {
        let platform_version = PlatformVersion::latest();
        use integer_encoding::VarInt;
        // Varint 42 followed by junk
        let mut buf = 42u64.encode_var_vec();
        buf.extend_from_slice(&[0u8; 32]);

        let result = ExtendedDocumentV0::from_bytes(&buf, platform_version);
        assert!(
            matches!(
                result,
                Err(crate::ProtocolError::UnknownVersionMismatch { received: 42, .. })
            ),
            "unknown serialized version should yield UnknownVersionMismatch, got {:?}",
            result
        );
    }

    #[test]
    fn from_bytes_v0_truncated_contract_fails() {
        let platform_version = PlatformVersion::latest();
        use integer_encoding::VarInt;
        // Valid version prefix but no contract data at all — the contract
        // deserialization should fail.
        let buf = 0u64.encode_var_vec();

        let result = ExtendedDocumentV0::from_bytes(&buf, platform_version);
        assert!(
            result.is_err(),
            "truncated buffer (no contract) must fail deserialization"
        );
    }

    // ================================================================
    //  to_map_value: token_payment_info is serialized when Some
    // ================================================================

    #[test]
    fn to_map_value_omits_token_payment_info_when_none() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document(platform_version);
        assert!(ext_doc.token_payment_info.is_none());
        let map = ext_doc.to_map_value().expect("to_map_value should succeed");
        assert!(
            !map.contains_key(property_names::TOKEN_PAYMENT_INFO),
            "token_payment_info should be absent when None"
        );
    }

    // ================================================================
    //  token_payment_info: Some branches for to_map_value / into_map_value.
    //
    //  These exercise the `if let Some(token_payment_info) = ...` arms
    //  that prior tests never hit (since they all used token_payment_info =
    //  None).
    // ================================================================

    fn make_extended_document_with_token_payment_info(
        platform_version: &PlatformVersion,
    ) -> (ExtendedDocumentV0, crate::prelude::DataContract) {
        use crate::tokens::token_payment_info::v0::TokenPaymentInfoV0;
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("profile")
            .expect("expected profile document type");
        let document = document_type
            .random_document(Some(11), platform_version)
            .expect("expected random document");
        let tpi = TokenPaymentInfo::V0(TokenPaymentInfoV0::default());
        let ext_doc = ExtendedDocumentV0 {
            document_type_name: "profile".to_string(),
            data_contract_id: contract.id(),
            document,
            data_contract: contract.clone(),
            metadata: None,
            entropy: Default::default(),
            token_payment_info: Some(tpi),
        };
        (ext_doc, contract)
    }

    #[test]
    fn to_map_value_contains_token_payment_info_when_some() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document_with_token_payment_info(platform_version);
        let map = ext_doc.to_map_value().expect("to_map_value ok");
        assert!(
            map.contains_key(property_names::TOKEN_PAYMENT_INFO),
            "token_payment_info should be present in map when Some"
        );
    }

    #[test]
    fn into_map_value_contains_token_payment_info_when_some() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, _) = make_extended_document_with_token_payment_info(platform_version);
        let map = ext_doc.into_map_value().expect("into_map_value ok");
        assert!(
            map.contains_key(property_names::TOKEN_PAYMENT_INFO),
            "token_payment_info should be present in map when Some"
        );
    }

    // ================================================================
    //  set_untrusted: binary-path arm (exercises ReplacementType::BinaryBytes).
    //
    //  The contactInfo document type in dashpay has `encToUserId` as a
    //  byteArray property — which makes it a binary path. Hitting that
    //  branch exercises the second arm of set_untrusted's if-else chain
    //  (binary_paths.contains(path) → BinaryBytes replacement).
    // ================================================================

    #[test]
    fn set_untrusted_binary_path_replaces_bytes() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("contactInfo")
            .expect("contactInfo type");
        let document = document_type
            .random_document(Some(3), platform_version)
            .expect("random contactInfo");
        let mut ext_doc = ExtendedDocumentV0::from_document_with_additional_info(
            document,
            contract,
            "contactInfo".to_string(),
            None,
        );

        // "encToUserId" IS a binary_path — the set_untrusted impl should
        // invoke ReplacementType::BinaryBytes.replace_for_bytes.
        let payload = vec![0xABu8; 32];
        ext_doc
            .set_untrusted("encToUserId", Value::Bytes(payload.clone()))
            .expect("set_untrusted on a binary path should succeed");
        // The stored value should be a Bytes variant — BinaryBytes replaces
        // it into Bytes(bytes).
        let val = ext_doc
            .get_optional_value("encToUserId")
            .expect("value should be stored");
        // The replacement normalizes to Value::Bytes, not Value::Array(...).
        match val {
            Value::Bytes(b) => assert_eq!(b, &payload),
            other => panic!("expected Value::Bytes after set_untrusted, got {:?}", other),
        }
    }

    // ================================================================
    //  Hash differs when document_type_name is changed on an otherwise
    //  identical ExtendedDocument — the hash incorporates the contract
    //  serialization AND the type name length prefix.
    // ================================================================

    #[test]
    fn hash_fails_when_document_type_name_unknown() {
        // Exercises the error path of hash(): serialize_v0 calls
        // document_type()? which fails when the name isn't in the contract.
        let platform_version = PlatformVersion::latest();
        let (mut ext_doc, _) = make_extended_document(platform_version);
        ext_doc.document_type_name = "doesNotExist".to_string();
        let result = ext_doc.hash(platform_version);
        assert!(
            result.is_err(),
            "hash must fail when document_type_name is not on the contract"
        );
    }

    // ================================================================
    //  from_trusted_platform_value honors a $dataContractId override in
    //  the input value (exercises the `remove_optional_hash256_bytes`
    //  branch that overrides the contract's default id).
    // ================================================================

    #[test]
    fn from_trusted_platform_value_honors_data_contract_id_override() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, contract) = make_extended_document(platform_version);

        let mut map = ext_doc.to_map_value().expect("to_map_value ok");
        // Override the $dataContractId with a DIFFERENT id.
        let override_id = [0xEEu8; 32];
        map.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Bytes(override_id.to_vec()),
        );
        let val: Value = map.into();

        let recovered =
            ExtendedDocumentV0::from_trusted_platform_value(val, contract, platform_version)
                .expect("from_trusted_platform_value should succeed");

        assert_eq!(
            recovered.data_contract_id,
            Identifier::from(override_id),
            "override $dataContractId should be used"
        );
    }

    // ================================================================
    //  from_trusted_platform_value falls back to contract id when
    //  $dataContractId is absent from the value.
    // ================================================================

    #[test]
    fn from_trusted_platform_value_falls_back_to_contract_id() {
        let platform_version = PlatformVersion::latest();
        let (ext_doc, contract) = make_extended_document(platform_version);
        let expected_id = contract.id();

        let mut map = ext_doc.to_map_value().expect("to_map_value ok");
        // Remove the $dataContractId so the unwrap_or branch kicks in.
        map.remove(property_names::DATA_CONTRACT_ID);
        let val: Value = map.into();

        let recovered =
            ExtendedDocumentV0::from_trusted_platform_value(val, contract, platform_version)
                .expect("from_trusted_platform_value ok");
        assert_eq!(recovered.data_contract_id, expected_id);
    }

    // ================================================================
    //  from_trusted_platform_value fails when the Value is not a map
    //  (exercises the `into_btree_string_map` error branch).
    // ================================================================

    #[test]
    fn from_trusted_platform_value_rejects_non_map_value() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        // Passing a scalar Value should fail the initial `into_btree_string_map`.
        let result = ExtendedDocumentV0::from_trusted_platform_value(
            Value::Text("not a map".to_string()),
            contract,
            platform_version,
        );
        assert!(result.is_err());
    }

    #[test]
    fn from_untrusted_platform_value_rejects_non_map_value() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let result = ExtendedDocumentV0::from_untrusted_platform_value(
            Value::U64(42),
            contract,
            platform_version,
        );
        assert!(result.is_err());
    }

    // ================================================================
    //  from_document_with_additional_info preserves the token_payment_info
    //  argument — exercises the `Some(...)` code path for that constructor.
    // ================================================================

    #[test]
    fn from_document_with_additional_info_stores_token_payment_info() {
        use crate::tokens::token_payment_info::v0::TokenPaymentInfoV0;
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("profile")
            .expect("profile type");
        let document = document_type
            .random_document(Some(123), platform_version)
            .expect("random");
        let tpi = TokenPaymentInfo::V0(TokenPaymentInfoV0::default());

        let ext_doc = ExtendedDocumentV0::from_document_with_additional_info(
            document,
            contract,
            "profile".to_string(),
            Some(tpi),
        );
        assert!(ext_doc.token_payment_info.is_some());
    }

    // ================================================================
    //  properties / properties_as_mut delegate to the underlying Document.
    //  Mutation via properties_as_mut must be visible through properties().
    // ================================================================

    #[test]
    fn properties_as_mut_and_properties_share_state() {
        let platform_version = PlatformVersion::latest();
        let (mut ext_doc, _) = make_extended_document(platform_version);
        ext_doc
            .properties_as_mut()
            .insert("bulkField".to_string(), Value::U64(777));
        // properties() returns the same map.
        assert_eq!(
            ext_doc.properties().get("bulkField"),
            Some(&Value::U64(777))
        );
    }

    // ================================================================
    //  document_type() fails when document_type_name points to an unknown
    //  type (exercises the error branch in document_type()).
    //  This complements `document_type_fails_for_invalid_type_name` but
    //  goes through `can_be_modified()` and `requires_revision()` which
    //  also depend on document_type().
    // ================================================================

    #[test]
    fn can_be_modified_fails_when_type_name_unknown() {
        let platform_version = PlatformVersion::latest();
        let (mut ext_doc, _) = make_extended_document(platform_version);
        ext_doc.document_type_name = "doesNotExist".to_string();
        assert!(ext_doc.can_be_modified().is_err());
        assert!(ext_doc.requires_revision().is_err());
    }

    // ================================================================
    //  set_untrusted error path: when the target is an identifier path
    //  but the value cannot be converted to identifier bytes.
    //
    //  We can approximate this by attempting set_untrusted on a binary
    //  path with a Value that is NOT byte-like. `to_identifier_bytes` will
    //  fail, which yields an Err from set_untrusted. This exercises the
    //  error branch.
    // ================================================================

    #[test]
    fn set_untrusted_binary_path_returns_err_for_non_byte_value() {
        let platform_version = PlatformVersion::latest();
        let contract = load_dashpay_contract(platform_version);
        let document_type = contract
            .document_type_for_name("contactInfo")
            .expect("contactInfo type");
        let document = document_type
            .random_document(Some(88), platform_version)
            .expect("random contactInfo");
        let mut ext_doc = ExtendedDocumentV0::from_document_with_additional_info(
            document,
            contract,
            "contactInfo".to_string(),
            None,
        );

        // Providing a Text value for a binary path should fail the
        // to_identifier_bytes conversion.
        let result = ext_doc.set_untrusted("encToUserId", Value::Text("not bytes".to_string()));
        assert!(
            result.is_err(),
            "set_untrusted on binary path with non-byte value must fail, got {:?}",
            result
        );
    }
}
