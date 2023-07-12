use std::collections::HashSet;
use crate::data_contract::DataContract;
use crate::document::Document;
use crate::ProtocolError;
use crate::version::PlatformVersion;

mod v0;

impl Document {
    pub fn get_identifiers_and_binary_paths<'a>(
        data_contract: &'a DataContract,
        document_type_name: &'a str,
        platform_version: &'a PlatformVersion,
    ) -> Result<(HashSet<&'a str>, HashSet<&'a str>), ProtocolError> {
        match platform_version
            .dpp
            .document_versions
            .document_class_method_versions
            .get_identifiers_and_binary_paths
        {
            0 => Self::get_identifiers_and_binary_paths_v0(data_contract, document_type_name),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::get_identifiers_and_binary_paths".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}