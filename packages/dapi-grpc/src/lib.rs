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
        use tonic::IntoRequest;

        include!("platform/proto/org.dash.platform.dapi.v0.rs");

        /// Request to get identity balance.
        // We need to create separate GetIdentityBalanceRequest
        // because it has different response type
        //
        // TODO: Implement GetIdentityBalanceRequest in platform.proto  and remove this one
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct GetIdentityBalanceRequest(pub GetIdentityRequest);
        impl IntoRequest<GetIdentityRequest> for GetIdentityBalanceRequest {
            fn into_request(self) -> tonic::Request<GetIdentityRequest> {
                self.0.into_request()
            }
        }
    }
    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;
}

#[cfg(feature = "serde")]
// Serde deserialization logic
pub mod deserialization;
