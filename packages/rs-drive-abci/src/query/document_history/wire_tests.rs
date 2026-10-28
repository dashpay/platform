use super::*;
use crate::query::tests::setup_platform;
use dapi_grpc::platform::v0::Proof;
use dapi_grpc::Message;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use drive::drive::Drive;
use drive::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};

// These independent transport types pin the timestamp/offset format. They do
// not depend on the generated definitions used by the server under test.
#[derive(Clone, PartialEq, prost::Message)]
struct ReleasedRequest {
    #[prost(message, optional, tag = "1")]
    v0: Option<ReleasedRequestV0>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ReleasedRequestV0 {
    #[prost(bytes = "vec", tag = "1")]
    data_contract_id: Vec<u8>,
    #[prost(string, tag = "2")]
    document_type_name: String,
    #[prost(bytes = "vec", tag = "3")]
    document_id: Vec<u8>,
    #[prost(message, optional, tag = "4")]
    limit: Option<u32>,
    #[prost(message, optional, tag = "5")]
    offset: Option<u32>,
    #[prost(uint64, tag = "6")]
    start_at_ms: u64,
    #[prost(bool, tag = "7")]
    prove: bool,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ReleasedResponse {
    #[prost(message, optional, tag = "1")]
    v0: Option<ReleasedResponseV0>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ReleasedResponseV0 {
    #[prost(message, optional, tag = "1")]
    document_history: Option<ReleasedHistory>,
    #[prost(message, optional, tag = "2")]
    proof: Option<Proof>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ReleasedHistory {
    #[prost(message, repeated, tag = "1")]
    document_entries: Vec<ReleasedEntry>,
}

#[derive(Clone, PartialEq, prost::Message)]
struct ReleasedEntry {
    #[prost(uint64, tag = "1")]
    date: u64,
    #[prost(bytes = "vec", tag = "2")]
    value: Vec<u8>,
}

#[test]
fn should_refuse_document_history_v0_after_activation_before_reading_state() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let result = platform
        .query_document_history(
            GetDocumentHistoryRequest {
                version: Some(RequestVersion::V0(Default::default())),
            },
            &state,
            version,
        )
        .unwrap();
    assert!(
        matches!(&result.errors[0], QueryError::UnsupportedQueryVersion(name, 1, 1, protocol, 0)
        if name == "document_history" && *protocol == version.protocol_version)
    );
}

#[test]
fn should_refuse_document_history_v1_before_activation_before_reading_state() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, Some(14));
    let result = platform
        .query_document_history(
            GetDocumentHistoryRequest {
                version: Some(RequestVersion::V1(Default::default())),
            },
            &state,
            version,
        )
        .unwrap();
    assert!(
        matches!(&result.errors[0], QueryError::UnsupportedQueryVersion(name, 0, 0, 14, 1)
        if name == "document_history")
    );
}

#[test]
fn should_serve_released_document_history_wire_requests_before_activation() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, Some(14));
    let contract = json_document_to_contract(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../rs-drive/tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json"),
        false,
        version,
    ).unwrap();
    platform
        .drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../rs-drive/tests/supporting_files/contract/dashpay/profile0.json"
        ),
        Some([8; 32].into()),
        document_type,
        version,
    )
    .unwrap();
    let mut expected = Vec::new();
    for (revision, time) in [(1, 1000), (2, 2000), (3, 3000)] {
        document.set_revision(Some(revision));
        platform
            .drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(time),
                true,
                None,
                version,
                None,
            )
            .unwrap();
        expected.push((
            time,
            document
                .serialize(document_type, &contract, version)
                .unwrap(),
        ));
    }
    for (prove, offset) in [
        (false, None),
        (false, Some(0)),
        (false, Some(1)),
        (true, Some(0)),
    ] {
        let wire = ReleasedRequest {
            v0: Some(ReleasedRequestV0 {
                data_contract_id: contract.id().to_vec(),
                document_type_name: "profile".to_owned(),
                document_id: document.id().to_vec(),
                limit: Some(2),
                offset,
                start_at_ms: 1000,
                prove,
            }),
        }
        .encode_to_vec();
        let request = GetDocumentHistoryRequest::decode(wire.as_slice())
            .expect("a timestamp/offset history client must remain wire compatible");
        let result = platform
            .query_document_history(request, &state, version)
            .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let response = result.data.unwrap();
        let released = ReleasedResponse::decode(response.encode_to_vec().as_slice())
            .unwrap()
            .v0
            .unwrap();
        let expected = expected
            .iter()
            .skip(1 + offset.unwrap_or(0) as usize)
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        if prove {
            let proof = released.proof.expect("a version 0 proof response");
            let (_, verified) = Drive::verify_document_history_legacy(
                &proof.grovedb_proof,
                contract.id().to_buffer(),
                "profile",
                document_type,
                document.id().to_buffer(),
                1000,
                Some(2),
                Some(0),
                version,
            )
            .unwrap();
            assert_eq!(
                verified
                    .unwrap()
                    .into_iter()
                    .map(|(time, document)| (
                        time,
                        document
                            .serialize(document_type, &contract, version)
                            .unwrap()
                    ))
                    .collect::<Vec<_>>(),
                expected
            );
            let path_query = Drive::fetch_document_history_query(
                contract.id().to_buffer(),
                "profile",
                document.id().to_buffer(),
                1000,
                Some(2),
                Some(0),
                version,
            )
            .unwrap();
            let original_proof = platform
                .drive
                .grove_get_proved_path_query(&path_query, None, &mut Vec::new(), &version.drive)
                .unwrap();
            assert_eq!(proof.grovedb_proof, original_proof);
        } else {
            assert_eq!(
                released
                    .document_history
                    .unwrap()
                    .document_entries
                    .into_iter()
                    .map(|entry| (entry.date, entry.value))
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}
