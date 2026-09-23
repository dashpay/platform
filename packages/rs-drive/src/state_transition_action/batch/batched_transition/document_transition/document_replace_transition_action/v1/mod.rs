//! Generation 1 of the replace-action → `Document` conversion: generation 0
//! plus the contract-version stamp, without the transient values. A replace
//! re-supplies the full document contents, so the document is re-stamped with
//! the current contract version. This generation must only be selected by
//! platform versions whose document serialization format writes the stamp
//! (format 3, protocol v14+); the version table pairs the two.
//!
//! A transient property is judged on the transition and never stored: a
//! create drops its value before the document is built. Generation 0 stored
//! whatever a replace carried, so a replaced document kept the values its
//! create had dropped. Generation 1 drops them the same way, by top-level
//! name. Edited in place: protocol version 14, the only one selecting this
//! generation, is unreleased.

use std::collections::BTreeSet;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use super::{DocumentFromReplaceTransitionActionV0, DocumentReplaceTransitionActionV0};

/// document from replace transition v1
pub trait DocumentFromReplaceTransitionActionV1 {
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction`
    /// reference and `owner_id`, re-stamped with the current contract version and
    /// without the document type's transient values.
    fn try_from_replace_transition_action_v1(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// Attempts to create a new `Document` from the given `DocumentReplaceTransitionAction`
    /// instance and `owner_id`, re-stamped with the current contract version and
    /// without the document type's transient values.
    fn try_from_owned_replace_transition_action_v1(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl DocumentFromReplaceTransitionActionV1 for Document {
    fn try_from_replace_transition_action_v1(
        value: &DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let contract_version = value.base.data_contract_fetch_info_ref().contract.version();
        let mut document =
            Self::try_from_replace_transition_action_v0(value, owner_id, platform_version)?;
        document.set_contract_version(Some(contract_version));
        drop_transient_values(
            &mut document,
            value.base.document_type()?.transient_fields(),
        );
        Ok(document)
    }

    fn try_from_owned_replace_transition_action_v1(
        value: DocumentReplaceTransitionActionV0,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let contract_version = value.base.data_contract_fetch_info_ref().contract.version();
        let transient_fields = value.base.document_type()?.transient_fields().clone();
        let mut document =
            Self::try_from_owned_replace_transition_action_v0(value, owner_id, platform_version)?;
        document.set_contract_version(Some(contract_version));
        drop_transient_values(&mut document, &transient_fields);
        Ok(document)
    }
}

/// Drops the values of `transient_fields` from `document`, as the create action
/// drops them from its data.
fn drop_transient_values(document: &mut Document, transient_fields: &BTreeSet<String>) {
    if !transient_fields.is_empty() {
        document
            .properties_mut()
            .retain(|key, _| !transient_fields.contains(key));
    }
}
