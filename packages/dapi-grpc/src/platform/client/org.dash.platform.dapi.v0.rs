#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Proof {
    #[prost(bytes = "vec", tag = "1")]
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub grovedb_proof: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub quorum_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "4")]
    pub round: u32,
    #[prost(bytes = "vec", tag = "5")]
    #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
    pub block_id_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub quorum_type: u32,
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResponseMetadata {
    #[prost(uint64, tag = "1")]
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::deserialization::from_to_string")
    )]
    pub height: u64,
    #[prost(uint32, tag = "2")]
    pub core_chain_locked_height: u32,
    #[prost(uint32, tag = "3")]
    pub epoch: u32,
    #[prost(uint64, tag = "4")]
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::deserialization::from_to_string")
    )]
    pub time_ms: u64,
    #[prost(uint32, tag = "5")]
    pub protocol_version: u32,
    #[prost(string, tag = "6")]
    pub chain_id: ::prost::alloc::string::String,
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StateTransitionBroadcastError {
    #[prost(uint32, tag = "1")]
    pub code: u32,
    #[prost(string, tag = "2")]
    pub message: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "3")]
    pub data: ::prost::alloc::vec::Vec<u8>,
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub state_transition: ::prost::alloc::vec::Vec<u8>,
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionResponse {}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityRequest {
    #[prost(oneof = "get_identity_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_request::Version>,
}
/// Nested message and enum types in `GetIdentityRequest`.
pub mod get_identity_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityNonceRequest {
    #[prost(oneof = "get_identity_nonce_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_nonce_request::Version>,
}
/// Nested message and enum types in `GetIdentityNonceRequest`.
pub mod get_identity_nonce_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityNonceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityNonceRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityContractNonceRequest {
    #[prost(oneof = "get_identity_contract_nonce_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_contract_nonce_request::Version>,
}
/// Nested message and enum types in `GetIdentityContractNonceRequest`.
pub mod get_identity_contract_nonce_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityContractNonceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "2")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "3")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityContractNonceRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceRequest {
    #[prost(oneof = "get_identity_balance_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_balance_request::Version>,
}
/// Nested message and enum types in `GetIdentityBalanceRequest`.
pub mod get_identity_balance_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityBalanceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceAndRevisionRequest {
    #[prost(oneof = "get_identity_balance_and_revision_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_identity_balance_and_revision_request::Version,
    >,
}
/// Nested message and enum types in `GetIdentityBalanceAndRevisionRequest`.
pub mod get_identity_balance_and_revision_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityBalanceAndRevisionRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceAndRevisionRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityResponse {
    #[prost(oneof = "get_identity_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_response::Version>,
}
/// Nested message and enum types in `GetIdentityResponse`.
pub mod get_identity_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_identity_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_identity_response_v0::Result>,
    }
    /// Nested message and enum types in `GetIdentityResponseV0`.
    pub mod get_identity_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(bytes, tag = "1")]
            Identity(::prost::alloc::vec::Vec<u8>),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityNonceResponse {
    #[prost(oneof = "get_identity_nonce_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_nonce_response::Version>,
}
/// Nested message and enum types in `GetIdentityNonceResponse`.
pub mod get_identity_nonce_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityNonceResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_identity_nonce_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_identity_nonce_response_v0::Result>,
    }
    /// Nested message and enum types in `GetIdentityNonceResponseV0`.
    pub mod get_identity_nonce_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            IdentityNonce(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityNonceResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityContractNonceResponse {
    #[prost(oneof = "get_identity_contract_nonce_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_contract_nonce_response::Version>,
}
/// Nested message and enum types in `GetIdentityContractNonceResponse`.
pub mod get_identity_contract_nonce_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityContractNonceResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_identity_contract_nonce_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_identity_contract_nonce_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetIdentityContractNonceResponseV0`.
    pub mod get_identity_contract_nonce_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            IdentityContractNonce(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityContractNonceResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceResponse {
    #[prost(oneof = "get_identity_balance_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_balance_response::Version>,
}
/// Nested message and enum types in `GetIdentityBalanceResponse`.
pub mod get_identity_balance_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityBalanceResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_identity_balance_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_identity_balance_response_v0::Result>,
    }
    /// Nested message and enum types in `GetIdentityBalanceResponseV0`.
    pub mod get_identity_balance_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            Balance(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceAndRevisionResponse {
    #[prost(oneof = "get_identity_balance_and_revision_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_identity_balance_and_revision_response::Version,
    >,
}
/// Nested message and enum types in `GetIdentityBalanceAndRevisionResponse`.
pub mod get_identity_balance_and_revision_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityBalanceAndRevisionResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_identity_balance_and_revision_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_identity_balance_and_revision_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetIdentityBalanceAndRevisionResponseV0`.
    pub mod get_identity_balance_and_revision_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct BalanceAndRevision {
            #[prost(uint64, tag = "1")]
            pub balance: u64,
            #[prost(uint64, tag = "2")]
            pub revision: u64,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            BalanceAndRevision(BalanceAndRevision),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceAndRevisionResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyRequestType {
    #[prost(oneof = "key_request_type::Request", tags = "1, 2, 3")]
    pub request: ::core::option::Option<key_request_type::Request>,
}
/// Nested message and enum types in `KeyRequestType`.
pub mod key_request_type {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Request {
        #[prost(message, tag = "1")]
        AllKeys(super::AllKeys),
        #[prost(message, tag = "2")]
        SpecificKeys(super::SpecificKeys),
        #[prost(message, tag = "3")]
        SearchKey(super::SearchKey),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllKeys {}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpecificKeys {
    #[prost(uint32, repeated, tag = "1")]
    pub key_ids: ::prost::alloc::vec::Vec<u32>,
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SearchKey {
    #[prost(map = "uint32, message", tag = "1")]
    pub purpose_map: ::std::collections::HashMap<u32, SecurityLevelMap>,
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SecurityLevelMap {
    #[prost(
        map = "uint32, enumeration(security_level_map::KeyKindRequestType)",
        tag = "1"
    )]
    pub security_level_map: ::std::collections::HashMap<u32, i32>,
}
/// Nested message and enum types in `SecurityLevelMap`.
pub mod security_level_map {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum KeyKindRequestType {
        CurrentKeyOfKindRequest = 0,
        AllKeysOfKindRequest = 1,
    }
    impl KeyKindRequestType {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                KeyKindRequestType::CurrentKeyOfKindRequest => {
                    "CURRENT_KEY_OF_KIND_REQUEST"
                }
                KeyKindRequestType::AllKeysOfKindRequest => "ALL_KEYS_OF_KIND_REQUEST",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "CURRENT_KEY_OF_KIND_REQUEST" => Some(Self::CurrentKeyOfKindRequest),
                "ALL_KEYS_OF_KIND_REQUEST" => Some(Self::AllKeysOfKindRequest),
                _ => None,
            }
        }
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityKeysRequest {
    #[prost(oneof = "get_identity_keys_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_keys_request::Version>,
}
/// Nested message and enum types in `GetIdentityKeysRequest`.
pub mod get_identity_keys_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityKeysRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub request_type: ::core::option::Option<super::KeyRequestType>,
        #[prost(message, optional, tag = "3")]
        pub limit: ::core::option::Option<u32>,
        #[prost(message, optional, tag = "4")]
        pub offset: ::core::option::Option<u32>,
        #[prost(bool, tag = "5")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityKeysRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityKeysResponse {
    #[prost(oneof = "get_identity_keys_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identity_keys_response::Version>,
}
/// Nested message and enum types in `GetIdentityKeysResponse`.
pub mod get_identity_keys_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityKeysResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_identity_keys_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_identity_keys_response_v0::Result>,
    }
    /// Nested message and enum types in `GetIdentityKeysResponseV0`.
    pub mod get_identity_keys_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Keys {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub keys_bytes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Keys(Keys),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityKeysResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesContractKeysRequest {
    #[prost(oneof = "get_identities_contract_keys_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identities_contract_keys_request::Version>,
}
/// Nested message and enum types in `GetIdentitiesContractKeysRequest`.
pub mod get_identities_contract_keys_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentitiesContractKeysRequestV0 {
        #[prost(bytes = "vec", repeated, tag = "1")]
        pub identities_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bytes = "vec", tag = "2")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(string, optional, tag = "3")]
        pub document_type_name: ::core::option::Option<::prost::alloc::string::String>,
        #[prost(enumeration = "super::KeyPurpose", repeated, tag = "4")]
        pub purposes: ::prost::alloc::vec::Vec<i32>,
        #[prost(bool, tag = "5")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentitiesContractKeysRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesContractKeysResponse {
    #[prost(oneof = "get_identities_contract_keys_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identities_contract_keys_response::Version>,
}
/// Nested message and enum types in `GetIdentitiesContractKeysResponse`.
pub mod get_identities_contract_keys_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentitiesContractKeysResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_identities_contract_keys_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_identities_contract_keys_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetIdentitiesContractKeysResponseV0`.
    pub mod get_identities_contract_keys_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct PurposeKeys {
            #[prost(enumeration = "super::super::KeyPurpose", tag = "1")]
            pub purpose: i32,
            #[prost(bytes = "vec", repeated, tag = "2")]
            pub keys_bytes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct IdentityKeys {
            #[prost(bytes = "vec", tag = "1")]
            #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
            pub identity_id: ::prost::alloc::vec::Vec<u8>,
            #[prost(message, repeated, tag = "2")]
            pub keys: ::prost::alloc::vec::Vec<PurposeKeys>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct IdentitiesKeys {
            #[prost(message, repeated, tag = "1")]
            pub entries: ::prost::alloc::vec::Vec<IdentityKeys>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            IdentitiesKeys(IdentitiesKeys),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentitiesContractKeysResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofsRequest {
    #[prost(oneof = "get_proofs_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_proofs_request::Version>,
}
/// Nested message and enum types in `GetProofsRequest`.
pub mod get_proofs_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProofsRequestV0 {
        #[prost(message, repeated, tag = "1")]
        pub identities: ::prost::alloc::vec::Vec<get_proofs_request_v0::IdentityRequest>,
        #[prost(message, repeated, tag = "2")]
        pub contracts: ::prost::alloc::vec::Vec<get_proofs_request_v0::ContractRequest>,
        #[prost(message, repeated, tag = "3")]
        pub documents: ::prost::alloc::vec::Vec<get_proofs_request_v0::DocumentRequest>,
        #[prost(message, repeated, tag = "4")]
        pub votes: ::prost::alloc::vec::Vec<get_proofs_request_v0::VoteStatusRequest>,
    }
    /// Nested message and enum types in `GetProofsRequestV0`.
    pub mod get_proofs_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct DocumentRequest {
            #[prost(bytes = "vec", tag = "1")]
            pub contract_id: ::prost::alloc::vec::Vec<u8>,
            #[prost(string, tag = "2")]
            pub document_type: ::prost::alloc::string::String,
            #[prost(bool, tag = "3")]
            pub document_type_keeps_history: bool,
            #[prost(bytes = "vec", tag = "4")]
            pub document_id: ::prost::alloc::vec::Vec<u8>,
            #[prost(
                enumeration = "document_request::DocumentContestedStatus",
                tag = "5"
            )]
            pub document_contested_status: i32,
        }
        /// Nested message and enum types in `DocumentRequest`.
        pub mod document_request {
            #[cfg_attr(
                feature = "serde",
                derive(::serde::Serialize, ::serde::Deserialize)
            )]
            #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            #[derive(
                Clone,
                Copy,
                Debug,
                PartialEq,
                Eq,
                Hash,
                PartialOrd,
                Ord,
                ::prost::Enumeration
            )]
            #[repr(i32)]
            pub enum DocumentContestedStatus {
                NotContested = 0,
                MaybeContested = 1,
                Contested = 2,
            }
            impl DocumentContestedStatus {
                /// String value of the enum field names used in the ProtoBuf definition.
                ///
                /// The values are not transformed in any way and thus are considered stable
                /// (if the ProtoBuf definition does not change) and safe for programmatic use.
                pub fn as_str_name(&self) -> &'static str {
                    match self {
                        DocumentContestedStatus::NotContested => "NOT_CONTESTED",
                        DocumentContestedStatus::MaybeContested => "MAYBE_CONTESTED",
                        DocumentContestedStatus::Contested => "CONTESTED",
                    }
                }
                /// Creates an enum from field names used in the ProtoBuf definition.
                pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                    match value {
                        "NOT_CONTESTED" => Some(Self::NotContested),
                        "MAYBE_CONTESTED" => Some(Self::MaybeContested),
                        "CONTESTED" => Some(Self::Contested),
                        _ => None,
                    }
                }
            }
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct IdentityRequest {
            #[prost(bytes = "vec", tag = "1")]
            #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
            pub identity_id: ::prost::alloc::vec::Vec<u8>,
            #[prost(enumeration = "identity_request::Type", tag = "2")]
            pub request_type: i32,
        }
        /// Nested message and enum types in `IdentityRequest`.
        pub mod identity_request {
            #[cfg_attr(
                feature = "serde",
                derive(::serde::Serialize, ::serde::Deserialize)
            )]
            #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            #[derive(
                Clone,
                Copy,
                Debug,
                PartialEq,
                Eq,
                Hash,
                PartialOrd,
                Ord,
                ::prost::Enumeration
            )]
            #[repr(i32)]
            pub enum Type {
                FullIdentity = 0,
                Balance = 1,
                Keys = 2,
                Revision = 3,
            }
            impl Type {
                /// String value of the enum field names used in the ProtoBuf definition.
                ///
                /// The values are not transformed in any way and thus are considered stable
                /// (if the ProtoBuf definition does not change) and safe for programmatic use.
                pub fn as_str_name(&self) -> &'static str {
                    match self {
                        Type::FullIdentity => "FULL_IDENTITY",
                        Type::Balance => "BALANCE",
                        Type::Keys => "KEYS",
                        Type::Revision => "REVISION",
                    }
                }
                /// Creates an enum from field names used in the ProtoBuf definition.
                pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                    match value {
                        "FULL_IDENTITY" => Some(Self::FullIdentity),
                        "BALANCE" => Some(Self::Balance),
                        "KEYS" => Some(Self::Keys),
                        "REVISION" => Some(Self::Revision),
                        _ => None,
                    }
                }
            }
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContractRequest {
            #[prost(bytes = "vec", tag = "1")]
            pub contract_id: ::prost::alloc::vec::Vec<u8>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VoteStatusRequest {
            #[prost(oneof = "vote_status_request::RequestType", tags = "1")]
            pub request_type: ::core::option::Option<vote_status_request::RequestType>,
        }
        /// Nested message and enum types in `VoteStatusRequest`.
        pub mod vote_status_request {
            #[cfg_attr(
                feature = "serde",
                derive(::serde::Serialize, ::serde::Deserialize)
            )]
            #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            #[derive(::dapi_grpc_macros::Mockable)]
            #[allow(clippy::derive_partial_eq_without_eq)]
            #[derive(Clone, PartialEq, ::prost::Message)]
            pub struct ContestedResourceVoteStatusRequest {
                #[prost(bytes = "vec", tag = "1")]
                pub contract_id: ::prost::alloc::vec::Vec<u8>,
                #[prost(string, tag = "2")]
                pub document_type_name: ::prost::alloc::string::String,
                #[prost(string, tag = "3")]
                pub index_name: ::prost::alloc::string::String,
                #[prost(bytes = "vec", repeated, tag = "4")]
                pub index_values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
                #[prost(bytes = "vec", tag = "5")]
                pub voter_identifier: ::prost::alloc::vec::Vec<u8>,
            }
            #[cfg_attr(
                feature = "serde",
                derive(::serde::Serialize, ::serde::Deserialize)
            )]
            #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            #[allow(clippy::derive_partial_eq_without_eq)]
            #[derive(Clone, PartialEq, ::prost::Oneof)]
            pub enum RequestType {
                #[prost(message, tag = "1")]
                ContestedResourceVoteStatusRequest(ContestedResourceVoteStatusRequest),
            }
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProofsRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofsResponse {
    #[prost(oneof = "get_proofs_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_proofs_response::Version>,
}
/// Nested message and enum types in `GetProofsResponse`.
pub mod get_proofs_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProofsResponseV0 {
        #[prost(message, optional, tag = "2")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_proofs_response_v0::Result", tags = "1")]
        pub result: ::core::option::Option<get_proofs_response_v0::Result>,
    }
    /// Nested message and enum types in `GetProofsResponseV0`.
    pub mod get_proofs_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProofsResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractRequest {
    #[prost(oneof = "get_data_contract_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_data_contract_request::Version>,
}
/// Nested message and enum types in `GetDataContractRequest`.
pub mod get_data_contract_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractResponse {
    #[prost(oneof = "get_data_contract_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_data_contract_response::Version>,
}
/// Nested message and enum types in `GetDataContractResponse`.
pub mod get_data_contract_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_data_contract_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_data_contract_response_v0::Result>,
    }
    /// Nested message and enum types in `GetDataContractResponseV0`.
    pub mod get_data_contract_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(bytes, tag = "1")]
            DataContract(::prost::alloc::vec::Vec<u8>),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractsRequest {
    #[prost(oneof = "get_data_contracts_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_data_contracts_request::Version>,
}
/// Nested message and enum types in `GetDataContractsRequest`.
pub mod get_data_contracts_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractsRequestV0 {
        #[prost(bytes = "vec", repeated, tag = "1")]
        #[cfg_attr(
            feature = "serde",
            serde(with = "crate::deserialization::vec_base64string")
        )]
        pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractsRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractsResponse {
    #[prost(oneof = "get_data_contracts_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_data_contracts_response::Version>,
}
/// Nested message and enum types in `GetDataContractsResponse`.
pub mod get_data_contracts_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub identifier: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub data_contract: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContracts {
        #[prost(message, repeated, tag = "1")]
        pub data_contract_entries: ::prost::alloc::vec::Vec<DataContractEntry>,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractsResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_data_contracts_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_data_contracts_response_v0::Result>,
    }
    /// Nested message and enum types in `GetDataContractsResponseV0`.
    pub mod get_data_contracts_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            DataContracts(super::DataContracts),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractsResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractHistoryRequest {
    #[prost(oneof = "get_data_contract_history_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_data_contract_history_request::Version>,
}
/// Nested message and enum types in `GetDataContractHistoryRequest`.
pub mod get_data_contract_history_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractHistoryRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub limit: ::core::option::Option<u32>,
        #[prost(message, optional, tag = "3")]
        pub offset: ::core::option::Option<u32>,
        #[prost(uint64, tag = "4")]
        #[cfg_attr(
            feature = "serde",
            serde(with = "crate::deserialization::from_to_string")
        )]
        pub start_at_ms: u64,
        #[prost(bool, tag = "5")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractHistoryRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractHistoryResponse {
    #[prost(oneof = "get_data_contract_history_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_data_contract_history_response::Version>,
}
/// Nested message and enum types in `GetDataContractHistoryResponse`.
pub mod get_data_contract_history_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractHistoryResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_data_contract_history_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<
            get_data_contract_history_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetDataContractHistoryResponseV0`.
    pub mod get_data_contract_history_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct DataContractHistoryEntry {
            #[prost(uint64, tag = "1")]
            pub date: u64,
            #[prost(bytes = "vec", tag = "2")]
            pub value: ::prost::alloc::vec::Vec<u8>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct DataContractHistory {
            #[prost(message, repeated, tag = "1")]
            pub data_contract_entries: ::prost::alloc::vec::Vec<
                DataContractHistoryEntry,
            >,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            DataContractHistory(DataContractHistory),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractHistoryResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDocumentsRequest {
    #[prost(oneof = "get_documents_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_documents_request::Version>,
}
/// Nested message and enum types in `GetDocumentsRequest`.
pub mod get_documents_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDocumentsRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub data_contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(string, tag = "2")]
        pub document_type: ::prost::alloc::string::String,
        #[prost(bytes = "vec", tag = "3")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub r#where: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "4")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub order_by: ::prost::alloc::vec::Vec<u8>,
        #[prost(uint32, tag = "5")]
        pub limit: u32,
        #[prost(bool, tag = "8")]
        pub prove: bool,
        #[prost(oneof = "get_documents_request_v0::Start", tags = "6, 7")]
        pub start: ::core::option::Option<get_documents_request_v0::Start>,
    }
    /// Nested message and enum types in `GetDocumentsRequestV0`.
    pub mod get_documents_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Start {
            #[prost(bytes, tag = "6")]
            StartAfter(::prost::alloc::vec::Vec<u8>),
            #[prost(bytes, tag = "7")]
            StartAt(::prost::alloc::vec::Vec<u8>),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDocumentsRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDocumentsResponse {
    #[prost(oneof = "get_documents_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_documents_response::Version>,
}
/// Nested message and enum types in `GetDocumentsResponse`.
pub mod get_documents_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDocumentsResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_documents_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_documents_response_v0::Result>,
    }
    /// Nested message and enum types in `GetDocumentsResponseV0`.
    pub mod get_documents_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Documents {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub documents: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Documents(Documents),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDocumentsResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityByPublicKeyHashRequest {
    #[prost(oneof = "get_identity_by_public_key_hash_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_identity_by_public_key_hash_request::Version,
    >,
}
/// Nested message and enum types in `GetIdentityByPublicKeyHashRequest`.
pub mod get_identity_by_public_key_hash_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityByPublicKeyHashRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub public_key_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityByPublicKeyHashRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityByPublicKeyHashResponse {
    #[prost(oneof = "get_identity_by_public_key_hash_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_identity_by_public_key_hash_response::Version,
    >,
}
/// Nested message and enum types in `GetIdentityByPublicKeyHashResponse`.
pub mod get_identity_by_public_key_hash_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityByPublicKeyHashResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_identity_by_public_key_hash_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_identity_by_public_key_hash_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetIdentityByPublicKeyHashResponseV0`.
    pub mod get_identity_by_public_key_hash_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(bytes, tag = "1")]
            Identity(::prost::alloc::vec::Vec<u8>),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityByPublicKeyHashResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitForStateTransitionResultRequest {
    #[prost(oneof = "wait_for_state_transition_result_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        wait_for_state_transition_result_request::Version,
    >,
}
/// Nested message and enum types in `WaitForStateTransitionResultRequest`.
pub mod wait_for_state_transition_result_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct WaitForStateTransitionResultRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        pub state_transition_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(WaitForStateTransitionResultRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitForStateTransitionResultResponse {
    #[prost(oneof = "wait_for_state_transition_result_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        wait_for_state_transition_result_response::Version,
    >,
}
/// Nested message and enum types in `WaitForStateTransitionResultResponse`.
pub mod wait_for_state_transition_result_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct WaitForStateTransitionResultResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "wait_for_state_transition_result_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            wait_for_state_transition_result_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `WaitForStateTransitionResultResponseV0`.
    pub mod wait_for_state_transition_result_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Error(super::super::StateTransitionBroadcastError),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(WaitForStateTransitionResultResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsRequest {
    #[prost(oneof = "get_consensus_params_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_consensus_params_request::Version>,
}
/// Nested message and enum types in `GetConsensusParamsRequest`.
pub mod get_consensus_params_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetConsensusParamsRequestV0 {
        #[prost(int32, tag = "1")]
        pub height: i32,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetConsensusParamsRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsResponse {
    #[prost(oneof = "get_consensus_params_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_consensus_params_response::Version>,
}
/// Nested message and enum types in `GetConsensusParamsResponse`.
pub mod get_consensus_params_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ConsensusParamsBlock {
        #[prost(string, tag = "1")]
        pub max_bytes: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub max_gas: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub time_iota_ms: ::prost::alloc::string::String,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ConsensusParamsEvidence {
        #[prost(string, tag = "1")]
        pub max_age_num_blocks: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub max_age_duration: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub max_bytes: ::prost::alloc::string::String,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetConsensusParamsResponseV0 {
        #[prost(message, optional, tag = "1")]
        pub block: ::core::option::Option<ConsensusParamsBlock>,
        #[prost(message, optional, tag = "2")]
        pub evidence: ::core::option::Option<ConsensusParamsEvidence>,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetConsensusParamsResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProtocolVersionUpgradeStateRequest {
    #[prost(oneof = "get_protocol_version_upgrade_state_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_protocol_version_upgrade_state_request::Version,
    >,
}
/// Nested message and enum types in `GetProtocolVersionUpgradeStateRequest`.
pub mod get_protocol_version_upgrade_state_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProtocolVersionUpgradeStateRequestV0 {
        #[prost(bool, tag = "1")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeStateRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProtocolVersionUpgradeStateResponse {
    #[prost(oneof = "get_protocol_version_upgrade_state_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_protocol_version_upgrade_state_response::Version,
    >,
}
/// Nested message and enum types in `GetProtocolVersionUpgradeStateResponse`.
pub mod get_protocol_version_upgrade_state_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProtocolVersionUpgradeStateResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_protocol_version_upgrade_state_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_protocol_version_upgrade_state_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetProtocolVersionUpgradeStateResponseV0`.
    pub mod get_protocol_version_upgrade_state_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Versions {
            #[prost(message, repeated, tag = "1")]
            pub versions: ::prost::alloc::vec::Vec<VersionEntry>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VersionEntry {
            #[prost(uint32, tag = "1")]
            pub version_number: u32,
            #[prost(uint32, tag = "2")]
            pub vote_count: u32,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Versions(Versions),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeStateResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProtocolVersionUpgradeVoteStatusRequest {
    #[prost(
        oneof = "get_protocol_version_upgrade_vote_status_request::Version",
        tags = "1"
    )]
    pub version: ::core::option::Option<
        get_protocol_version_upgrade_vote_status_request::Version,
    >,
}
/// Nested message and enum types in `GetProtocolVersionUpgradeVoteStatusRequest`.
pub mod get_protocol_version_upgrade_vote_status_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProtocolVersionUpgradeVoteStatusRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        pub start_pro_tx_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(uint32, tag = "2")]
        pub count: u32,
        #[prost(bool, tag = "3")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeVoteStatusRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProtocolVersionUpgradeVoteStatusResponse {
    #[prost(
        oneof = "get_protocol_version_upgrade_vote_status_response::Version",
        tags = "1"
    )]
    pub version: ::core::option::Option<
        get_protocol_version_upgrade_vote_status_response::Version,
    >,
}
/// Nested message and enum types in `GetProtocolVersionUpgradeVoteStatusResponse`.
pub mod get_protocol_version_upgrade_vote_status_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProtocolVersionUpgradeVoteStatusResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_protocol_version_upgrade_vote_status_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_protocol_version_upgrade_vote_status_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetProtocolVersionUpgradeVoteStatusResponseV0`.
    pub mod get_protocol_version_upgrade_vote_status_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VersionSignals {
            #[prost(message, repeated, tag = "1")]
            pub version_signals: ::prost::alloc::vec::Vec<VersionSignal>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VersionSignal {
            #[prost(bytes = "vec", tag = "1")]
            pub pro_tx_hash: ::prost::alloc::vec::Vec<u8>,
            #[prost(uint32, tag = "2")]
            pub version: u32,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Versions(VersionSignals),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeVoteStatusResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEpochsInfoRequest {
    #[prost(oneof = "get_epochs_info_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_epochs_info_request::Version>,
}
/// Nested message and enum types in `GetEpochsInfoRequest`.
pub mod get_epochs_info_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetEpochsInfoRequestV0 {
        #[prost(message, optional, tag = "1")]
        pub start_epoch: ::core::option::Option<u32>,
        #[prost(uint32, tag = "2")]
        pub count: u32,
        #[prost(bool, tag = "3")]
        pub ascending: bool,
        #[prost(bool, tag = "4")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetEpochsInfoRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEpochsInfoResponse {
    #[prost(oneof = "get_epochs_info_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_epochs_info_response::Version>,
}
/// Nested message and enum types in `GetEpochsInfoResponse`.
pub mod get_epochs_info_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetEpochsInfoResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_epochs_info_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_epochs_info_response_v0::Result>,
    }
    /// Nested message and enum types in `GetEpochsInfoResponseV0`.
    pub mod get_epochs_info_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct EpochInfos {
            #[prost(message, repeated, tag = "1")]
            pub epoch_infos: ::prost::alloc::vec::Vec<EpochInfo>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct EpochInfo {
            #[prost(uint32, tag = "1")]
            pub number: u32,
            #[prost(uint64, tag = "2")]
            pub first_block_height: u64,
            #[prost(uint32, tag = "3")]
            pub first_core_block_height: u32,
            #[prost(uint64, tag = "4")]
            pub start_time: u64,
            #[prost(double, tag = "5")]
            pub fee_multiplier: f64,
            #[prost(uint32, tag = "6")]
            pub protocol_version: u32,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Epochs(EpochInfos),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetEpochsInfoResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourcesRequest {
    #[prost(oneof = "get_contested_resources_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_contested_resources_request::Version>,
}
/// Nested message and enum types in `GetContestedResourcesRequest`.
pub mod get_contested_resources_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourcesRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(string, tag = "2")]
        pub document_type_name: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub index_name: ::prost::alloc::string::String,
        #[prost(bytes = "vec", repeated, tag = "4")]
        pub start_index_values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bytes = "vec", repeated, tag = "5")]
        pub end_index_values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(message, optional, tag = "6")]
        pub start_at_value_info: ::core::option::Option<
            get_contested_resources_request_v0::StartAtValueInfo,
        >,
        #[prost(uint32, optional, tag = "7")]
        pub count: ::core::option::Option<u32>,
        #[prost(bool, tag = "8")]
        pub order_ascending: bool,
        #[prost(bool, tag = "9")]
        pub prove: bool,
    }
    /// Nested message and enum types in `GetContestedResourcesRequestV0`.
    pub mod get_contested_resources_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StartAtValueInfo {
            #[prost(bytes = "vec", tag = "1")]
            pub start_value: ::prost::alloc::vec::Vec<u8>,
            #[prost(bool, tag = "2")]
            pub start_value_included: bool,
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourcesRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourcesResponse {
    #[prost(oneof = "get_contested_resources_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_contested_resources_response::Version>,
}
/// Nested message and enum types in `GetContestedResourcesResponse`.
pub mod get_contested_resources_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourcesResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_contested_resources_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_contested_resources_response_v0::Result>,
    }
    /// Nested message and enum types in `GetContestedResourcesResponseV0`.
    pub mod get_contested_resources_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContestedResourceValues {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub contested_resource_values: ::prost::alloc::vec::Vec<
                ::prost::alloc::vec::Vec<u8>,
            >,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            ContestedResourceValues(ContestedResourceValues),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourcesResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetVotePollsByEndDateRequest {
    #[prost(oneof = "get_vote_polls_by_end_date_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_vote_polls_by_end_date_request::Version>,
}
/// Nested message and enum types in `GetVotePollsByEndDateRequest`.
pub mod get_vote_polls_by_end_date_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetVotePollsByEndDateRequestV0 {
        #[prost(message, optional, tag = "1")]
        pub start_time_info: ::core::option::Option<
            get_vote_polls_by_end_date_request_v0::StartAtTimeInfo,
        >,
        #[prost(message, optional, tag = "2")]
        pub end_time_info: ::core::option::Option<
            get_vote_polls_by_end_date_request_v0::EndAtTimeInfo,
        >,
        #[prost(uint32, optional, tag = "3")]
        pub limit: ::core::option::Option<u32>,
        #[prost(uint32, optional, tag = "4")]
        pub offset: ::core::option::Option<u32>,
        #[prost(bool, tag = "5")]
        pub ascending: bool,
        #[prost(bool, tag = "6")]
        pub prove: bool,
    }
    /// Nested message and enum types in `GetVotePollsByEndDateRequestV0`.
    pub mod get_vote_polls_by_end_date_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StartAtTimeInfo {
            #[prost(uint64, tag = "1")]
            pub start_time_ms: u64,
            #[prost(bool, tag = "2")]
            pub start_time_included: bool,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct EndAtTimeInfo {
            #[prost(uint64, tag = "1")]
            pub end_time_ms: u64,
            #[prost(bool, tag = "2")]
            pub end_time_included: bool,
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetVotePollsByEndDateRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetVotePollsByEndDateResponse {
    #[prost(oneof = "get_vote_polls_by_end_date_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_vote_polls_by_end_date_response::Version>,
}
/// Nested message and enum types in `GetVotePollsByEndDateResponse`.
pub mod get_vote_polls_by_end_date_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetVotePollsByEndDateResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_vote_polls_by_end_date_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<
            get_vote_polls_by_end_date_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetVotePollsByEndDateResponseV0`.
    pub mod get_vote_polls_by_end_date_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct SerializedVotePollsByTimestamp {
            #[prost(uint64, tag = "1")]
            pub timestamp: u64,
            #[prost(bytes = "vec", repeated, tag = "2")]
            pub serialized_vote_polls: ::prost::alloc::vec::Vec<
                ::prost::alloc::vec::Vec<u8>,
            >,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct SerializedVotePollsByTimestamps {
            #[prost(message, repeated, tag = "1")]
            pub vote_polls_by_timestamps: ::prost::alloc::vec::Vec<
                SerializedVotePollsByTimestamp,
            >,
            #[prost(bool, tag = "2")]
            pub finished_results: bool,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            VotePollsByTimestamps(SerializedVotePollsByTimestamps),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetVotePollsByEndDateResponseV0),
    }
}
/// What's the state of a contested resource vote? (ie who is winning?)
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourceVoteStateRequest {
    #[prost(oneof = "get_contested_resource_vote_state_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_contested_resource_vote_state_request::Version,
    >,
}
/// Nested message and enum types in `GetContestedResourceVoteStateRequest`.
pub mod get_contested_resource_vote_state_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourceVoteStateRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(string, tag = "2")]
        pub document_type_name: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub index_name: ::prost::alloc::string::String,
        #[prost(bytes = "vec", repeated, tag = "4")]
        pub index_values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(
            enumeration = "get_contested_resource_vote_state_request_v0::ResultType",
            tag = "5"
        )]
        pub result_type: i32,
        #[prost(bool, tag = "6")]
        pub allow_include_locked_and_abstaining_vote_tally: bool,
        #[prost(message, optional, tag = "7")]
        pub start_at_identifier_info: ::core::option::Option<
            get_contested_resource_vote_state_request_v0::StartAtIdentifierInfo,
        >,
        #[prost(uint32, optional, tag = "8")]
        pub count: ::core::option::Option<u32>,
        #[prost(bool, tag = "9")]
        pub prove: bool,
    }
    /// Nested message and enum types in `GetContestedResourceVoteStateRequestV0`.
    pub mod get_contested_resource_vote_state_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StartAtIdentifierInfo {
            #[prost(bytes = "vec", tag = "1")]
            pub start_identifier: ::prost::alloc::vec::Vec<u8>,
            #[prost(bool, tag = "2")]
            pub start_identifier_included: bool,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(
            Clone,
            Copy,
            Debug,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::prost::Enumeration
        )]
        #[repr(i32)]
        pub enum ResultType {
            Documents = 0,
            VoteTally = 1,
            DocumentsAndVoteTally = 2,
        }
        impl ResultType {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    ResultType::Documents => "DOCUMENTS",
                    ResultType::VoteTally => "VOTE_TALLY",
                    ResultType::DocumentsAndVoteTally => "DOCUMENTS_AND_VOTE_TALLY",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "DOCUMENTS" => Some(Self::Documents),
                    "VOTE_TALLY" => Some(Self::VoteTally),
                    "DOCUMENTS_AND_VOTE_TALLY" => Some(Self::DocumentsAndVoteTally),
                    _ => None,
                }
            }
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourceVoteStateRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourceVoteStateResponse {
    #[prost(oneof = "get_contested_resource_vote_state_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_contested_resource_vote_state_response::Version,
    >,
}
/// Nested message and enum types in `GetContestedResourceVoteStateResponse`.
pub mod get_contested_resource_vote_state_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourceVoteStateResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_contested_resource_vote_state_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_contested_resource_vote_state_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetContestedResourceVoteStateResponseV0`.
    pub mod get_contested_resource_vote_state_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct FinishedVoteInfo {
            #[prost(enumeration = "finished_vote_info::FinishedVoteOutcome", tag = "1")]
            pub finished_vote_outcome: i32,
            /// Only used when vote_choice_type is TOWARDS_IDENTITY
            #[prost(bytes = "vec", optional, tag = "2")]
            pub won_by_identity_id: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
            #[prost(uint64, tag = "3")]
            pub finished_at_block_height: u64,
            #[prost(uint32, tag = "4")]
            pub finished_at_core_block_height: u32,
            #[prost(uint64, tag = "5")]
            pub finished_at_block_time_ms: u64,
            #[prost(uint32, tag = "6")]
            pub finished_at_epoch: u32,
        }
        /// Nested message and enum types in `FinishedVoteInfo`.
        pub mod finished_vote_info {
            #[cfg_attr(
                feature = "serde",
                derive(::serde::Serialize, ::serde::Deserialize)
            )]
            #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            #[derive(
                Clone,
                Copy,
                Debug,
                PartialEq,
                Eq,
                Hash,
                PartialOrd,
                Ord,
                ::prost::Enumeration
            )]
            #[repr(i32)]
            pub enum FinishedVoteOutcome {
                TowardsIdentity = 0,
                Locked = 1,
                NoPreviousWinner = 2,
            }
            impl FinishedVoteOutcome {
                /// String value of the enum field names used in the ProtoBuf definition.
                ///
                /// The values are not transformed in any way and thus are considered stable
                /// (if the ProtoBuf definition does not change) and safe for programmatic use.
                pub fn as_str_name(&self) -> &'static str {
                    match self {
                        FinishedVoteOutcome::TowardsIdentity => "TOWARDS_IDENTITY",
                        FinishedVoteOutcome::Locked => "LOCKED",
                        FinishedVoteOutcome::NoPreviousWinner => "NO_PREVIOUS_WINNER",
                    }
                }
                /// Creates an enum from field names used in the ProtoBuf definition.
                pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                    match value {
                        "TOWARDS_IDENTITY" => Some(Self::TowardsIdentity),
                        "LOCKED" => Some(Self::Locked),
                        "NO_PREVIOUS_WINNER" => Some(Self::NoPreviousWinner),
                        _ => None,
                    }
                }
            }
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContestedResourceContenders {
            #[prost(message, repeated, tag = "1")]
            pub contenders: ::prost::alloc::vec::Vec<Contender>,
            #[prost(uint32, optional, tag = "2")]
            pub abstain_vote_tally: ::core::option::Option<u32>,
            #[prost(uint32, optional, tag = "3")]
            pub lock_vote_tally: ::core::option::Option<u32>,
            #[prost(message, optional, tag = "4")]
            pub finished_vote_info: ::core::option::Option<FinishedVoteInfo>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Contender {
            #[prost(bytes = "vec", tag = "1")]
            pub identifier: ::prost::alloc::vec::Vec<u8>,
            #[prost(uint32, optional, tag = "2")]
            pub vote_count: ::core::option::Option<u32>,
            #[prost(bytes = "vec", optional, tag = "3")]
            pub document: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            ContestedResourceContenders(ContestedResourceContenders),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourceVoteStateResponseV0),
    }
}
/// Who voted for a contested resource to go to a specific identity?
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourceVotersForIdentityRequest {
    #[prost(
        oneof = "get_contested_resource_voters_for_identity_request::Version",
        tags = "1"
    )]
    pub version: ::core::option::Option<
        get_contested_resource_voters_for_identity_request::Version,
    >,
}
/// Nested message and enum types in `GetContestedResourceVotersForIdentityRequest`.
pub mod get_contested_resource_voters_for_identity_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourceVotersForIdentityRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(string, tag = "2")]
        pub document_type_name: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub index_name: ::prost::alloc::string::String,
        #[prost(bytes = "vec", repeated, tag = "4")]
        pub index_values: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bytes = "vec", tag = "5")]
        pub contestant_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "6")]
        pub start_at_identifier_info: ::core::option::Option<
            get_contested_resource_voters_for_identity_request_v0::StartAtIdentifierInfo,
        >,
        #[prost(uint32, optional, tag = "7")]
        pub count: ::core::option::Option<u32>,
        #[prost(bool, tag = "8")]
        pub order_ascending: bool,
        #[prost(bool, tag = "9")]
        pub prove: bool,
    }
    /// Nested message and enum types in `GetContestedResourceVotersForIdentityRequestV0`.
    pub mod get_contested_resource_voters_for_identity_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StartAtIdentifierInfo {
            #[prost(bytes = "vec", tag = "1")]
            pub start_identifier: ::prost::alloc::vec::Vec<u8>,
            #[prost(bool, tag = "2")]
            pub start_identifier_included: bool,
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourceVotersForIdentityRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourceVotersForIdentityResponse {
    #[prost(
        oneof = "get_contested_resource_voters_for_identity_response::Version",
        tags = "1"
    )]
    pub version: ::core::option::Option<
        get_contested_resource_voters_for_identity_response::Version,
    >,
}
/// Nested message and enum types in `GetContestedResourceVotersForIdentityResponse`.
pub mod get_contested_resource_voters_for_identity_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourceVotersForIdentityResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_contested_resource_voters_for_identity_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_contested_resource_voters_for_identity_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetContestedResourceVotersForIdentityResponseV0`.
    pub mod get_contested_resource_voters_for_identity_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContestedResourceVoters {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub voters: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
            #[prost(bool, tag = "2")]
            pub finished_results: bool,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            ContestedResourceVoters(ContestedResourceVoters),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourceVotersForIdentityResponseV0),
    }
}
/// How did an identity vote?
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourceIdentityVotesRequest {
    #[prost(
        oneof = "get_contested_resource_identity_votes_request::Version",
        tags = "1"
    )]
    pub version: ::core::option::Option<
        get_contested_resource_identity_votes_request::Version,
    >,
}
/// Nested message and enum types in `GetContestedResourceIdentityVotesRequest`.
pub mod get_contested_resource_identity_votes_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourceIdentityVotesRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub limit: ::core::option::Option<u32>,
        #[prost(message, optional, tag = "3")]
        pub offset: ::core::option::Option<u32>,
        #[prost(bool, tag = "4")]
        pub order_ascending: bool,
        #[prost(message, optional, tag = "5")]
        pub start_at_vote_poll_id_info: ::core::option::Option<
            get_contested_resource_identity_votes_request_v0::StartAtVotePollIdInfo,
        >,
        #[prost(bool, tag = "6")]
        pub prove: bool,
    }
    /// Nested message and enum types in `GetContestedResourceIdentityVotesRequestV0`.
    pub mod get_contested_resource_identity_votes_request_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct StartAtVotePollIdInfo {
            #[prost(bytes = "vec", tag = "1")]
            pub start_at_poll_identifier: ::prost::alloc::vec::Vec<u8>,
            #[prost(bool, tag = "2")]
            pub start_poll_identifier_included: bool,
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourceIdentityVotesRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetContestedResourceIdentityVotesResponse {
    #[prost(
        oneof = "get_contested_resource_identity_votes_response::Version",
        tags = "1"
    )]
    pub version: ::core::option::Option<
        get_contested_resource_identity_votes_response::Version,
    >,
}
/// Nested message and enum types in `GetContestedResourceIdentityVotesResponse`.
pub mod get_contested_resource_identity_votes_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetContestedResourceIdentityVotesResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_contested_resource_identity_votes_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_contested_resource_identity_votes_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetContestedResourceIdentityVotesResponseV0`.
    pub mod get_contested_resource_identity_votes_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContestedResourceIdentityVotes {
            #[prost(message, repeated, tag = "1")]
            pub contested_resource_identity_votes: ::prost::alloc::vec::Vec<
                ContestedResourceIdentityVote,
            >,
            #[prost(bool, tag = "2")]
            pub finished_results: bool,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ResourceVoteChoice {
            #[prost(enumeration = "resource_vote_choice::VoteChoiceType", tag = "1")]
            pub vote_choice_type: i32,
            /// Only used when vote_choice_type is TOWARDS_IDENTITY
            #[prost(bytes = "vec", optional, tag = "2")]
            #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
            pub identity_id: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
        }
        /// Nested message and enum types in `ResourceVoteChoice`.
        pub mod resource_vote_choice {
            #[cfg_attr(
                feature = "serde",
                derive(::serde::Serialize, ::serde::Deserialize)
            )]
            #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
            #[derive(
                Clone,
                Copy,
                Debug,
                PartialEq,
                Eq,
                Hash,
                PartialOrd,
                Ord,
                ::prost::Enumeration
            )]
            #[repr(i32)]
            pub enum VoteChoiceType {
                TowardsIdentity = 0,
                Abstain = 1,
                Lock = 2,
            }
            impl VoteChoiceType {
                /// String value of the enum field names used in the ProtoBuf definition.
                ///
                /// The values are not transformed in any way and thus are considered stable
                /// (if the ProtoBuf definition does not change) and safe for programmatic use.
                pub fn as_str_name(&self) -> &'static str {
                    match self {
                        VoteChoiceType::TowardsIdentity => "TOWARDS_IDENTITY",
                        VoteChoiceType::Abstain => "ABSTAIN",
                        VoteChoiceType::Lock => "LOCK",
                    }
                }
                /// Creates an enum from field names used in the ProtoBuf definition.
                pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                    match value {
                        "TOWARDS_IDENTITY" => Some(Self::TowardsIdentity),
                        "ABSTAIN" => Some(Self::Abstain),
                        "LOCK" => Some(Self::Lock),
                        _ => None,
                    }
                }
            }
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContestedResourceIdentityVote {
            #[prost(bytes = "vec", tag = "1")]
            pub contract_id: ::prost::alloc::vec::Vec<u8>,
            #[prost(string, tag = "2")]
            pub document_type_name: ::prost::alloc::string::String,
            #[prost(bytes = "vec", repeated, tag = "3")]
            pub serialized_index_storage_values: ::prost::alloc::vec::Vec<
                ::prost::alloc::vec::Vec<u8>,
            >,
            #[prost(message, optional, tag = "4")]
            pub vote_choice: ::core::option::Option<ResourceVoteChoice>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Votes(ContestedResourceIdentityVotes),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetContestedResourceIdentityVotesResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPrefundedSpecializedBalanceRequest {
    #[prost(oneof = "get_prefunded_specialized_balance_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_prefunded_specialized_balance_request::Version,
    >,
}
/// Nested message and enum types in `GetPrefundedSpecializedBalanceRequest`.
pub mod get_prefunded_specialized_balance_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetPrefundedSpecializedBalanceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[cfg_attr(feature = "serde", serde(with = "serde_bytes"))]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetPrefundedSpecializedBalanceRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPrefundedSpecializedBalanceResponse {
    #[prost(oneof = "get_prefunded_specialized_balance_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_prefunded_specialized_balance_response::Version,
    >,
}
/// Nested message and enum types in `GetPrefundedSpecializedBalanceResponse`.
pub mod get_prefunded_specialized_balance_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetPrefundedSpecializedBalanceResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_prefunded_specialized_balance_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_prefunded_specialized_balance_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetPrefundedSpecializedBalanceResponseV0`.
    pub mod get_prefunded_specialized_balance_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            Balance(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetPrefundedSpecializedBalanceResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPathElementsRequest {
    #[prost(oneof = "get_path_elements_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_path_elements_request::Version>,
}
/// Nested message and enum types in `GetPathElementsRequest`.
pub mod get_path_elements_request {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetPathElementsRequestV0 {
        #[prost(bytes = "vec", repeated, tag = "1")]
        pub path: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bytes = "vec", repeated, tag = "2")]
        pub keys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bool, tag = "3")]
        pub prove: bool,
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetPathElementsRequestV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetPathElementsResponse {
    #[prost(oneof = "get_path_elements_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_path_elements_response::Version>,
}
/// Nested message and enum types in `GetPathElementsResponse`.
pub mod get_path_elements_response {
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetPathElementsResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_path_elements_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_path_elements_response_v0::Result>,
    }
    /// Nested message and enum types in `GetPathElementsResponseV0`.
    pub mod get_path_elements_response_v0 {
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Elements {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub elements: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        }
        #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Elements(Elements),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetPathElementsResponseV0),
    }
}
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum KeyPurpose {
    Authentication = 0,
    Encryption = 1,
    Decryption = 2,
    Transfer = 3,
    Voting = 5,
}
impl KeyPurpose {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            KeyPurpose::Authentication => "AUTHENTICATION",
            KeyPurpose::Encryption => "ENCRYPTION",
            KeyPurpose::Decryption => "DECRYPTION",
            KeyPurpose::Transfer => "TRANSFER",
            KeyPurpose::Voting => "VOTING",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "AUTHENTICATION" => Some(Self::Authentication),
            "ENCRYPTION" => Some(Self::Encryption),
            "DECRYPTION" => Some(Self::Decryption),
            "TRANSFER" => Some(Self::Transfer),
            "VOTING" => Some(Self::Voting),
            _ => None,
        }
    }
}
/// Generated client implementations.
pub mod platform_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct PlatformClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl PlatformClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> PlatformClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> PlatformClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            PlatformClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn broadcast_state_transition(
            &mut self,
            request: impl tonic::IntoRequest<super::BroadcastStateTransitionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::BroadcastStateTransitionResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/broadcastStateTransition",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "broadcastStateTransition",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentity",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("org.dash.platform.dapi.v0.Platform", "getIdentity"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity_keys(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityKeysRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityKeysResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityKeys",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityKeys",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identities_contract_keys(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentitiesContractKeysRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentitiesContractKeysResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentitiesContractKeys",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentitiesContractKeys",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity_nonce(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityNonceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityNonceResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityNonce",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityNonce",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity_contract_nonce(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityContractNonceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityContractNonceResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityContractNonce",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityContractNonce",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity_balance(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityBalanceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityBalanceResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityBalance",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityBalance",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity_balance_and_revision(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityBalanceAndRevisionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityBalanceAndRevisionResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityBalanceAndRevision",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityBalanceAndRevision",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_proofs(
            &mut self,
            request: impl tonic::IntoRequest<super::GetProofsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetProofsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getProofs",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("org.dash.platform.dapi.v0.Platform", "getProofs"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_data_contract(
            &mut self,
            request: impl tonic::IntoRequest<super::GetDataContractRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDataContractResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getDataContract",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getDataContract",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_data_contract_history(
            &mut self,
            request: impl tonic::IntoRequest<super::GetDataContractHistoryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDataContractHistoryResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getDataContractHistory",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getDataContractHistory",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_data_contracts(
            &mut self,
            request: impl tonic::IntoRequest<super::GetDataContractsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDataContractsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getDataContracts",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getDataContracts",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_documents(
            &mut self,
            request: impl tonic::IntoRequest<super::GetDocumentsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDocumentsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getDocuments",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("org.dash.platform.dapi.v0.Platform", "getDocuments"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_identity_by_public_key_hash(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityByPublicKeyHashRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityByPublicKeyHashResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getIdentityByPublicKeyHash",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityByPublicKeyHash",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn wait_for_state_transition_result(
            &mut self,
            request: impl tonic::IntoRequest<super::WaitForStateTransitionResultRequest>,
        ) -> std::result::Result<
            tonic::Response<super::WaitForStateTransitionResultResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "waitForStateTransitionResult",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_consensus_params(
            &mut self,
            request: impl tonic::IntoRequest<super::GetConsensusParamsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetConsensusParamsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getConsensusParams",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getConsensusParams",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_protocol_version_upgrade_state(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetProtocolVersionUpgradeStateRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::GetProtocolVersionUpgradeStateResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getProtocolVersionUpgradeState",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getProtocolVersionUpgradeState",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_protocol_version_upgrade_vote_status(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetProtocolVersionUpgradeVoteStatusRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::GetProtocolVersionUpgradeVoteStatusResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getProtocolVersionUpgradeVoteStatus",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getProtocolVersionUpgradeVoteStatus",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_epochs_info(
            &mut self,
            request: impl tonic::IntoRequest<super::GetEpochsInfoRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetEpochsInfoResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getEpochsInfo",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getEpochsInfo",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// What votes are currently happening for a specific contested index
        pub async fn get_contested_resources(
            &mut self,
            request: impl tonic::IntoRequest<super::GetContestedResourcesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetContestedResourcesResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getContestedResources",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getContestedResources",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// What's the state of a contested resource vote? (ie who is winning?)
        pub async fn get_contested_resource_vote_state(
            &mut self,
            request: impl tonic::IntoRequest<super::GetContestedResourceVoteStateRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetContestedResourceVoteStateResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getContestedResourceVoteState",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getContestedResourceVoteState",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// Who voted for a contested resource to go to a specific identity?
        pub async fn get_contested_resource_voters_for_identity(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetContestedResourceVotersForIdentityRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::GetContestedResourceVotersForIdentityResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getContestedResourceVotersForIdentity",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getContestedResourceVotersForIdentity",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// How did an identity vote?
        pub async fn get_contested_resource_identity_votes(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetContestedResourceIdentityVotesRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::GetContestedResourceIdentityVotesResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getContestedResourceIdentityVotes",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getContestedResourceIdentityVotes",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        /// What vote polls will end soon?
        pub async fn get_vote_polls_by_end_date(
            &mut self,
            request: impl tonic::IntoRequest<super::GetVotePollsByEndDateRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetVotePollsByEndDateResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getVotePollsByEndDate",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getVotePollsByEndDate",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_prefunded_specialized_balance(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetPrefundedSpecializedBalanceRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::GetPrefundedSpecializedBalanceResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getPrefundedSpecializedBalance",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getPrefundedSpecializedBalance",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_path_elements(
            &mut self,
            request: impl tonic::IntoRequest<super::GetPathElementsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetPathElementsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/org.dash.platform.dapi.v0.Platform/getPathElements",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getPathElements",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
    }
}
