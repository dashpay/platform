mod v0;

use std::collections::BTreeMap;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use crate::data_contract::document_type::Index;
use crate::data_contract::JsonSchema;
use crate::ProtocolError;

impl Index {

    /// Checks if there exists a wrongly updated non-unique index.
    ///
    /// A non-unique index is wrongly updated if:
    /// - The properties in the new index differ from the existing properties.
    /// - The rest of the new properties are not defined in the existing schema.
    ///
    /// # Arguments
    ///
    /// * `existing_schema_indices` - A reference to an array of existing schema indices.
    /// * `new_indices` - A reference to a map containing new indices.
    /// * `existing_schema` - A reference to the existing JSON schema.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<&'a Index>, ProtocolError>` - The wrongly updated non-unique index, if any. Otherwise, `None`.
    pub(in crate::data_contract::document_type) fn get_wrongly_updated_non_unique_index<'a>(
        existing_schema_indices: &'a [Index],
        new_indices: &'a BTreeMap<String, Index>,
        existing_schema: &'a Value,
        platform_version: &PlatformVersion,
    ) -> Result<Option<&'a Index>, ProtocolError> {
        match platform_version.dpp.contract_versions.index_versions.validation.get_wrongly_updated_non_unique_index {
            0 => Index::get_wrongly_updated_non_unique_index_v0(existing_schema_indices, new_indices, existing_schema),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Index::get_wrongly_updated_non_unique_index".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}