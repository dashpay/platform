use dapi_grpc::platform::v0 as platform;
use prost::Message;

fn main() {
    let request = platform::GetConsensusParamsRequest {
        height: 123,
        prove: false,
    };

    let mut buffer = Vec::<u8>::new();
    request.encode(&mut buffer).expect("failed to encode data");
    let decoded = platform::GetConsensusParamsRequest::decode(buffer.as_ref())
        .expect("failed to decode data");
    assert_eq!(request, decoded);
}
