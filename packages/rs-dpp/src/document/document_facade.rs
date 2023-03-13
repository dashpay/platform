use anyhow::anyhow;
use platform_value::Value;
use std::sync::Arc;

use crate::document::ExtendedDocument;
use crate::{
    data_contract::DataContract, prelude::Identifier, state_repository::StateRepositoryLike,
    validation::ValidationResult, version::ProtocolVersionValidator, ProtocolError,
};

use super::{
    document_factory::{DocumentFactory, FactoryOptions},
    document_transition::Action,
    document_validator::DocumentValidator,
    fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    DocumentsBatchTransition,
};

pub struct DocumentFacade<SR> {
    pub data_contract_fetcher_and_validator: DataContractFetcherAndValidator<SR>,
    pub validator: DocumentValidator,
    pub factory: DocumentFactory<SR>,
}

impl<SR> DocumentFacade<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Self {
        let protocol_version = protocol_version_validator.protocol_version();
        let document_validator = DocumentValidator::new(protocol_version_validator);
        let data_contract_fetcher_and_validator =
            DataContractFetcherAndValidator::new(state_repository);

        let document_factory = DocumentFactory::new(
            protocol_version,
            document_validator.clone(),
            data_contract_fetcher_and_validator.clone(),
            None,
        );

        Self {
            data_contract_fetcher_and_validator,
            factory: document_factory,
            validator: document_validator,
        }
    }

    pub fn create(
        &self,
        data_contract: DataContract,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<ExtendedDocument, ProtocolError> {
        self.factory.create_extended_document_for_state_transition(
            data_contract,
            owner_id,
            document_type_name,
            data,
        )
    }

    /// Creates Document from object
    pub async fn create_from_object(
        &self,
        raw_document: Value,
        options: FactoryOptions,
    ) -> Result<ExtendedDocument, ProtocolError> {
        self.factory.create_from_object(raw_document, options).await
    }

    /// Creates Document form bytes
    pub async fn create_from_buffer(
        &self,
        bytes: impl AsRef<[u8]>,
        options: FactoryOptions,
    ) -> Result<ExtendedDocument, ProtocolError> {
        self.factory.create_from_buffer(bytes, options).await
    }

    /// Creates Documents State Transition
    pub fn create_state_transition(
        &self,
        documents: impl IntoIterator<Item = (Action, Vec<ExtendedDocument>)>,
    ) -> Result<DocumentsBatchTransition, ProtocolError> {
        self.factory.create_state_transition(documents)
    }

    /// Creates Documents State Transition
    pub async fn validate_document(
        &self,
        extended_document: &ExtendedDocument,
    ) -> Result<ValidationResult<DataContract>, ProtocolError> {
        let raw_extended_document = extended_document.to_value()?;
        self.validate_raw_document(&raw_extended_document).await
    }

    /// Creates Documents State Transition
    pub async fn validate_raw_document(
        &self,
        raw_extended_document: &Value,
    ) -> Result<ValidationResult<DataContract>, ProtocolError> {
        let result = self
            .data_contract_fetcher_and_validator
            .validate_extended(raw_extended_document)
            .await?;

        if !result.is_valid() {
            return Ok(result);
        }

        let data_contract = result
            .data()
            .ok_or_else(|| anyhow!("Data Contract for document not present"))?;
        let result = self
            .validator
            .validate_extended(raw_extended_document, data_contract)?;

        Ok(ValidationResult::new(Some(result.errors)))
    }
}
