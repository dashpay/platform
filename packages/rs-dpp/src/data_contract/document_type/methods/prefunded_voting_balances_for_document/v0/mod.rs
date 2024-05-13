use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::{Document, DocumentV0Getters};
use crate::fee::Credits;
use crate::version::PlatformVersion;
use std::collections::BTreeMap;

impl DocumentTypeV0 {
    /// Figures out the prefunded voting balance (v0) for a document in a document type
    pub(in crate::data_contract::document_type) fn prefunded_voting_balances_for_document_v0(
        &self,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> BTreeMap<String, Credits> {
        self.indexes()
            .iter()
            .filter_map(|(name, index)| {
                if let Some(contested_index_info) = &index.contested_index {
                    if let Some(value) = document.get(&contested_index_info.contested_field_name) {
                        if contested_index_info.field_match.matches(value) {
                            return Some((
                                name.clone(),
                                platform_version
                                    .fee_version
                                    .vote_resolution_fund_fees
                                    .conflicting_vote_resolution_fund_required_amount,
                            ));
                        }
                    }
                }
                None
            })
            .collect()
    }
}
