//! A document query answer is verified at the protocol version its quorum signed, and the
//! signature covers that version.
mod signed_response;

use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, GetDocumentsResponseV0, Version,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, ResponseMetadata};
use dapi_grpc::Message;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::{DataContract, Identifier};
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tests::fixtures::get_dpns_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::DriveDocumentQuery;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::{Error, FromProof};
use signed_response::{signed, Provider, NEWER_PROTOCOL_VERSION};

const DOCUMENT_TYPE: &str = "preorder";

fn latest() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn previous() -> &'static PlatformVersion {
    PlatformVersion::get(latest().protocol_version - 1).expect("previous protocol version")
}

fn query(contract: &DataContract) -> DriveDocumentQuery<'_> {
    DriveDocumentQuery::all_items_query(
        contract,
        contract.document_type_for_name(DOCUMENT_TYPE).unwrap(),
        Some(10),
    )
}

/// A state holding one document, produced at `producer`, and the test quorum's answer to
/// [`query`] over it, signed at `producer`.
fn signed_answer(producer: &PlatformVersion) -> (DataContract, Document, GetDocumentsResponse) {
    let drive = setup_drive_with_initial_state_structure(Some(producer));
    let contract = get_dpns_data_contract_fixture(
        Some(Identifier::new([4; 32])),
        0,
        producer.protocol_version,
    )
    .data_contract_owned();
    drive
        .insert_contract(&contract, BlockInfo::default(), true, None, producer)
        .unwrap();
    let document_type = contract.document_type_for_name(DOCUMENT_TYPE).unwrap();
    let document = document_type.random_document(Some(99), producer).unwrap();
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: Some(document.owner_id().to_buffer()),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            producer,
            None,
        )
        .unwrap();
    let (bytes, _) = query(&contract)
        .execute_with_proof(&drive, None, None, producer)
        .unwrap();
    let root = drive
        .grove
        .root_hash(None, &producer.drive.grove_version)
        .unwrap()
        .unwrap();
    let metadata = ResponseMetadata {
        height: 10,
        core_chain_locked_height: 1,
        time_ms: 1_700_000_000_000,
        protocol_version: producer.protocol_version,
        chain_id: "dash-testnet-51".into(),
        ..Default::default()
    };
    let response = GetDocumentsResponse {
        version: Some(Version::V0(GetDocumentsResponseV0 {
            result: Some(get_documents_response_v0::Result::Proof(signed(
                bytes, &metadata, root,
            ))),
            metadata: Some(metadata),
        })),
    };
    (contract, document, response)
}

fn set_metadata_protocol_version(response: &mut GetDocumentsResponse, protocol_version: u32) {
    let Some(Version::V0(v0)) = response.version.as_mut() else {
        panic!("v0 response")
    };
    v0.metadata.as_mut().unwrap().protocol_version = protocol_version;
}

fn verify(
    contract: &DataContract,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
) -> Result<Option<Documents>, Box<Error>> {
    <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
        query(contract),
        response,
        Network::Testnet,
        platform_version,
        &Provider,
    )
    .map(|(documents, _, _)| documents)
    .map_err(Box::new)
}

/// Answers are verified at the version that signed them.
#[test]
fn should_verify_a_signed_document_answer_at_its_signed_version() {
    for producer in [previous(), latest()] {
        let (contract, document, response) = signed_answer(producer);
        let documents = verify(&contract, response, producer)
            .expect("an answer verifies at the version that signed it")
            .expect("the proved page");
        assert_eq!(
            documents.get(&document.id()),
            Some(&Some(document.clone())),
            "protocol {}",
            producer.protocol_version
        );
    }
}

/// Answers are verified at the version that signed them, so one signed at the previous
/// version is not verified with the latest rules.
#[test]
fn should_refuse_a_signed_document_answer_at_a_newer_verifier_version() {
    let (contract, _, response) = signed_answer(previous());
    let error = verify(&contract, response, latest()).unwrap_err();
    assert!(
        matches!(
            *error,
            Error::ProofProtocolVersionMismatch { signed, verifier }
                if signed == previous().protocol_version
                    && verifier == latest().protocol_version
        ),
        "{error:?}"
    );
}

/// The quorum signature covers the protocol version in the metadata: an answer whose
/// metadata version is changed after signing fails the signature check.
#[test]
fn should_bind_the_protocol_version_of_a_document_answer_to_its_signature() {
    let (contract, _, mut response) = signed_answer(latest());
    set_metadata_protocol_version(&mut response, NEWER_PROTOCOL_VERSION);
    let error = verify(&contract, response, latest()).unwrap_err();
    assert!(
        matches!(*error, Error::InvalidSignature { .. }),
        "{error:?}"
    );
}

/// Writes the signed answer the SDK tests replay as a node answering at the SDK's protocol
/// version: all `preorder` documents, limit 10, of `pv14-contract.bin`, signed at protocol 14.
#[test]
#[ignore = "regenerates the signed document query answer the SDK tests replay"]
fn should_export_document_answer_sdk_fixture() {
    // Fixed at protocol 14, not the latest version, so regenerating it rewrites the same
    // answer the SDK tests read.
    let producer = PlatformVersion::get(14).expect("protocol 14 is known");
    let (contract, document, response) = signed_answer(producer);
    let documents = verify(&contract, response.clone(), producer)
        .expect("the answer verifies at the version that signed it")
        .expect("the proved page");
    assert_eq!(documents.get(&document.id()), Some(&Some(document)));
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-drive-proof-verifier/tests/vectors/document-outcome");
    assert_eq!(
        contract
            .serialize_to_bytes_with_platform_version(producer)
            .expect("serialize the contract"),
        std::fs::read(directory.join("pv14-contract.bin")).expect("read pv14-contract.bin"),
        "the answer is over the contract the SDK tests load"
    );
    std::fs::write(
        directory.join("pv14-documents-response.bin"),
        response.encode_to_vec(),
    )
    .expect("write the fixture");
}
