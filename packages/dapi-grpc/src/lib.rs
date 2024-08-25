pub use prost::Message;

#[cfg(feature = "core")]
pub mod core {
    #![allow(non_camel_case_types)]
    pub mod v0 {
        #[cfg(all(feature = "server", not(feature = "client")))]
        include!("core/server/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "client", not(feature = "server")))]
        include!("core/client/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "server", feature = "client"))]
        include!("core/client_server/org.dash.platform.dapi.v0.rs");
    }
}

#[cfg(feature = "platform")]
pub mod platform {
    pub mod v0 {
        #[cfg(all(feature = "server", not(feature = "client")))]
        include!("platform/server/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "client", not(feature = "server")))]
        include!("platform/client/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "server", feature = "client"))]
        include!("platform/client_server/org.dash.platform.dapi.v0.rs");
    }

    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;

    #[cfg(any(feature = "server", feature = "client"))]
    mod versioning;
    #[cfg(any(feature = "server", feature = "client"))]
    pub use versioning::{VersionedGrpcMessage, VersionedGrpcResponse};
}

#[cfg(feature = "serde")]
// Serde deserialization logic
pub mod deserialization;

// We need mock module even if the feature is disabled
pub mod mock;

// Re-export tonic to ensure everyone uses the same version
pub use tonic;
