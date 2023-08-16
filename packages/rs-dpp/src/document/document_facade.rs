use crate::document::document_factory::DocumentFactory;

pub struct DocumentFacade {
    pub factory: DocumentFactory,
}

//
// impl<SR> DocumentFacade<SR>
// where
//     SR: StateRepositoryLike,
// {
//     pub fn new(
//         state_repository: Arc<SR>,
//         protocol_version_validator: Arc<ProtocolVersionValidator>,
//     ) -> Self {
//         let protocol_version = protocol_version_validator.protocol_version();
//         let document_validator = DocumentValidator::new(protocol_version_validator);
//         let data_contract_fetcher_and_validator =
//             DataContractFetcherAndValidator::new(state_repository);
//
//         let document_factory = DocumentFactory::new(
//             protocol_version,
//             document_validator.clone(),
//             data_contract_fetcher_and_validator.clone(),
//         );
//
//         Self {
//             data_contract_fetcher_and_validator,
//             factory: document_factory,
//             validator: document_validator,
//         }
//     }
//
//     pub fn create(
//         &self,
//         data_contract: DataContract,
//         owner_id: Identifier,
//         document_type_name: String,
//         data: Value,
//     ) -> Result<ExtendedDocument, ProtocolError> {
//         self.factory.create_extended_document_for_state_transition(
//             data_contract,
//             owner_id,
//             document_type_name,
//             data,
//         )
//     }
//
//     /// Creates Document from object
//     pub async fn create_from_object(
//         &self,
//         raw_document: Value,
//         options: FactoryOptions,
//     ) -> Result<ExtendedDocument, ProtocolError> {
//         self.factory.create_from_object(raw_document, options).await
//     }
//
//     /// Creates Document form bytes
//     pub async fn create_from_buffer(
//         &self,
//         bytes: impl AsRef<[u8]>,
//         options: FactoryOptions,
//     ) -> Result<ExtendedDocument, ProtocolError> {
//         self.factory.create_from_buffer(bytes, options).await
//     }
//
//     pub async fn create_extended_from_document_buffer(
//         &self,
//         buffer: &[u8],
//         document_type: &str,
//         data_contract: &DataContract,
//     ) -> Result<ExtendedDocument, ProtocolError> {
//         self.factory
//             .create_extended_from_document_buffer(buffer, document_type, data_contract)
//     }
//
//     /// Creates Documents State Transition
//     pub fn create_state_transition(
//         &self,
//         documents: impl IntoIterator<Item = (Action, Vec<ExtendedDocument>)>,
//     ) -> Result<DocumentsBatchTransition, ProtocolError> {
//         self.factory.create_state_transition(documents)
//     }
//
//     /// Creates Documents State Transition
//     pub async fn validate_extended_document(
//         &self,
//         extended_document: &ExtendedDocument,
//     ) -> Result<ConsensusValidationResult<DataContract>, ProtocolError> {
//         let raw_extended_document = extended_document.to_value()?;
//         self.validate_raw_extended_document(&raw_extended_document)
//             .await
//     }
//
//     /// Creates Documents State Transition
//     pub async fn validate_raw_extended_document(
//         &self,
//         raw_extended_document: &Value,
//     ) -> Result<ConsensusValidationResult<DataContract>, ProtocolError> {
//         let mut result = self
//             .data_contract_fetcher_and_validator
//             .validate_extended(raw_extended_document)
//             .await?;
//
//         if !result.is_valid() {
//             return Ok(result);
//         }
//
//         let data_contract = result.data_as_borrowed()?;
//         let validation_result = self
//             .validator
//             .validate_extended(raw_extended_document, data_contract)?;
//
//         result.add_errors(validation_result.errors);
//
//         Ok(result)
//     }
// }
