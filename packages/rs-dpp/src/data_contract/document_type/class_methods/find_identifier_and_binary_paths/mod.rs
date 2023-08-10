use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::version::dpp_versions::DocumentTypeVersions;
use crate::ProtocolError;
use std::collections::{BTreeMap, BTreeSet};

mod v0;

impl DocumentType {
    pub fn find_identifier_and_binary_paths(
        properties: &BTreeMap<String, DocumentProperty>,
        document_type_version: &DocumentTypeVersions,
    ) -> Result<(BTreeSet<String>, BTreeSet<String>), ProtocolError> {
        match document_type_version.find_identifier_and_binary_paths {
            0 => Ok(DocumentTypeV0::find_identifier_and_binary_paths_v0(
                properties,
            )),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "find_identifier_and_binary_paths".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
