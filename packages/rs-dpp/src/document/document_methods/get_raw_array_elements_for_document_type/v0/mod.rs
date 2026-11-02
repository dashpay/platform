use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::DocumentV0Getters;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::Value;
use std::collections::BTreeSet;

pub trait DocumentGetRawArrayElementsForDocumentTypeV0: DocumentV0Getters {
    /// The index keys of the elements of the typed array at `key_path`, each encoded as a
    /// property of the element's type is, sorted and without repeats: an index naming the
    /// array holds one entry per key. A list holding an element twice, which a type without
    /// `uniqueItems` allows, is keyed once, and two elements that encode alike are one key.
    /// An absent, null or empty list has no keys.
    fn get_raw_array_elements_for_document_type_v0(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, ProtocolError> {
        let elements = match self.properties().get_optional_at_path(key_path)? {
            None | Some(Value::Null) => return Ok(Vec::new()),
            Some(Value::Array(elements)) => elements,
            Some(_) => {
                return Err(ProtocolError::CorruptedCodeExecution(format!(
                    "the indexed typed array {key_path} of document {} does not hold a list",
                    self.id()
                )))
            }
        };
        let keys = elements
            .iter()
            .map(|element| {
                document_type.serialize_value_for_key(key_path, element, platform_version)
            })
            .collect::<Result<BTreeSet<Vec<u8>>, ProtocolError>>()?;
        Ok(keys.into_iter().collect())
    }
}
