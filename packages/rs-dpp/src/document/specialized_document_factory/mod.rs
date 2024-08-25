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
pub use v0::SpecializedDocumentFactoryV0;

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
pub enum SpecializedDocumentFactory {
    /// The version 0 implementation of the data contract factory.
    V0(SpecializedDocumentFactoryV0),
}

impl SpecializedDocumentFactory {
    /// Create a new document factory knowing versions
    pub fn new(protocol_version: u32, data_contract: DataContract) -> Result<Self, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .platform_architecture
            .document_factory_structure_version
        {
            0 => Ok(SpecializedDocumentFactoryV0::new(protocol_version, data_contract).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DocumentFactory::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        data_contract: DataContract,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Result<Self, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .platform_architecture
            .document_factory_structure_version
        {
            0 => Ok(SpecializedDocumentFactoryV0::new_with_entropy_generator(
                protocol_version,
                data_contract,
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

    pub fn data_contract(&self) -> &DataContract {
        match self {
            SpecializedDocumentFactory::V0(v0) => &v0.data_contract,
        }
    }

    pub fn create_document(
        &self,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<Document, ProtocolError> {
        match self {
            SpecializedDocumentFactory::V0(v0) => {
                v0.create_document_without_time_based_properties(owner_id, document_type_name, data)
            }
        }
    }

    #[cfg(feature = "extended-document")]
    pub fn create_extended_document(
        &self,
        owner_id: Identifier,
        document_type_name: String,
        data: Value,
    ) -> Result<ExtendedDocument, ProtocolError> {
        match self {
            SpecializedDocumentFactory::V0(v0) => {
                v0.create_extended_document(owner_id, document_type_name, data)
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
            SpecializedDocumentFactory::V0(v0) => {
                v0.create_state_transition(documents_iter, nonce_counter)
            }
        }
    }

    #[cfg(feature = "extended-document")]
    pub fn create_extended_from_document_buffer(
        &self,
        buffer: &[u8],
        document_type_name: &str,
        platform_version: &PlatformVersion,
    ) -> Result<ExtendedDocument, ProtocolError> {
        match self {
            SpecializedDocumentFactory::V0(v0) => v0.create_extended_from_document_buffer(
                buffer,
                document_type_name,
                platform_version,
            ),
        }
    }
}
