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
use crate::state_transition::batch_transition::{
    batched_transition::document_transition_action_type::DocumentTransitionActionType,
    BatchTransition,
};
use crate::tokens::token_payment_info::TokenPaymentInfo;
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
            .dpp
            .factory_versions
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
            .dpp
            .factory_versions
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
                Vec<(
                    Document,
                    DocumentTypeRef<'a>,
                    Bytes32,
                    Option<TokenPaymentInfo>,
                )>,
            ),
        >,
        nonce_counter: &mut BTreeMap<(Identifier, Identifier), u64>, //IdentityID/ContractID -> nonce
    ) -> Result<BatchTransition, ProtocolError> {
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

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::document::DocumentV0Getters;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::util::entropy_generator::EntropyGenerator;
    use platform_value::platform_value;
    use platform_version::version::PlatformVersion;

    /// Deterministic entropy generator for tests.
    struct TestEntropyGenerator;

    impl EntropyGenerator for TestEntropyGenerator {
        fn generate(&self) -> anyhow::Result<[u8; 32]> {
            Ok([7u8; 32])
        }
    }

    /// Always-failing entropy generator — used to exercise the error
    /// surface in `DocumentFactory` when the generator itself fails.
    struct FailingEntropyGenerator;

    impl EntropyGenerator for FailingEntropyGenerator {
        fn generate(&self) -> anyhow::Result<[u8; 32]> {
            Err(anyhow::anyhow!("synthetic entropy failure"))
        }
    }

    fn setup_factory() -> (DocumentFactory, DataContract) {
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let factory = DocumentFactory::new_with_entropy_generator(
            platform_version.protocol_version,
            Box::new(TestEntropyGenerator),
        )
        .expect("factory construction should succeed");
        (factory, data_contract)
    }

    // ----- Construction ------------------------------------------------------

    #[test]
    fn new_with_bad_protocol_version_returns_error() {
        // An invalid protocol version should bubble out of the PlatformVersion lookup.
        let result = DocumentFactory::new(u32::MAX);
        assert!(
            matches!(result, Err(ProtocolError::PlatformVersionError(_))),
            "expected PlatformVersionError, got {:?}",
            result.err()
        );
    }

    #[test]
    fn new_with_zero_protocol_version_returns_error() {
        // `PlatformVersion::get(0)` also returns an error.
        let result = DocumentFactory::new(0);
        assert!(result.is_err(), "expected error for version 0");
    }

    #[test]
    fn new_with_entropy_generator_bad_version_returns_error() {
        let result =
            DocumentFactory::new_with_entropy_generator(u32::MAX, Box::new(TestEntropyGenerator));
        assert!(result.is_err());
    }

    #[test]
    fn new_with_entropy_generator_valid_version_succeeds() {
        let platform_version = PlatformVersion::latest();
        let result = DocumentFactory::new_with_entropy_generator(
            platform_version.protocol_version,
            Box::new(TestEntropyGenerator),
        );
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), DocumentFactory::V0(_)));
    }

    #[test]
    fn new_variant_is_v0() {
        let platform_version = PlatformVersion::latest();
        let factory = DocumentFactory::new(platform_version.protocol_version).unwrap();
        match factory {
            DocumentFactory::V0(_) => {}
        }
    }

    // ----- create_document (error paths) -------------------------------------

    #[test]
    fn create_document_with_invalid_type_returns_error() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([0xAAu8; 32]);

        let result = factory.create_document(
            &data_contract,
            owner_id,
            "nonExistentDocType".to_string(),
            Value::Null,
        );

        // InvalidDocumentTypeError is a DataContract error wrapped in ProtocolError.
        assert!(result.is_err(), "expected error for unknown type");
    }

    #[test]
    fn create_document_with_failing_entropy_returns_error() {
        // Sanity-check: entropy generator errors surface as ProtocolError.
        let platform_version = PlatformVersion::latest();
        let created = get_data_contract_fixture(None, 0, platform_version.protocol_version);
        let data_contract = created.data_contract_owned();
        let factory = DocumentFactory::new_with_entropy_generator(
            platform_version.protocol_version,
            Box::new(FailingEntropyGenerator),
        )
        .unwrap();

        let owner_id = Identifier::from([0x11u8; 32]);
        let result = factory.create_document(
            &data_contract,
            owner_id,
            "noTimeDocument".to_string(),
            platform_value!({ "name": "x" }),
        );
        assert!(result.is_err(), "failing entropy generator should surface");
    }

    #[test]
    fn create_document_happy_path_has_zero_time_based_props() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([0xBBu8; 32]);

        let doc = factory
            .create_document(
                &data_contract,
                owner_id,
                "noTimeDocument".to_string(),
                platform_value!({ "name": "widget" }),
            )
            .expect("document should be created");

        // `create_document_without_time_based_properties` is called internally;
        // verify the time-based metadata was not populated.
        assert_eq!(doc.owner_id(), owner_id);
        assert_eq!(doc.created_at(), None);
        assert_eq!(doc.updated_at(), None);
    }

    // ----- create_extended_document (error paths) ----------------------------

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_document_with_invalid_type_returns_error() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([0xCCu8; 32]);

        let result = factory.create_extended_document(
            &data_contract,
            owner_id,
            "bogusTypeName".to_string(),
            Value::Null,
        );
        assert!(result.is_err());
    }

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_document_happy_path() {
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([0xDDu8; 32]);

        let result = factory.create_extended_document(
            &data_contract,
            owner_id,
            "noTimeDocument".to_string(),
            platform_value!({ "name": "z" }),
        );
        assert!(result.is_ok(), "extended document creation should succeed");
        let ext = result.unwrap();
        assert_eq!(ext.data_contract_id(), data_contract.id());
        assert_eq!(ext.document_type_name(), "noTimeDocument");
        // Entropy matches our deterministic generator.
        assert_eq!(ext.entropy().to_buffer(), [7u8; 32]);
    }

    // ----- create_extended_from_document_buffer ------------------------------

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_from_document_buffer_roundtrips() {
        use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        let (factory, data_contract) = setup_factory();
        let owner_id = Identifier::from([0x55u8; 32]);
        let platform_version = PlatformVersion::latest();

        let doc = factory
            .create_document(
                &data_contract,
                owner_id,
                "noTimeDocument".to_string(),
                platform_value!({ "name": "abc" }),
            )
            .expect("doc should be created");

        let doc_type = data_contract
            .document_type_for_name("noTimeDocument")
            .unwrap();
        let bytes = doc
            .serialize(doc_type, &data_contract, platform_version)
            .expect("serialize");

        let ext = factory
            .create_extended_from_document_buffer(
                bytes.as_slice(),
                "noTimeDocument",
                &data_contract,
                platform_version,
            )
            .expect("extended doc should be parsed");

        assert_eq!(ext.data_contract_id(), data_contract.id());
        assert_eq!(ext.document_type_name(), "noTimeDocument");
        // Buffer-derived extended docs have default (zero) entropy.
        assert_eq!(ext.entropy(), &Bytes32::default());
    }

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_from_document_buffer_invalid_type_fails() {
        let (factory, data_contract) = setup_factory();
        let platform_version = PlatformVersion::latest();

        let result = factory.create_extended_from_document_buffer(
            &[0u8; 16],
            "thisTypeDoesNotExist",
            &data_contract,
            platform_version,
        );
        assert!(result.is_err(), "unknown doc type should surface error");
    }

    #[cfg(feature = "extended-document")]
    #[test]
    fn create_extended_from_document_buffer_malformed_bytes_fails() {
        let (factory, data_contract) = setup_factory();
        let platform_version = PlatformVersion::latest();

        // Totally random bytes should not deserialize as a Document.
        let result = factory.create_extended_from_document_buffer(
            &[0xFFu8; 6],
            "noTimeDocument",
            &data_contract,
            platform_version,
        );
        assert!(result.is_err(), "malformed buffer should fail to decode");
    }

    // ----- create_state_transition (error paths) -----------------------------

    #[cfg(feature = "state-transitions")]
    mod state_transition_tests {
        use super::*;
        use crate::document::errors::DocumentError;
        use crate::document::{DocumentV0Setters, INITIAL_REVISION};
        use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
        use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;
        use crate::state_transition::StateTransitionOwned;

        fn build_doc(
            factory: &DocumentFactory,
            data_contract: &DataContract,
            owner: Identifier,
            type_name: &str,
        ) -> Document {
            factory
                .create_document(
                    data_contract,
                    owner,
                    type_name.to_string(),
                    platform_value!({ "name": "x" }),
                )
                .expect("doc should build")
        }

        #[test]
        fn create_state_transition_empty_iter_returns_error() {
            let (factory, _) = setup_factory();
            let mut nonce_counter: BTreeMap<(Identifier, Identifier), u64> = BTreeMap::new();
            let empty: Vec<(
                DocumentTransitionActionType,
                Vec<(Document, DocumentTypeRef, Bytes32, Option<TokenPaymentInfo>)>,
            )> = vec![];

            let result = factory.create_state_transition(empty, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e)) if matches!(*e, DocumentError::NoDocumentsSuppliedError)
                ),
                "expected NoDocumentsSuppliedError"
            );
        }

        #[test]
        fn create_state_transition_outer_iter_has_empty_inner_returns_error() {
            // An outer entry with an empty Vec should also yield NoDocumentsSupplied.
            let (factory, _) = setup_factory();
            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(DocumentTransitionActionType::Create, vec![])];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e)) if matches!(*e, DocumentError::NoDocumentsSuppliedError)
                ),
                "expected NoDocumentsSuppliedError"
            );
        }

        #[test]
        fn create_state_transition_mismatched_owner_returns_error() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner_a = Identifier::from([0x01u8; 32]);
            let owner_b = Identifier::from([0x02u8; 32]);
            let doc_a = build_doc(&factory, &data_contract, owner_a, "noTimeDocument");
            let doc_b = build_doc(&factory, &data_contract, owner_b, "noTimeDocument");

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![
                    (doc_a, doc_type, Bytes32::new([1u8; 32]), None),
                    (doc_b, doc_type, Bytes32::new([2u8; 32]), None),
                ],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::MismatchOwnerIdsError { .. })
                ),
                "expected MismatchOwnerIdsError"
            );
        }

        #[test]
        fn create_state_transition_create_wrong_initial_revision_errors() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner = Identifier::from([0x05u8; 32]);
            let mut doc = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            doc.set_revision(Some(9999));

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::InvalidInitialRevisionError { .. })
                ),
                "expected InvalidInitialRevisionError"
            );
        }

        #[test]
        fn create_state_transition_create_missing_revision_on_mutable_errors() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner = Identifier::from([0x06u8; 32]);
            let mut doc = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            doc.set_revision(None);

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::RevisionAbsentError { .. })
                ),
                "expected RevisionAbsentError"
            );
        }

        #[test]
        fn create_state_transition_replace_missing_revision_errors() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner = Identifier::from([0x07u8; 32]);
            let mut doc = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            doc.set_revision(None);

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Replace,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::RevisionAbsentError { .. })
                ),
                "expected RevisionAbsentError for replace"
            );
        }

        #[test]
        fn create_state_transition_delete_missing_revision_errors() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner = Identifier::from([0x08u8; 32]);
            let mut doc = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            doc.set_revision(None);

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![(
                DocumentTransitionActionType::Delete,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let result = factory.create_state_transition(entries, &mut nonce_counter);
            assert!(
                matches!(
                    result,
                    Err(ProtocolError::Document(e))
                        if matches!(*e, DocumentError::RevisionAbsentError { .. })
                ),
                "expected RevisionAbsentError for delete"
            );
        }

        #[test]
        fn create_state_transition_create_increments_nonce() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner = Identifier::from([0x20u8; 32]);
            let doc = build_doc(&factory, &data_contract, owner, "noTimeDocument");

            let mut nonce_counter = BTreeMap::new();
            nonce_counter.insert((owner, data_contract.id()), 7);

            let entries = vec![(
                DocumentTransitionActionType::Create,
                vec![(doc, doc_type, Bytes32::default(), None)],
            )];
            let batch = factory
                .create_state_transition(entries, &mut nonce_counter)
                .expect("should build");
            assert_eq!(batch.owner_id(), owner);
            assert_eq!(batch.transitions_len(), 1);
            // Pre-seeded nonce 7 → 8.
            assert_eq!(*nonce_counter.get(&(owner, data_contract.id())).unwrap(), 8);
        }

        #[test]
        fn create_state_transition_mix_actions_combines_transitions() {
            let (factory, data_contract) = setup_factory();
            let doc_type = data_contract
                .document_type_for_name("noTimeDocument")
                .unwrap();
            let owner = Identifier::from([0x30u8; 32]);

            // Two create docs (same owner, distinct ids).
            let mut c1 = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            c1.set_id(Identifier::from([0xAAu8; 32]));
            let mut c2 = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            c2.set_id(Identifier::from([0xBBu8; 32]));

            // One replace doc — must be mutable + have revision.
            let mut r1 = build_doc(&factory, &data_contract, owner, "noTimeDocument");
            r1.set_id(Identifier::from([0xCCu8; 32]));
            assert_eq!(r1.revision(), Some(INITIAL_REVISION));

            let mut nonce_counter = BTreeMap::new();
            let entries = vec![
                (
                    DocumentTransitionActionType::Create,
                    vec![
                        (c1, doc_type, Bytes32::new([0x01; 32]), None),
                        (c2, doc_type, Bytes32::new([0x02; 32]), None),
                    ],
                ),
                (
                    DocumentTransitionActionType::Replace,
                    vec![(r1, doc_type, Bytes32::default(), None)],
                ),
            ];
            let batch = factory
                .create_state_transition(entries, &mut nonce_counter)
                .expect("mixed batch should build");
            assert_eq!(batch.transitions_len(), 3);
            // 2 creates + 1 replace = nonce increments 3 times for same (owner, contract).
            assert_eq!(*nonce_counter.get(&(owner, data_contract.id())).unwrap(), 3);
        }
    }
}

//
// #[cfg(test)]
// mod old_disabled_test {
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
