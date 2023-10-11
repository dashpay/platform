use dapi_grpc::platform::v0::{GetIdentityRequest, GetIdentityResponse, Proof};

use rs_dapi_client::{mock::MockDapiClient, Dapi, DapiRequest, RequestSettings};

#[tokio::test]
async fn test_mock_get_identity_dapi_client() {
    let mut dapi = MockDapiClient::new();

    let request = GetIdentityRequest::default();
    let response = GetIdentityResponse {
        result: Some(
            dapi_grpc::platform::v0::get_identity_response::Result::Proof(Proof {
                quorum_type: 106,
                ..Default::default()
            }),
        ),
        metadata: Default::default(),
    };

    dapi.expect(&request, &response);

    let settings = RequestSettings::default();

    let result = dapi.execute(request.clone(), settings).await.unwrap();

    let result2 = request.execute(&mut dapi, settings).await.unwrap();

    assert_eq!(result, response);
    assert_eq!(result2, response);
}
