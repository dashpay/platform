mod enrich_with_base_schema;

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::data_contract_methods::base::DataContractBaseMethodsV0;
use crate::data_contract::data_contract_methods::document_schema::DataContractDocumentSchemaMethodsV0;
use crate::data_contract::data_contract_methods::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::{DocumentTypeMutRef, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DataContract, DocumentName, JsonSchema, PropertyPath};
use crate::version::PlatformVersion;
use crate::{identifier, ProtocolError};
use itertools::{Either, Itertools};
use platform_value::string_encoding::Encoding;
use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, HashSet};

impl DataContractBaseMethodsV0 for DataContractV0 {
    /// Increments version of Data Contract
    fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Returns true if document type is defined
    fn is_document_defined(&self, document_type_name: &str) -> bool {
        self.document_types.get(document_type_name).is_some()
    }

    fn optional_document_type_for_name(&self, document_type_name: &str) -> Option<DocumentTypeRef> {
        self.document_types
            .get(document_type_name)
            .map(|document_type| document_type.as_ref())
    }

    fn optional_document_type_mut_for_name(
        &mut self,
        document_type_name: &str,
    ) -> Option<DocumentTypeMutRef> {
        self.document_types
            .get_mut(document_type_name)
            .map(|document_type| document_type.as_mut_ref())
    }

    fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<DocumentTypeRef, ProtocolError> {
        Ok(self
            .document_types
            .get(document_type_name)
            .ok_or({
                ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                    "can not get document type from contract",
                ))
            })?
            .as_ref())
    }

    fn has_document_type_for_name(&self, document_type_name: &str) -> bool {
        self.document_types.get(document_type_name).is_some()
    }
}

impl DataContractIdentifiersAndBinaryPathsMethodsV0 for DataContractV0 {
    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContractV0::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ProtocolError> {
        self.get_optional_binary_properties(doc_type)?
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ))
    }

    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContractV0::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    // TODO: Naming is confusing. It's not clear, it sounds like it will return optional document properties
    //   but not None if document type is not present. Rename this
    fn get_optional_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<Option<&BTreeMap<String, JsonValue>>, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Ok(None);
        }

        // The rust implementation doesn't set the value if it is not present in `binary_properties`. The difference is caused by
        // required `mut` annotation. As `get_binary_properties` is reused in many other read-only methods, the mutation would require
        // propagating the `mut` to other getters which by the definition shouldn't be mutable.
        self.binary_properties
            .get(doc_type)
            .ok_or_else(|| {
                {
                    anyhow::anyhow!(
                        "document '{}' has not generated binary_properties",
                        doc_type
                    )
                }
                .into()
            })
            .map(Some)
    }

    fn generate_binary_properties(
        &mut self,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        self.binary_properties = self
            .documents
            .iter()
            .map(|(doc_type, schema)| {
                Ok((
                    String::from(doc_type),
                    DataContract::get_binary_properties(schema, platform_version)?,
                ))
            })
            .collect::<Result<BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError>>()?;
        Ok(())
    }

    fn get_identifiers_and_binary_paths(
        &self,
        document_type: &str,
    ) -> Result<(HashSet<&str>, HashSet<&str>), ProtocolError> {
        let binary_properties = self.get_optional_binary_properties(document_type)?;

        // At this point we don't bother about returned error from `get_binary_properties`.
        // If document of given type isn't found, then empty vectors will be returned.
        let (binary_paths, identifiers_paths) = match binary_properties {
            None => (HashSet::new(), HashSet::new()),
            Some(binary_properties) => binary_properties.iter().partition_map(|(path, v)| {
                if let Some(JsonValue::String(content_type)) = v.get("contentMediaType") {
                    if content_type == identifier::MEDIA_TYPE {
                        Either::Right(path.as_str())
                    } else {
                        Either::Left(path.as_str())
                    }
                } else {
                    Either::Left(path.as_str())
                }
            }),
        };
        Ok((identifiers_paths, binary_paths))
    }

    fn get_identifiers_and_binary_paths_owned<
        I: IntoIterator<Item = String> + Extend<String> + Default,
    >(
        &self,
        document_type: &str,
    ) -> Result<(I, I), ProtocolError> {
        let binary_properties = self.get_optional_binary_properties(document_type)?;

        // At this point we don't bother about returned error from `get_binary_properties`.
        // If document of given type isn't found, then empty vectors will be returned.
        Ok(binary_properties
            .map(|binary_properties| {
                binary_properties.iter().partition_map(|(path, v)| {
                    if let Some(JsonValue::String(content_type)) = v.get("contentMediaType") {
                        if content_type == platform_value::IDENTIFIER_MEDIA_TYPE {
                            Either::Left(path.clone())
                        } else {
                            Either::Right(path.clone())
                        }
                    } else {
                        Either::Right(path.clone())
                    }
                })
            })
            .unwrap_or_default())
    }
}

impl DataContractDocumentSchemaMethodsV0 for DataContractV0 {
    fn set_document_schema(
        &mut self,
        doc_type: String,
        schema: JsonSchema,
        platform_version: &PlatformVersion,
    ) -> Result<(), ProtocolError> {
        let binary_properties = DataContract::get_binary_properties(&schema, platform_version)?;
        self.documents.insert(doc_type.clone(), schema.clone());
        self.binary_properties
            .insert(doc_type.clone(), binary_properties);

        let document_type_value = platform_value::Value::from(schema);

        // Make sure the document_type_value is a map
        let Some(document_type_value_map) = document_type_value.as_map() else {
            return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            )));
        };

        let document_type = DocumentTypeV0::from_platform_value(
            self.id,
            &doc_type,
            document_type_value_map,
            &BTreeMap::new(),
            self.config.documents_keep_history_contract_default(),
            self.config.documents_mutable_contract_default(),
            platform_version,
        )?;

        self.document_types.insert(doc_type, document_type.into());

        Ok(())
    }

    fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        let document = self
            .documents
            .get(doc_type)
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ))?;
        Ok(document)
    }

    fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ));
        };

        Ok(format!(
            "{}#/documents/{}",
            self.id.to_string(Encoding::Base58),
            doc_type
        ))
    }
}
