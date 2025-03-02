use crate::data_contract::document_type::property::DocumentProperty;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use indexmap::IndexMap;
use platform_version::version::dpp_versions::dpp_contract_versions::DocumentTypeVersions;
use std::collections::BTreeSet;

mod v0;

impl DocumentType {
    pub(in crate::data_contract) fn find_identifier_and_binary_paths(
        properties: &IndexMap<String, DocumentProperty>,
        document_type_version: &DocumentTypeVersions,
    ) -> Result<(BTreeSet<String>, BTreeSet<String>), ProtocolError> {
        match document_type_version
            .schema
            .find_identifier_and_binary_paths
        {
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
