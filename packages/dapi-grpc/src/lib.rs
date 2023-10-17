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
    #[cfg(feature = "tenderdash-proto")]
    pub use tenderdash_proto as proto;
}

#[cfg(feature = "serde")]
// Serde deserialization logic
pub mod deserialization;
