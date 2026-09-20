use crate::consensus::basic::data_contract::{
    DataContractUpdateEntryKind, DataContractUpdateOverlappingEntriesError,
};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;

impl DataContractUpdateTransitionV1 {
    /// Names the first entry this delta lists in two sections that can not
    /// both apply: a document type or schema definition that is both new
    /// and updated, or a keyword that is both added and removed.
    ///
    /// Basic structure validation rejects such a delta with this error and
    /// proof verification refuses to verify one, so one check answers both.
    pub fn overlapping_entry(&self) -> Option<DataContractUpdateOverlappingEntriesError> {
        let contract_id = self.data_contract_id;

        if let Some(name) = self
            .updated_document_schemas
            .keys()
            .find(|name| self.new_document_schemas.contains_key(*name))
        {
            return Some(DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::DocumentType,
                name.clone(),
            ));
        }

        if let Some(name) = self
            .updated_schema_defs
            .keys()
            .find(|name| self.new_schema_defs.contains_key(*name))
        {
            return Some(DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::SchemaDef,
                name.clone(),
            ));
        }

        if let Some(keyword) = self
            .add_keywords
            .iter()
            .find(|keyword| self.remove_keywords.contains(*keyword))
        {
            return Some(DataContractUpdateOverlappingEntriesError::new(
                contract_id,
                DataContractUpdateEntryKind::Keyword,
                keyword.clone(),
            ));
        }

        None
    }
}
