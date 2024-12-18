use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::{Document, DocumentV0Getters};
use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use crate::voting::vote_polls::VotePoll;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::Value;
use std::collections::BTreeMap;

impl DocumentTypeV0 {
    /// Figures out the prefunded voting balance (v0) for a document in a document type
    pub(in crate::data_contract::document_type) fn contested_vote_poll_for_document_v0(
        &self,
        document: &Document,
    ) -> Option<VotePoll> {
        self.contested_vote_poll_for_document_properties_v0(document.properties())
    }

    pub(in crate::data_contract::document_type) fn contested_vote_poll_for_document_properties_v0(
        &self,
        document_properties: &BTreeMap<String, Value>,
    ) -> Option<VotePoll> {
        self.indexes()
            .values()
            .find(|index| {
                if let Some(contested_index_info) = &index.contested_index {
                    contested_index_info
                        .field_matches
                        .iter()
                        .all(|(field, field_match)| {
                            if let Some(value) = document_properties
                                .get_optional_at_path(field)
                                .ok()
                                .flatten()
                            {
                                field_match.matches(value)
                            } else {
                                false
                            }
                        })
                } else {
                    false
                }
            })
            .map(|index| {
                let index_values = index.extract_values(document_properties);
                VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
                    contract_id: self.data_contract_id,
                    document_type_name: self.name.clone(),
                    index_name: index.name.clone(),
                    index_values,
                })
            })
    }
}
