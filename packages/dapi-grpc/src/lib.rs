pub use prost::Message;

pub mod core {
    #[cfg(feature = "core_v0")]
    pub mod v0 {
        include!("core/proto/org.dash.platform.dapi.v0.rs");
    }
}

pub mod platform {
    #[cfg(feature = "platform_v0")]
    pub mod v0 {
        include!("platform/proto/org.dash.platform.dapi.v0.rs");
    }
}

#[cfg(test)]
mod test {
    use crate::platform::v0::platform_client::PlatformClient;

    #[cfg(feature = "testnet")]
    #[tokio::test]
    async fn testnet_request() {
        use crate::platform::v0::get_identity_response::Result as ResponseResult;
        use crate::platform::v0::GetIdentityRequest;
        use tonic::transport::{Channel, ClientTlsConfig};

        let endpoint = Channel::from_static("https://seed-1.testnet.networks.dash.org:1443")
            .tls_config(ClientTlsConfig::new())
            .unwrap();

        // FIXME: fails with H2NotNegotiated - it needs http/2 support
        let mut c = PlatformClient::connect(endpoint).await.unwrap();
        let request = GetIdentityRequest {
            id: vec![1u8; 32],
            prove: true,
        };
        let response = c.get_identity(request).await.unwrap();
        let response = response.get_ref();
        assert!(matches!(
            response.result.as_ref().unwrap(),
            ResponseResult::Proof(_)
        ));
    }
}
