use dapi_grpc::core::v0 as core;
use dapi_grpc::Message;

fn main() {
    let request = core::GetBlockRequest {
        block: Some(core::get_block_request::Block::Height(123)),
    };
    let mut buffer = Vec::<u8>::new();
    request.encode(&mut buffer).expect("failed to encode data");

    let decoded = core::GetBlockRequest::decode(buffer.as_ref()).expect("failed to decode data");

    assert_eq!(request, decoded);
}
