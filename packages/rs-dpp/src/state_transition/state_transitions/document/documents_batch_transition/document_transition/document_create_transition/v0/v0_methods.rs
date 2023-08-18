use crate::consensus::basic::document::{
    InvalidDocumentTransitionIdError, InvalidDocumentTypeError, MissingDocumentTypeError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::validation::DataContractValidationMethodsV0;
use crate::data_contract::DataContract;
use crate::document::{property_names, Document};
use crate::identity::TimestampMillis;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

pub trait DocumentCreateTransitionV0Methods {
    /// Returns a reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base(&self) -> &DocumentBaseTransition;

    /// Returns a mut reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;

    /// Sets the value of the `base` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `base` - A value of type `DocumentBaseTransition` to set.
    fn set_base(&mut self, base: DocumentBaseTransition);

    /// Returns a reference to the `entropy` field of the `DocumentCreateTransitionV0`.
    fn entropy(&self) -> [u8; 32];

    /// Sets the value of the `entropy` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `entropy` - An array of 32 bytes to set.
    fn set_entropy(&mut self, entropy: [u8; 32]);

    /// Returns a reference to the `created_at` field of the `DocumentCreateTransitionV0`.
    fn created_at(&self) -> Option<TimestampMillis>;

    /// Sets the value of the `created_at` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `created_at` - An `Option` containing a `TimestampMillis` value to set.
    fn set_created_at(&mut self, created_at: Option<TimestampMillis>);

    /// Returns a reference to the `updated_at` field of the `DocumentCreateTransitionV0`.
    fn updated_at(&self) -> Option<TimestampMillis>;

    /// Sets the value of the `updated_at` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `updated_at` - An `Option` containing a `TimestampMillis` value to set.
    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>);

    /// Returns an optional reference to the `data` field of the `DocumentCreateTransitionV0`.
    fn data(&self) -> &BTreeMap<String, Value>;

    /// Returns an optional mutable reference to the `data` field of the `DocumentCreateTransitionV0`.
    fn data_mut(&mut self) -> &mut BTreeMap<String, Value>;

    /// Sets the value of the `data` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `data` - An `Option` containing a `BTreeMap<String, Value>` to set.
    fn set_data(&mut self, data: BTreeMap<String, Value>);

    #[cfg(feature = "validation")]
    fn validate(
        &self,
        data_contract: &DataContract,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DocumentCreateTransitionV0Methods for DocumentCreateTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base;
    }

    fn entropy(&self) -> [u8; 32] {
        self.entropy
    }

    fn set_entropy(&mut self, entropy: [u8; 32]) {
        self.entropy = entropy;
    }

    fn created_at(&self) -> Option<TimestampMillis> {
        self.created_at
    }

    fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        self.created_at = created_at;
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
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Make sure that the document type is defined in the contract
        let document_type_name = self.base().document_type_name();

        let Some(document_type) = data_contract
            .document_type_optional_for_name(document_type_name) else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id()).into(),
            ));
        };

        // Validate the ID
        let generated_document_id = Document::generate_document_id_v0(
            data_contract.id_ref(),
            &owner_id,
            document_type_name,
            &self.entropy(),
        );

        let id = self.base().id();
        if generated_document_id != id {
            // dbg!(
            //     "g {} d {} c id {} owner {} dt {} e {}",
            //     hex::encode(generated_document_id),
            //     hex::encode(document_id),
            //     hex::encode(data_contract.id),
            //     hex::encode(owner_id),
            //     document_type,
            //     hex::encode(entropy)
            // );
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionIdError::new(generated_document_id, id).into(),
            ));
        }

        // Make sure that timestamps are present if required
        let required_fields = document_type.required_fields();

        if required_fields.contains(property_names::CREATED_AT) && self.created_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        if required_fields.contains(property_names::UPDATED_AT) && self.updated_at().is_none() {
            // TODO: Create a special consensus error for this
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MissingDocumentTypeError::new().into(),
            ));
        }

        if self.created_at().is_some()
            && self.updated_at().is_some()
            && self.created_at() != self.updated_at()
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTimestampsMismatchError::new(self.base().id()).into(),
            ));
        }

        // Validate user defined properties
        let data = platform_value::to_value(self.data())?;

        data_contract.validate_document_properties(document_type_name, &data, platform_version)
    }
}
