#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Proof {
    #[prost(bytes = "vec", tag = "1")]
    #[serde(with = "serde_bytes")]
    pub grovedb_proof: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    #[serde(with = "serde_bytes")]
    pub quorum_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    #[serde(with = "serde_bytes")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "4")]
    pub round: u32,
    #[prost(bytes = "vec", tag = "5")]
    #[serde(with = "serde_bytes")]
    pub block_id_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "6")]
    pub quorum_type: u32,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResponseMetadata {
    #[prost(uint64, tag = "1")]
    #[serde(with = "crate::deserialization::from_to_string")]
    pub height: u64,
    #[prost(uint32, tag = "2")]
    pub core_chain_locked_height: u32,
    #[prost(uint32, tag = "3")]
    pub epoch: u32,
    #[prost(uint64, tag = "4")]
    #[serde(with = "crate::deserialization::from_to_string")]
    pub time_ms: u64,
    #[prost(uint32, tag = "5")]
    pub protocol_version: u32,
    #[prost(string, tag = "6")]
    pub chain_id: ::prost::alloc::string::String,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub state_transition: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionResponse {}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityNonceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityNonceRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityContractNonceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "2")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "3")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityContractNonceRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityBalanceRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityBalanceAndRevisionRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceAndRevisionRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(bytes, tag = "1")]
            Identity(::prost::alloc::vec::Vec<u8>),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesRequest {
    #[prost(oneof = "get_identities_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_identities_request::Version>,
}
/// Nested message and enum types in `GetIdentitiesRequest`.
pub mod get_identities_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentitiesRequestV0 {
        #[prost(bytes = "vec", repeated, tag = "1")]
        #[serde(with = "crate::deserialization::vec_base64string")]
        pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentitiesRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesResponse {
    #[prost(oneof = "get_identities_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_identities_response::Version>,
}
/// Nested message and enum types in `GetIdentitiesResponse`.
pub mod get_identities_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentityValue {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentityEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<IdentityValue>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Identities {
        #[prost(message, repeated, tag = "1")]
        pub identity_entries: ::prost::alloc::vec::Vec<IdentityEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentitiesResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(oneof = "get_identities_response_v0::Result", tags = "1, 2")]
        pub result: ::core::option::Option<get_identities_response_v0::Result>,
    }
    /// Nested message and enum types in `GetIdentitiesResponseV0`.
    pub mod get_identities_response_v0 {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Identities(super::Identities),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentitiesResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            IdentityNonce(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityNonceResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            IdentityContractNonce(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityContractNonceResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(uint64, tag = "1")]
            Balance(u64),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct BalanceAndRevision {
            #[prost(uint64, tag = "1")]
            pub balance: u64,
            #[prost(uint64, tag = "2")]
            pub revision: u64,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            BalanceAndRevision(BalanceAndRevision),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityBalanceAndRevisionResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyRequestType {
    #[prost(oneof = "key_request_type::Request", tags = "1, 2, 3")]
    pub request: ::core::option::Option<key_request_type::Request>,
}
/// Nested message and enum types in `KeyRequestType`.
pub mod key_request_type {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllKeys {}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpecificKeys {
    #[prost(uint32, repeated, tag = "1")]
    pub key_ids: ::prost::alloc::vec::Vec<u32>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SearchKey {
    #[prost(map = "uint32, message", tag = "1")]
    pub purpose_map: ::std::collections::HashMap<u32, SecurityLevelMap>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityKeysRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityKeysRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Keys {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub keys_bytes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Keys(Keys),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityKeysResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    }
    /// Nested message and enum types in `GetProofsRequestV0`.
    pub mod get_proofs_request_v0 {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
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
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct IdentityRequest {
            #[prost(bytes = "vec", tag = "1")]
            #[serde(with = "serde_bytes")]
            pub identity_id: ::prost::alloc::vec::Vec<u8>,
            #[prost(enumeration = "identity_request::Type", tag = "2")]
            pub request_type: i32,
        }
        /// Nested message and enum types in `IdentityRequest`.
        pub mod identity_request {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct ContractRequest {
            #[prost(bytes = "vec", tag = "1")]
            pub contract_id: ::prost::alloc::vec::Vec<u8>,
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProofsRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProofsResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(bytes, tag = "1")]
            DataContract(::prost::alloc::vec::Vec<u8>),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractsRequestV0 {
        #[prost(bytes = "vec", repeated, tag = "1")]
        #[serde(with = "crate::deserialization::vec_base64string")]
        pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractsRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub identifier: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub data_contract: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContracts {
        #[prost(message, repeated, tag = "1")]
        pub data_contract_entries: ::prost::alloc::vec::Vec<DataContractEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            DataContracts(super::DataContracts),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractsResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDataContractHistoryRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub id: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub limit: ::core::option::Option<u32>,
        #[prost(message, optional, tag = "3")]
        pub offset: ::core::option::Option<u32>,
        #[prost(uint64, tag = "4")]
        #[serde(with = "crate::deserialization::from_to_string")]
        pub start_at_ms: u64,
        #[prost(bool, tag = "5")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractHistoryRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct DataContractHistoryEntry {
            #[prost(uint64, tag = "1")]
            pub date: u64,
            #[prost(bytes = "vec", tag = "2")]
            pub value: ::prost::alloc::vec::Vec<u8>,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct DataContractHistory {
            #[prost(message, repeated, tag = "1")]
            pub data_contract_entries: ::prost::alloc::vec::Vec<
                DataContractHistoryEntry,
            >,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            DataContractHistory(DataContractHistory),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDataContractHistoryResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetDocumentsRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub data_contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(string, tag = "2")]
        pub document_type: ::prost::alloc::string::String,
        #[prost(bytes = "vec", tag = "3")]
        #[serde(with = "serde_bytes")]
        pub r#where: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "4")]
        #[serde(with = "serde_bytes")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Start {
            #[prost(bytes, tag = "6")]
            StartAfter(::prost::alloc::vec::Vec<u8>),
            #[prost(bytes, tag = "7")]
            StartAt(::prost::alloc::vec::Vec<u8>),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDocumentsRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Documents {
            #[prost(bytes = "vec", repeated, tag = "1")]
            pub documents: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Documents(Documents),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetDocumentsResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::VersionedGrpcMessage)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesByPublicKeyHashesRequest {
    #[prost(oneof = "get_identities_by_public_key_hashes_request::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_identities_by_public_key_hashes_request::Version,
    >,
}
/// Nested message and enum types in `GetIdentitiesByPublicKeyHashesRequest`.
pub mod get_identities_by_public_key_hashes_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentitiesByPublicKeyHashesRequestV0 {
        #[prost(bytes = "vec", repeated, tag = "1")]
        #[serde(with = "crate::deserialization::vec_base64string")]
        pub public_key_hashes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentitiesByPublicKeyHashesRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(
    ::dapi_grpc_macros::VersionedGrpcMessage,
    ::dapi_grpc_macros::VersionedGrpcResponse
)]
#[grpc_versions(0)]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesByPublicKeyHashesResponse {
    #[prost(oneof = "get_identities_by_public_key_hashes_response::Version", tags = "1")]
    pub version: ::core::option::Option<
        get_identities_by_public_key_hashes_response::Version,
    >,
}
/// Nested message and enum types in `GetIdentitiesByPublicKeyHashesResponse`.
pub mod get_identities_by_public_key_hashes_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKeyHashIdentityEntry {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub public_key_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentitiesByPublicKeyHashes {
        #[prost(message, repeated, tag = "1")]
        pub identity_entries: ::prost::alloc::vec::Vec<PublicKeyHashIdentityEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentitiesByPublicKeyHashesResponseV0 {
        #[prost(message, optional, tag = "3")]
        pub metadata: ::core::option::Option<super::ResponseMetadata>,
        #[prost(
            oneof = "get_identities_by_public_key_hashes_response_v0::Result",
            tags = "1, 2"
        )]
        pub result: ::core::option::Option<
            get_identities_by_public_key_hashes_response_v0::Result,
        >,
    }
    /// Nested message and enum types in `GetIdentitiesByPublicKeyHashesResponseV0`.
    pub mod get_identities_by_public_key_hashes_response_v0 {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Identities(super::IdentitiesByPublicKeyHashes),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentitiesByPublicKeyHashesResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetIdentityByPublicKeyHashRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        #[serde(with = "serde_bytes")]
        pub public_key_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityByPublicKeyHashRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(bytes, tag = "1")]
            Identity(::prost::alloc::vec::Vec<u8>),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetIdentityByPublicKeyHashResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct WaitForStateTransitionResultRequestV0 {
        #[prost(bytes = "vec", tag = "1")]
        pub state_transition_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(WaitForStateTransitionResultRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Error(super::super::StateTransitionBroadcastError),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(WaitForStateTransitionResultResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsRequest {
    #[prost(oneof = "get_consensus_params_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_consensus_params_request::Version>,
}
/// Nested message and enum types in `GetConsensusParamsRequest`.
pub mod get_consensus_params_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetConsensusParamsRequestV0 {
        #[prost(int32, tag = "1")]
        pub height: i32,
        #[prost(bool, tag = "2")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetConsensusParamsRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsResponse {
    #[prost(oneof = "get_consensus_params_response::Version", tags = "1")]
    pub version: ::core::option::Option<get_consensus_params_response::Version>,
}
/// Nested message and enum types in `GetConsensusParamsResponse`.
pub mod get_consensus_params_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetConsensusParamsResponseV0 {
        #[prost(message, optional, tag = "1")]
        pub block: ::core::option::Option<ConsensusParamsBlock>,
        #[prost(message, optional, tag = "2")]
        pub evidence: ::core::option::Option<ConsensusParamsEvidence>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetConsensusParamsResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[derive(::dapi_grpc_macros::Mockable)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct GetProtocolVersionUpgradeStateRequestV0 {
        #[prost(bool, tag = "1")]
        pub prove: bool,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeStateRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct Versions {
            #[prost(message, repeated, tag = "1")]
            pub versions: ::prost::alloc::vec::Vec<VersionEntry>,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VersionEntry {
            #[prost(uint32, tag = "1")]
            pub version_number: u32,
            #[prost(uint32, tag = "2")]
            pub vote_count: u32,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Versions(Versions),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeStateResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeVoteStatusRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VersionSignals {
            #[prost(message, repeated, tag = "1")]
            pub version_signals: ::prost::alloc::vec::Vec<VersionSignal>,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct VersionSignal {
            #[prost(bytes = "vec", tag = "1")]
            pub pro_tx_hash: ::prost::alloc::vec::Vec<u8>,
            #[prost(uint32, tag = "2")]
            pub version: u32,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Versions(VersionSignals),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetProtocolVersionUpgradeVoteStatusResponseV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(::dapi_grpc_macros::Mockable)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEpochsInfoRequest {
    #[prost(oneof = "get_epochs_info_request::Version", tags = "1")]
    pub version: ::core::option::Option<get_epochs_info_request::Version>,
}
/// Nested message and enum types in `GetEpochsInfoRequest`.
pub mod get_epochs_info_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetEpochsInfoRequestV0),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[derive(::dapi_grpc_macros::Mockable)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Message)]
        pub struct EpochInfos {
            #[prost(message, repeated, tag = "1")]
            pub epoch_infos: ::prost::alloc::vec::Vec<EpochInfo>,
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
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
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "snake_case")]
        #[allow(clippy::derive_partial_eq_without_eq)]
        #[derive(Clone, PartialEq, ::prost::Oneof)]
        pub enum Result {
            #[prost(message, tag = "1")]
            Epochs(EpochInfos),
            #[prost(message, tag = "2")]
            Proof(super::super::Proof),
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Version {
        #[prost(message, tag = "1")]
        V0(GetEpochsInfoResponseV0),
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
        pub async fn get_identities(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentitiesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentitiesResponse>,
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
                "/org.dash.platform.dapi.v0.Platform/getIdentities",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentities",
                    ),
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
        pub async fn get_identities_by_public_key_hashes(
            &mut self,
            request: impl tonic::IntoRequest<
                super::GetIdentitiesByPublicKeyHashesRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentitiesByPublicKeyHashesResponse>,
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
                "/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentitiesByPublicKeyHashes",
                    ),
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
    }
}
/// Generated server implementations.
pub mod platform_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with PlatformServer.
    #[async_trait]
    pub trait Platform: Send + Sync + 'static {
        async fn broadcast_state_transition(
            &self,
            request: tonic::Request<super::BroadcastStateTransitionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::BroadcastStateTransitionResponse>,
            tonic::Status,
        >;
        async fn get_identity(
            &self,
            request: tonic::Request<super::GetIdentityRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityResponse>,
            tonic::Status,
        >;
        async fn get_identities(
            &self,
            request: tonic::Request<super::GetIdentitiesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentitiesResponse>,
            tonic::Status,
        >;
        async fn get_identity_keys(
            &self,
            request: tonic::Request<super::GetIdentityKeysRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityKeysResponse>,
            tonic::Status,
        >;
        async fn get_identity_nonce(
            &self,
            request: tonic::Request<super::GetIdentityNonceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityNonceResponse>,
            tonic::Status,
        >;
        async fn get_identity_contract_nonce(
            &self,
            request: tonic::Request<super::GetIdentityContractNonceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityContractNonceResponse>,
            tonic::Status,
        >;
        async fn get_identity_balance(
            &self,
            request: tonic::Request<super::GetIdentityBalanceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityBalanceResponse>,
            tonic::Status,
        >;
        async fn get_identity_balance_and_revision(
            &self,
            request: tonic::Request<super::GetIdentityBalanceAndRevisionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityBalanceAndRevisionResponse>,
            tonic::Status,
        >;
        async fn get_proofs(
            &self,
            request: tonic::Request<super::GetProofsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetProofsResponse>,
            tonic::Status,
        >;
        async fn get_data_contract(
            &self,
            request: tonic::Request<super::GetDataContractRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDataContractResponse>,
            tonic::Status,
        >;
        async fn get_data_contract_history(
            &self,
            request: tonic::Request<super::GetDataContractHistoryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDataContractHistoryResponse>,
            tonic::Status,
        >;
        async fn get_data_contracts(
            &self,
            request: tonic::Request<super::GetDataContractsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDataContractsResponse>,
            tonic::Status,
        >;
        async fn get_documents(
            &self,
            request: tonic::Request<super::GetDocumentsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetDocumentsResponse>,
            tonic::Status,
        >;
        async fn get_identities_by_public_key_hashes(
            &self,
            request: tonic::Request<super::GetIdentitiesByPublicKeyHashesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentitiesByPublicKeyHashesResponse>,
            tonic::Status,
        >;
        async fn get_identity_by_public_key_hash(
            &self,
            request: tonic::Request<super::GetIdentityByPublicKeyHashRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityByPublicKeyHashResponse>,
            tonic::Status,
        >;
        async fn wait_for_state_transition_result(
            &self,
            request: tonic::Request<super::WaitForStateTransitionResultRequest>,
        ) -> std::result::Result<
            tonic::Response<super::WaitForStateTransitionResultResponse>,
            tonic::Status,
        >;
        async fn get_consensus_params(
            &self,
            request: tonic::Request<super::GetConsensusParamsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetConsensusParamsResponse>,
            tonic::Status,
        >;
        async fn get_protocol_version_upgrade_state(
            &self,
            request: tonic::Request<super::GetProtocolVersionUpgradeStateRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetProtocolVersionUpgradeStateResponse>,
            tonic::Status,
        >;
        async fn get_protocol_version_upgrade_vote_status(
            &self,
            request: tonic::Request<super::GetProtocolVersionUpgradeVoteStatusRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetProtocolVersionUpgradeVoteStatusResponse>,
            tonic::Status,
        >;
        async fn get_epochs_info(
            &self,
            request: tonic::Request<super::GetEpochsInfoRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetEpochsInfoResponse>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct PlatformServer<T: Platform> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Platform> PlatformServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for PlatformServer<T>
    where
        T: Platform,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/org.dash.platform.dapi.v0.Platform/broadcastStateTransition" => {
                    #[allow(non_camel_case_types)]
                    struct broadcastStateTransitionSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::BroadcastStateTransitionRequest>
                    for broadcastStateTransitionSvc<T> {
                        type Response = super::BroadcastStateTransitionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::BroadcastStateTransitionRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).broadcast_state_transition(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = broadcastStateTransitionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentity" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentitySvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetIdentityRequest>
                    for getIdentitySvc<T> {
                        type Response = super::GetIdentityResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetIdentityRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentitySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentities" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentitiesSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetIdentitiesRequest>
                    for getIdentitiesSvc<T> {
                        type Response = super::GetIdentitiesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetIdentitiesRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identities(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentitiesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentityKeys" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentityKeysSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetIdentityKeysRequest>
                    for getIdentityKeysSvc<T> {
                        type Response = super::GetIdentityKeysResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetIdentityKeysRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity_keys(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentityKeysSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentityNonce" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentityNonceSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetIdentityNonceRequest>
                    for getIdentityNonceSvc<T> {
                        type Response = super::GetIdentityNonceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetIdentityNonceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity_nonce(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentityNonceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentityContractNonce" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentityContractNonceSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetIdentityContractNonceRequest>
                    for getIdentityContractNonceSvc<T> {
                        type Response = super::GetIdentityContractNonceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetIdentityContractNonceRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity_contract_nonce(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentityContractNonceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentityBalance" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentityBalanceSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetIdentityBalanceRequest>
                    for getIdentityBalanceSvc<T> {
                        type Response = super::GetIdentityBalanceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetIdentityBalanceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity_balance(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentityBalanceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentityBalanceAndRevision" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentityBalanceAndRevisionSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<
                        super::GetIdentityBalanceAndRevisionRequest,
                    > for getIdentityBalanceAndRevisionSvc<T> {
                        type Response = super::GetIdentityBalanceAndRevisionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetIdentityBalanceAndRevisionRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity_balance_and_revision(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentityBalanceAndRevisionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getProofs" => {
                    #[allow(non_camel_case_types)]
                    struct getProofsSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetProofsRequest>
                    for getProofsSvc<T> {
                        type Response = super::GetProofsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetProofsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).get_proofs(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getProofsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getDataContract" => {
                    #[allow(non_camel_case_types)]
                    struct getDataContractSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetDataContractRequest>
                    for getDataContractSvc<T> {
                        type Response = super::GetDataContractResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetDataContractRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_data_contract(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getDataContractSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getDataContractHistory" => {
                    #[allow(non_camel_case_types)]
                    struct getDataContractHistorySvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetDataContractHistoryRequest>
                    for getDataContractHistorySvc<T> {
                        type Response = super::GetDataContractHistoryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetDataContractHistoryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_data_contract_history(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getDataContractHistorySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getDataContracts" => {
                    #[allow(non_camel_case_types)]
                    struct getDataContractsSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetDataContractsRequest>
                    for getDataContractsSvc<T> {
                        type Response = super::GetDataContractsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetDataContractsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_data_contracts(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getDataContractsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getDocuments" => {
                    #[allow(non_camel_case_types)]
                    struct getDocumentsSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetDocumentsRequest>
                    for getDocumentsSvc<T> {
                        type Response = super::GetDocumentsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetDocumentsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_documents(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getDocumentsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentitiesByPublicKeyHashesSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<
                        super::GetIdentitiesByPublicKeyHashesRequest,
                    > for getIdentitiesByPublicKeyHashesSvc<T> {
                        type Response = super::GetIdentitiesByPublicKeyHashesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetIdentitiesByPublicKeyHashesRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identities_by_public_key_hashes(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentitiesByPublicKeyHashesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getIdentityByPublicKeyHash" => {
                    #[allow(non_camel_case_types)]
                    struct getIdentityByPublicKeyHashSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<
                        super::GetIdentityByPublicKeyHashRequest,
                    > for getIdentityByPublicKeyHashSvc<T> {
                        type Response = super::GetIdentityByPublicKeyHashResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetIdentityByPublicKeyHashRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_identity_by_public_key_hash(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getIdentityByPublicKeyHashSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult" => {
                    #[allow(non_camel_case_types)]
                    struct waitForStateTransitionResultSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<
                        super::WaitForStateTransitionResultRequest,
                    > for waitForStateTransitionResultSvc<T> {
                        type Response = super::WaitForStateTransitionResultResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::WaitForStateTransitionResultRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).wait_for_state_transition_result(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = waitForStateTransitionResultSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getConsensusParams" => {
                    #[allow(non_camel_case_types)]
                    struct getConsensusParamsSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetConsensusParamsRequest>
                    for getConsensusParamsSvc<T> {
                        type Response = super::GetConsensusParamsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetConsensusParamsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_consensus_params(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getConsensusParamsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getProtocolVersionUpgradeState" => {
                    #[allow(non_camel_case_types)]
                    struct getProtocolVersionUpgradeStateSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<
                        super::GetProtocolVersionUpgradeStateRequest,
                    > for getProtocolVersionUpgradeStateSvc<T> {
                        type Response = super::GetProtocolVersionUpgradeStateResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetProtocolVersionUpgradeStateRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_protocol_version_upgrade_state(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getProtocolVersionUpgradeStateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getProtocolVersionUpgradeVoteStatus" => {
                    #[allow(non_camel_case_types)]
                    struct getProtocolVersionUpgradeVoteStatusSvc<T: Platform>(
                        pub Arc<T>,
                    );
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<
                        super::GetProtocolVersionUpgradeVoteStatusRequest,
                    > for getProtocolVersionUpgradeVoteStatusSvc<T> {
                        type Response = super::GetProtocolVersionUpgradeVoteStatusResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::GetProtocolVersionUpgradeVoteStatusRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner)
                                    .get_protocol_version_upgrade_vote_status(request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getProtocolVersionUpgradeVoteStatusSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/org.dash.platform.dapi.v0.Platform/getEpochsInfo" => {
                    #[allow(non_camel_case_types)]
                    struct getEpochsInfoSvc<T: Platform>(pub Arc<T>);
                    impl<
                        T: Platform,
                    > tonic::server::UnaryService<super::GetEpochsInfoRequest>
                    for getEpochsInfoSvc<T> {
                        type Response = super::GetEpochsInfoResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetEpochsInfoRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).get_epochs_info(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = getEpochsInfoSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Platform> Clone for PlatformServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: Platform> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Platform> tonic::server::NamedService for PlatformServer<T> {
        const NAME: &'static str = "org.dash.platform.dapi.v0.Platform";
    }
}
