use dapi_grpc::platform::v0 as platform;
use dapi_grpc::platform::v0::get_consensus_params_request::GetConsensusParamsRequestV0;
use dapi_grpc::Message;

fn main() {
    let request = platform::GetConsensusParamsRequest {
        version: Some(platform::get_consensus_params_request::Version::V0(
            GetConsensusParamsRequestV0 {
                prove: true,
                height: 123,
            },
        )),
    };

    let mut buffer = Vec::<u8>::new();
    request.encode(&mut buffer).expect("failed to encode data");
    let decoded = platform::GetConsensusParamsRequest::decode(buffer.as_ref())
        .expect("failed to decode data");
    assert_eq!(request, decoded);
}
