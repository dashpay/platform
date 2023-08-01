mod v0;

use std::collections::HashSet;
use platform_version::version::PlatformVersion;
use crate::data_contract::document_type::{Index, IndexProperty};
use crate::data_contract::document_type::index::validate_indices::get_wrongly_constructed_new_index::v0::get_wrongly_constructed_new_index_v0;
use crate::ProtocolError;

impl Index {
    /// Checks if there exists a wrongly constructed new index.
    ///
    /// A new index is wrongly constructed if:
    /// - It has old properties in them but in a different order than existing indices.
    /// - It is created for an unindexed field that is not a newly added property.
    ///
    /// # Arguments
    ///
    /// * `existing_schema_indices` - An iterator over existing schema indices.
    /// * `new_schema_indices` - An iterator over new schema indices.
    /// * `added_properties` - An iterator over added properties.
    ///
    /// # Returns
    ///
    /// * `Result<Option<&'a Index>, ProtocolError>` - The new index that is wrongly constructed, if any. Otherwise, `None`.
    pub(in crate::data_contract::document_type) fn get_wrongly_constructed_new_index<'a>(
        existing_schema_indices: impl IntoIterator<Item = &'a Index>,
        new_schema_indices: impl IntoIterator<Item = &'a Index>,
        added_properties: impl IntoIterator<Item = &'a str>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<&'a Index>, ProtocolError> {
        match platform_version.dpp.contract_versions.index_versions.validation.get_wrongly_constructed_new_index {
            0 => Index::get_wrongly_constructed_new_index_v0(existing_schema_indices, new_schema_indices, added_properties),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Index::get_wrongly_constructed_new_index".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
