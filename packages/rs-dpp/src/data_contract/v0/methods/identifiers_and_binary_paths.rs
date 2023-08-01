use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::base::DataContractBaseMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
use crate::data_contract::JsonValue;
use crate::{identifier, ProtocolError};
use itertools::{Either, Itertools};
use std::collections::{BTreeMap, HashSet};

impl DataContractIdentifiersAndBinaryPathsMethodsV0 for DataContractV0 {
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
