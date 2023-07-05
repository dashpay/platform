use std::collections::{BTreeMap, BTreeSet};
use crate::data_contract::document_type::document_field::DocumentField;
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use crate::version::dpp_versions::DocumentTypeVersions;

mod v0;

impl DocumentType {
    pub fn find_identifier_and_binary_paths(
        properties: &BTreeMap<String, DocumentField>,
        document_type_version: &DocumentTypeVersions
    ) -> Result<(BTreeSet<String>, BTreeSet<String>), ProtocolError> {
        match document_type_version.find_identifier_and_binary_paths {
            0 => Ok(Self::find_identifier_and_binary_paths_v0(properties)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "find_identifier_and_binary_paths".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}