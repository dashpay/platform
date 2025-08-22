pub use prost::Message;

#[cfg(feature = "core")]
pub mod core {
    #![allow(non_camel_case_types)]
    pub mod v0 {
        // Note: only one of the features can be analyzed at a time
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        include!("core/server/org.dash.platform.dapi.v0.rs");

        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        include!("core/client/org.dash.platform.dapi.v0.rs");

        #[cfg(target_arch = "wasm32")]
        include!("core/wasm/org.dash.platform.dapi.v0.rs");
    }
}

#[cfg(feature = "platform")]
pub mod platform {
    pub mod v0 {
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        include!("platform/server/org.dash.platform.dapi.v0.rs");

        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        include!("platform/client/org.dash.platform.dapi.v0.rs");

        #[cfg(target_arch = "wasm32")]
        include!("platform/wasm/org.dash.platform.dapi.v0.rs");
    }

    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;

    #[cfg(any(feature = "server", feature = "client", target_arch = "wasm32"))]
    mod versioning;
    #[cfg(any(feature = "server", feature = "client", target_arch = "wasm32"))]
    pub use versioning::{VersionedGrpcMessage, VersionedGrpcResponse};
}

#[cfg(all(feature = "drive", feature = "platform"))]
pub(crate) mod dapi {
    pub(crate) use crate::platform::*;
}

#[cfg(feature = "drive")]
pub mod drive {
    pub mod v0 {
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        include!("drive/server/org.dash.platform.drive.v0.rs");

        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        include!("drive/client/org.dash.platform.drive.v0.rs");
    }

    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;
}

#[cfg(feature = "serde")]
// Serde deserialization logic
pub mod deserialization;

// We need mock module even if the feature is disabled
pub mod mock;

// Re-export tonic to ensure everyone uses the same version
pub use tonic;
