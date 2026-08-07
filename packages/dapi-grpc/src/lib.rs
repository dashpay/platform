//! Protobuf message types and generated gRPC stubs for DAPI.
//!
//! # Feature flags
//!
//! | feature | meaning |
//! |---|---|
//! | `core` / `platform` / `drive` | which proto surfaces are generated |
//! | `client` | generate client stubs (generic over the transport) |
//! | `transport` | tonic's opt-in native transport: `connect()` on generated clients and TLS roots. Pulls the hyper/rustls transport stack. |
//! | `server` | generate server stubs; implies `client`, `drive`, `transport` |
//! | `serde` / `mocks` | serde derives / dump-and-replay support |
//!
//! Types-only consumers (proof verification, embedders with their own
//! transport) build with `default-features = false, features = ["platform",
//! "client"]` and get message types plus transport-generic client stubs with
//! no native networking stack in the dependency tree. The default feature set
//! is also wasm32-safe; native consumers that call generated `connect()`
//! methods must enable `transport` explicitly.

pub use prost::Message;

#[cfg(feature = "core")]
pub mod core {
    #![allow(non_camel_case_types)]
    pub mod v0 {
        // Note: only one of the features can be analyzed at a time
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/core/server/org.dash.platform.dapi.v0.rs"
        ));

        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/core/client/org.dash.platform.dapi.v0.rs"
        ));

        #[cfg(target_arch = "wasm32")]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/core/wasm/org.dash.platform.dapi.v0.rs"
        ));

        /// Serialized `FileDescriptorSet` for the Core proto — lets consumers
        /// enumerate the served rpcs (e.g. to assert metrics allowlists).
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/core/server/descriptor.bin"
        ));

        /// Serialized `FileDescriptorSet` for the Core proto — lets consumers
        /// enumerate the served rpcs (e.g. to assert metrics allowlists).
        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/core/client/descriptor.bin"
        ));
    }
}

#[cfg(feature = "platform")]
pub mod platform {
    pub mod v0 {
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/platform/server/org.dash.platform.dapi.v0.rs"
        ));

        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/platform/client/org.dash.platform.dapi.v0.rs"
        ));

        #[cfg(target_arch = "wasm32")]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/platform/wasm/org.dash.platform.dapi.v0.rs"
        ));

        /// Serialized `FileDescriptorSet` for the Platform proto — lets
        /// consumers enumerate the served rpcs (e.g. to assert metrics
        /// allowlists).
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/platform/server/descriptor.bin"
        ));

        /// Serialized `FileDescriptorSet` for the Platform proto — lets
        /// consumers enumerate the served rpcs (e.g. to assert metrics
        /// allowlists).
        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/platform/client/descriptor.bin"
        ));
    }

    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;

    #[cfg(any(feature = "server", feature = "client", target_arch = "wasm32"))]
    mod versioning;
    #[cfg(any(feature = "server", feature = "client", target_arch = "wasm32"))]
    pub use versioning::{
        MerkProofVersionedGrpcResponse, VersionedGrpcMessage, VersionedGrpcResponse,
    };
}

#[cfg(all(feature = "drive", feature = "platform"))]
pub(crate) mod dapi {
    pub(crate) use crate::platform::*;
}

#[cfg(feature = "drive")]
pub mod drive {
    pub mod v0 {
        #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/drive/server/org.dash.platform.drive.v0.rs"
        ));

        #[cfg(all(
            feature = "client",
            not(feature = "server"),
            not(target_arch = "wasm32")
        ))]
        include!(concat!(
            env!("DAPI_GRPC_OUT_DIR"),
            "/drive/client/org.dash.platform.drive.v0.rs"
        ));
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
// Ensure the prost codec crate is linked and available to generated code
pub use tonic_prost;
