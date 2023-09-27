use dapi_grpc::platform::v0::GetIdentityRequest;

use rs_dapi_client::{
    mock::{MockDapiClient, MockRequest},
    Dapi, RequestSettings,
};

#[tokio::test]
async fn test_mock_get_identity_dapi_client() {
    let req = GetIdentityRequest::default();
    let resp = dapi_grpc::platform::v0::GetIdentityResponse::default();

    let mock_req = MockRequest::new(req, resp.clone());

    let mut d = MockDapiClient::new();
    let settings = RequestSettings::default();
    let result = d.execute(mock_req, settings).await.unwrap();

    assert_eq!(result, resp);
}
