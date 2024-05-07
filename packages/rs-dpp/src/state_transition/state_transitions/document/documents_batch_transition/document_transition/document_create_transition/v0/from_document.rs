use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::{Document, DocumentV0Getters};
use crate::prelude::IdentityNonce;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::ProtocolError;
use platform_version::version::{FeatureVersion, PlatformVersion};

impl DocumentCreateTransitionV0 {
    pub(crate) fn from_document(
        document: Document,
        document_type: DocumentTypeRef,
        entropy: [u8; 32],
        identity_contract_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
        base_feature_version: Option<FeatureVersion>,
    ) -> Result<Self, ProtocolError> {
        let prefunded_voting_balances = document_type
            .indexes()
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
            .collect();
        Ok(DocumentCreateTransitionV0 {
            base: DocumentBaseTransition::from_document(
                &document,
                document_type,
                identity_contract_nonce,
                platform_version,
                base_feature_version,
            )?,
            entropy,
            data: document.properties_consumed(),
            prefunded_voting_balances,
        })
    }
}
