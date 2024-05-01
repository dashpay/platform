mod v0;

use crate::data_contract::DataContract;
use std::collections::BTreeMap;

use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;
use platform_value::{Bytes32, Identifier, Value};

use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::Document;
#[cfg(feature = "extended-document")]
use crate::document::ExtendedDocument;
#[cfg(feature = "state-transitions")]
use crate::state_transition::documents_batch_transition::{
    document_transition::action_type::DocumentTransitionActionType, DocumentsBatchTransition,
};
use crate::util::entropy_generator::EntropyGenerator;
pub use v0::DocumentFactoryV0;

/// # Document Factory
///
/// This module is responsible for creating instances of documents for a specific contract.
///
/// ## Versioning
///
/// The factory is versioned because the process of creating documents
/// can change over time. Changes may be due to modifications in
/// requirements, alterations in the document structure, or evolution in the
/// dependencies of the document. Versioning allows for these changes to be
/// tracked and managed effectively, providing flexibility to handle different
/// versions of documents as needed.
#[derive(From)]
pub enum DocumentFactory {
    /// The version 0 implementation of the data contract factory.
    V0(DocumentFactoryV0),
}

impl DocumentFactory {
    /// Create a new document factory knowing versions
    pub fn new(protocol_version: u32) -> Result<Self, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .platform_architecture
            .document_factory_structure_version
        {
            0 => Ok(DocumentFactoryV0::new(protocol_version).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentFactory::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Result<Self, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .platform_architecture
            .document_factory_structure_version
        {
            0 => Ok(DocumentFactoryV0::new_with_entropy_generator(
                protocol_version,
                entropy_generator,
            )
            .into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentFactory::new_with_entropy_generator".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn create_document(
        &self,
        data_contract: &DataContract,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentFactory::V0(v0) => v0.create_document_without_time_based_properties(
                data_contract,
                owner_id,
                document_type_name,
                data,
            ),
        }
    }

    #[cfg(feature = "extended-document")]
    pub fn create_extended_document(
        &self,
        data_contract: &DataContract,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<ExtendedDocument, ProtocolError> {
        match self {
            DocumentFactory::V0(v0) => {
                v0.create_extended_document(data_contract, owner_id, document_type_name, data)
            }
        }
    }

    #[cfg(feature = "state-transitions")]
    pub fn create_state_transition<'a>(
        &self,
        documents_iter: impl IntoIterator<
            Item = (
                DocumentTransitionActionType,
                Vec<(Document, DocumentTypeRef<'a>, Bytes32)>,
            ),
        >,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
    ) -> Result<DocumentsBatchTransition, ProtocolError> {
        match self {
            DocumentFactory::V0(v0) => v0.create_state_transition(documents_iter, nonce_counter),
        }
    }

    #[cfg(feature = "extended-document")]
    pub fn create_extended_from_document_buffer(
        &self,
        buffer: &[u8],
        document_type_name: &str,
        data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<ExtendedDocument, ProtocolError> {
        match self {
            DocumentFactory::V0(v0) => v0.create_extended_from_document_buffer(
                buffer,
                document_type_name,
                data_contract,
                platform_version,
            ),
        }
    }
}

//
// #[cfg(test)]
// mod test {
//     use platform_value::btreemap_extensions::BTreeValueMapHelper;
//     use platform_value::platform_value;
//     use platform_value::string_encoding::Encoding;
//     use std::sync::Arc;
//
//     use crate::tests::fixtures::get_extended_documents_fixture;
//     use crate::{
//         assert_error_contains,
//         state_repository::MockStateRepositoryLike,
//         tests::{
//             fixtures::{get_data_contract_fixture, get_document_validator_fixture},
//             utils::generate_random_identifier_struct,
//         },
//     };
//     use crate::document::document_factory::DocumentFactory;
//
//     use super::*;
//
//     #[test]
//     fn document_with_type_and_data() {
//         let mut data_contract = get_data_contract_fixture(None).data_contract;
//         let document_type = "niceDocument";
//
//         let factory = DocumentFactory::new(
//             1,
//             get_document_validator_fixture(),
//             DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
//         );
//         let name = "Cutie";
//         let contract_id = Identifier::from_string(
//             "FQco85WbwNgb5ix8QQAH6wurMcgEC5ENSCv5ixG9cj12",
//             Encoding::Base58,
//         )
//             .unwrap();
//         let owner_id = Identifier::from_string(
//             "5zcXZpTLWFwZjKjq3ME5KVavtZa9YUaZESVzrndehBhq",
//             Encoding::Base58,
//         )
//             .unwrap();
//
//         data_contract.id = contract_id;
//
//         let document = factory
//             .create_extended_document_for_state_transition(
//                 data_contract,
//                 owner_id,
//                 document_type.to_string(),
//                 platform_value!({ "name": name }),
//             )
//             .expect("document creation shouldn't fail");
//         assert_eq!(document_type, document.document_type_name);
//         assert_eq!(
//             name,
//             document
//                 .properties()
//                 .get_str("name")
//                 .expect("property 'name' should exist")
//         );
//         assert_eq!(contract_id, document.data_contract_id);
//         assert_eq!(owner_id, document.owner_id());
//         assert_eq!(
//             document_transition::INITIAL_REVISION,
//             *document.revision().unwrap()
//         );
//         assert!(!document.id().to_string(Encoding::Base58).is_empty());
//         assert!(document.created_at().is_some());
//     }
//
//     #[test]
//     fn create_state_transition_no_documents() {
//         let factory = DocumentFactory::new(
//             1,
//             get_document_validator_fixture(),
//             DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
//         );
//
//         let result = factory.create_state_transition(vec![]);
//         assert_error_contains!(result, "No documents were supplied to state transition")
//     }
//
//     #[test]
//     fn create_transition_mismatch_user_id() {
//         let data_contract = get_data_contract_fixture(None).data_contract;
//         let mut documents = get_extended_documents_fixture(data_contract).unwrap();
//
//         let factory = DocumentFactory::new(
//             1,
//             get_document_validator_fixture(),
//             DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
//         );
//
//         documents[0].document.owner_id = generate_random_identifier_struct();
//
//         let result = factory.create_state_transition(vec![(DocumentTransitionActionType::Create, documents)]);
//         assert_error_contains!(result, "Documents have mixed owner ids")
//     }
//
//     #[test]
//     fn create_transition_invalid_initial_revision() {
//         let data_contract = get_data_contract_fixture(None).data_contract;
//         let mut documents = get_extended_documents_fixture(data_contract).unwrap();
//         documents[0].document.revision = Some(3);
//
//         let factory = DocumentFactory::new(
//             1,
//             get_document_validator_fixture(),
//             DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
//         );
//         let result = factory.create_state_transition(vec![(DocumentTransitionActionType::Create, documents)]);
//         assert_error_contains!(result, "Invalid Document initial revision '3'")
//     }
//
//     #[test]
//     fn create_transitions_with_passed_documents() {
//         let data_contract = get_data_contract_fixture(None).data_contract;
//         let documents = get_extended_documents_fixture(data_contract).unwrap();
//         let factory = DocumentFactory::new(
//             1,
//             get_document_validator_fixture(),
//             DataContractFetcherAndValidator::new(Arc::new(MockStateRepositoryLike::new())),
//         );
//
//         let new_document = documents[0].clone();
//         let batch_transition = factory
//             .create_state_transition(vec![
//                 (DocumentTransitionActionType::Create, documents),
//                 (DocumentTransitionActionType::Replace, vec![new_document]),
//             ])
//             .expect("state transitions should be created");
//         assert_eq!(11, batch_transition.transitions.len());
//         assert_eq!(
//             10,
//             batch_transition
//                 .transitions
//                 .iter()
//                 .filter(|t| t.as_transition_create().is_some())
//                 .count()
//         );
//         assert_eq!(
//             1,
//             batch_transition
//                 .transitions
//                 .iter()
//                 .filter(|t| t.as_transition_replace().is_some())
//                 .count()
//         )
//     }
// }
