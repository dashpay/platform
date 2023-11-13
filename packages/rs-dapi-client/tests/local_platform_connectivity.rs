#[cfg(feature = "platform_integration_tests")]
mod tests {
    use dapi_grpc::platform::v0::{
        self as platform_proto, get_identity_response, GetIdentityRequest, GetIdentityResponse,
        ResponseMetadata,
    };
    use rs_dapi_client::{AddressList, DapiClient, DapiRequest, RequestSettings};

    pub const OWNER_ID_BYTES: [u8; 32] = [
        65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50,
        60, 215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
    ];

    #[tokio::test]
    async fn get_identity() {
        let mut address_list = AddressList::new();
        address_list.add_uri(rs_dapi_client::Uri::from_static("http://127.0.0.1:2443"));

        let mut client = DapiClient::new(address_list, RequestSettings::default());
        let request = platform_proto::GetIdentityRequest {
            id: OWNER_ID_BYTES.to_vec(),
            prove: false,
        };

        if let GetIdentityResponse {
            result: Some(get_identity_response::Result::Identity(bytes)),
            metadata: Some(ResponseMetadata {
                protocol_version, ..
            }),
            ..
        } = request
            .execute(&mut client, RequestSettings::default())
            .await
            .expect("unable to perform dapi request")
        {
            assert!(bytes.len() > 0);
            assert_eq!(protocol_version, 1);
        } else {
            panic!("no identity was received");
        }
    }
}
