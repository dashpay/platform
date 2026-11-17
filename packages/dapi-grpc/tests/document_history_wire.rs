use dapi_grpc::platform::v0::{GetDocumentHistoryRequest, GetDocumentHistoryResponse};
use dapi_grpc::Message;

#[test]
fn should_decode_released_document_history_offset_and_prove_fields() {
    // The offset wrapper occupies field 5; the proof flag occupies field 7.
    let bytes = [0x0a, 6, 0x2a, 2, 8, 1, 0x38, 1];
    let request = GetDocumentHistoryRequest::decode(bytes.as_slice())
        .expect("a document history request with an offset must remain decodable");
    assert_eq!(request.encode_to_vec(), bytes);
}

#[test]
fn should_decode_released_document_history_document_bytes() {
    // Entry field 2 contains document bytes, including arbitrary non-varints.
    let bytes = [0x0a, 10, 0x0a, 8, 0x0a, 6, 8, 1, 0x12, 2, 0xff, 0xff];
    let response = GetDocumentHistoryResponse::decode(bytes.as_slice())
        .expect("document history entries must preserve their document bytes");
    assert_eq!(response.encode_to_vec(), bytes);
}

#[test]
fn should_keep_composite_document_history_on_its_own_version_tag() {
    let bytes = [0x12, 4, 0x28, 1, 0x40, 2];
    let request = GetDocumentHistoryRequest::decode(bytes.as_slice()).unwrap();
    assert!(
        matches!(&request.version, Some(dapi_grpc::platform::v0::get_document_history_request::Version::V1(v1)) if v1.prove)
    );
    assert_eq!(request.encode_to_vec(), bytes);
}
