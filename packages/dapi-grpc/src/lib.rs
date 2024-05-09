pub use prost::Message;

#[cfg(feature = "core")]
pub mod core {
    #![allow(non_camel_case_types)]
    pub mod v0 {
        include!("core/proto/org.dash.platform.dapi.v0.rs");
    }
}

#[cfg(feature = "platform")]
pub mod platform {
    pub mod v0 {
        include!("platform/proto/org.dash.platform.dapi.v0.rs");
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
