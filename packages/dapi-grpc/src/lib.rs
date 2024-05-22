pub use prost::Message;

#[cfg(feature = "core")]
pub mod core {
    #![allow(non_camel_case_types)]
    pub mod v0 {
        // non-serde

        #[cfg(all(feature = "server", not(feature = "client"), not(feature = "serde")))]
        include!("core/server/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "client", not(feature = "server"), not(feature = "serde")))]
        include!("core/client/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "server", feature = "client", not(feature = "serde")))]
        include!("core/client_server/org.dash.platform.dapi.v0.rs");

        // serde

        #[cfg(all(feature = "server", not(feature = "client"), feature = "serde"))]
        include!("core/server_serde/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "client", not(feature = "server"), feature = "serde"))]
        include!("core/client_serde/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "server", feature = "client", feature = "serde"))]
        include!("core/client_server_serde/org.dash.platform.dapi.v0.rs");
    }
}

#[cfg(feature = "platform")]
pub mod platform {
    pub mod v0 {
        // non-serde

        #[cfg(all(feature = "server", not(feature = "client"), not(feature = "serde")))]
        include!("platform/server/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "client", not(feature = "server"), not(feature = "serde")))]
        include!("platform/client/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "server", feature = "client", not(feature = "serde")))]
        include!("platform/client_server/org.dash.platform.dapi.v0.rs");

        // serde

        #[cfg(all(feature = "server", not(feature = "client"), feature = "serde"))]
        include!("platform/server_serde/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "client", not(feature = "server"), feature = "serde"))]
        include!("platform/client_serde/org.dash.platform.dapi.v0.rs");

        #[cfg(all(feature = "server", feature = "client", feature = "serde"))]
        include!("platform/client_server_serde/org.dash.platform.dapi.v0.rs");
    }

    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;

    mod versioning;
    pub use versioning::{VersionedGrpcMessage, VersionedGrpcResponse};
}

#[cfg(feature = "serde")]
// Serde deserialization logic
pub mod deserialization;

// We need mock module even if the feature is disabled
pub mod mock;

// Re-export tonic to ensure everyone uses the same version
pub use tonic;
