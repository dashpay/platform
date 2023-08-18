use crate::consensus::basic::document::{InvalidDocumentTypeError, MissingDocumentTypeError};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::DataContract;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
#[cfg(feature = "validation")]
use crate::data_contract::validation::DataContractValidationMethodsV0;
use crate::identity::TimestampMillis;
use crate::prelude::Revision;
use crate::ProtocolError;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use crate::state_transition::documents_batch_transition::fields::property_names;
use crate::validation::SimpleConsensusValidationResult;

pub trait DocumentReplaceTransitionV0Methods {
    /// Returns a reference to the `base` field of the `DocumentReplaceTransitionV0`.
    fn base(&self) -> &DocumentBaseTransition;
    /// Returns a mut reference to the `base` field of the `DocumentReplaceTransitionV0`.
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;

    /// Sets the value of the `base` field in the `DocumentReplaceTransitionV0`.
    fn set_base(&mut self, base: DocumentBaseTransition);

    /// Returns a reference to the `revision` field of the `DocumentReplaceTransitionV0`.
    fn revision(&self) -> Revision;

    /// Sets the value of the `revision` field in the `DocumentReplaceTransitionV0`.
    fn set_revision(&mut self, revision: Revision);

    /// Returns a reference to the `updated_at` field of the `DocumentReplaceTransitionV0`.
    fn updated_at(&self) -> Option<TimestampMillis>;

    /// Sets the value of the `updated_at` field in the `DocumentReplaceTransitionV0`.
    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>);

    /// Returns a reference to the `data` field of the `DocumentReplaceTransitionV0`.
    fn data(&self) -> &BTreeMap<String, Value>;

    /// Returns a mutable reference to the `data` field of the `DocumentReplaceTransitionV0`.
    fn data_mut(&mut self) -> &mut BTreeMap<String, Value>;

    /// Sets the value of the `data` field in the `DocumentReplaceTransitionV0`.
    fn set_data(&mut self, data: BTreeMap<String, Value>);

    #[cfg(feature = "validation")]
    fn validate(
        &self,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentReplaceTransitionV0Methods for DocumentReplaceTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base;
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        self.updated_at
    }

    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        self.updated_at = updated_at;
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        &self.data
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.data
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        self.data = data;
    }

    #[cfg(feature = "validation")]
    fn validate(
        &self,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let document_type_name = self.base().document_type_name();

        // Make sure that the document type is defined in the contract
        let Some(document_type) = data_contract
            .document_type_optional_for_name(document_type_name) else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id()).into(),
            ));
        };

        // Make sure that timestamps are present if required
        let required_fields = document_type.required_fields();

        if required_fields.contains(property_names::UPDATED_AT) && self.updated_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        // Validate user defined properties
        let data = platform_value::to_value(self.data())?;

        data_contract.validate_document_properties(document_type_name, &data, platform_version)
    }
}
