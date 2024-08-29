use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::{Document, DocumentV0Getters};
use crate::fee::Credits;
use crate::version::PlatformVersion;

impl DocumentTypeV0 {
    /// Figures out the prefunded voting balance (v0) for a document in a document type
    pub(in crate::data_contract::document_type) fn prefunded_voting_balance_for_document_v0(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> Option<(String, Credits)> {
        self.indexes()
            .values()
            .find(|index| {
                if let Some(contested_index_info) = &index.contested_index {
                    contested_index_info
                        .field_matches
                        .iter()
                        .all(|(field, field_match)| {
                            if let Some(value) = document.get(field) {
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
                (
                    index.name.clone(),
                    platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_vote_resolution_fund_required_amount,
                )
            })
    }
}
