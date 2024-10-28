#[cfg(feature = "mocks")]
use {
    dapi_grpc::platform::v0::{GetIdentityRequest, GetIdentityResponse, Proof},
    rs_dapi_client::{
        mock::MockDapiClient, DapiRequest, DapiRequestExecutor, ExecutionResponse, RequestSettings,
    },
};

#[tokio::test]
#[cfg(feature = "mocks")]
async fn test_mock_get_identity_dapi_client() {
    let mut dapi = MockDapiClient::new();

    let request = GetIdentityRequest::default();
    let inner: GetIdentityResponse = GetIdentityResponse {
        version: Some(dapi_grpc::platform::v0::get_identity_response::Version::V0(dapi_grpc::platform::v0::get_identity_response::GetIdentityResponseV0 {
            result: Some(
                dapi_grpc::platform::v0::get_identity_response::get_identity_response_v0::Result::Proof(Proof {
                    quorum_type: 106,
                    ..Default::default()
                }),
            ),
            metadata: Default::default(),
        }))
    };
    let execution_response = ExecutionResponse {
        inner,
        retries: 0,
        address: "http://127.0.0.1:9000"
            .parse()
            .expect("failed to parse address"),
    };

    dapi.expect(&request, &Ok(execution_response.clone()))
        .expect("expectation added");

    let settings = RequestSettings::default();

    let result = dapi.execute(request.clone(), settings).await.unwrap();

    let result2 = request.execute(&dapi, settings).await.unwrap();

    assert_eq!(result, execution_response);
    assert_eq!(result2, execution_response);
}
