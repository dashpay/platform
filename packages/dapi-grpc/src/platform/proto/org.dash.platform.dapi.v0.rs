#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResponseMetadata {
    #[prost(uint64, tag = "1")]
    #[serde(with = "crate::deserialization::from_to_string")]
    pub height: u64,
    #[prost(uint32, tag = "2")]
    pub core_chain_locked_height: u32,
    #[prost(uint64, tag = "3")]
    #[serde(with = "crate::deserialization::from_to_string")]
    pub time_ms: u64,
    #[prost(uint32, tag = "4")]
    pub protocol_version: u32,
    #[prost(string, tag = "5")]
    pub chain_id: ::prost::alloc::string::String,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub state_transition: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionResponse {}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityRequest {
    #[prost(bytes = "vec", tag = "1")]
    #[serde(with = "serde_bytes")]
    pub id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identity_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_identity_response::Result>,
}
/// Nested message and enum types in `GetIdentityResponse`.
pub mod get_identity_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(bytes, tag = "1")]
        Identity(::prost::alloc::vec::Vec<u8>),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    #[serde(with = "crate::deserialization::vec_base64string")]
    pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identities_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_identities_response::Result>,
}
/// Nested message and enum types in `GetIdentitiesResponse`.
pub mod get_identities_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentityValue {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Identities {
        #[prost(message, repeated, tag = "1")]
        pub identity_entries: ::prost::alloc::vec::Vec<IdentityEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Identities(Identities),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identity_balance_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_identity_balance_response::Result>,
}
/// Nested message and enum types in `GetIdentityBalanceResponse`.
pub mod get_identity_balance_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Balance(u64),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceAndRevisionResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identity_balance_and_revision_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<
        get_identity_balance_and_revision_response::Result,
    >,
}
/// Nested message and enum types in `GetIdentityBalanceAndRevisionResponse`.
pub mod get_identity_balance_and_revision_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct BalanceAndRevision {
        #[prost(message, optional, tag = "1")]
        pub balance: ::core::option::Option<u64>,
        #[prost(message, optional, tag = "2")]
        pub revision: ::core::option::Option<u64>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        BalanceAndRevision(BalanceAndRevision),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllKeys {}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpecificKeys {
    #[prost(uint32, repeated, tag = "1")]
    pub key_ids: ::prost::alloc::vec::Vec<u32>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SearchKey {
    #[prost(map = "uint32, message", tag = "1")]
    pub purpose_map: ::std::collections::HashMap<u32, SecurityLevelMap>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityKeysRequest {
    #[prost(bytes = "vec", tag = "1")]
    #[serde(with = "serde_bytes")]
    pub identity_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub request_type: ::core::option::Option<KeyRequestType>,
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityKeysResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identity_keys_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_identity_keys_response::Result>,
}
/// Nested message and enum types in `GetIdentityKeysResponse`.
pub mod get_identity_keys_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesKeysRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub identity_ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(message, optional, tag = "2")]
    pub request_type: ::core::option::Option<KeyRequestType>,
    #[prost(message, optional, tag = "3")]
    pub limit: ::core::option::Option<u32>,
    #[prost(message, optional, tag = "4")]
    pub offset: ::core::option::Option<u32>,
    #[prost(bool, tag = "5")]
    pub prove: bool,
}
/// Nested message and enum types in `GetIdentitiesKeysRequest`.
pub mod get_identities_keys_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "CURRENT_KEY_OF_KIND_REQUEST" => Some(Self::CurrentKeyOfKindRequest),
                    _ => None,
                }
            }
        }
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesKeysResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identities_keys_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_identities_keys_response::Result>,
}
/// Nested message and enum types in `GetIdentitiesKeysResponse`.
pub mod get_identities_keys_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKey {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKeyEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<PublicKey>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKeyEntries {
        #[prost(message, repeated, tag = "1")]
        pub public_key_entries: ::prost::alloc::vec::Vec<PublicKeyEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        PublicKeys(PublicKeyEntries),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofsRequest {
    #[prost(message, repeated, tag = "1")]
    pub identities: ::prost::alloc::vec::Vec<get_proofs_request::IdentityRequest>,
    #[prost(message, repeated, tag = "2")]
    pub contracts: ::prost::alloc::vec::Vec<get_proofs_request::ContractRequest>,
    #[prost(message, repeated, tag = "3")]
    pub documents: ::prost::alloc::vec::Vec<get_proofs_request::DocumentRequest>,
}
/// Nested message and enum types in `GetProofsRequest`.
pub mod get_proofs_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "FULL_IDENTITY" => Some(Self::FullIdentity),
                    "BALANCE" => Some(Self::Balance),
                    "KEYS" => Some(Self::Keys),
                    _ => None,
                }
            }
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ContractRequest {
        #[prost(bytes = "vec", tag = "1")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "2")]
        pub is_historical: bool,
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofsResponse {
    #[prost(message, optional, tag = "1")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "2")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractRequest {
    #[prost(bytes = "vec", tag = "1")]
    #[serde(with = "serde_bytes")]
    pub id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_data_contract_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_data_contract_response::Result>,
}
/// Nested message and enum types in `GetDataContractResponse`.
pub mod get_data_contract_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(bytes, tag = "1")]
        DataContract(::prost::alloc::vec::Vec<u8>),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractsRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    #[serde(with = "crate::deserialization::vec_base64string")]
    pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractsResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_data_contracts_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_data_contracts_response::Result>,
}
/// Nested message and enum types in `GetDataContractsResponse`.
pub mod get_data_contracts_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractValue {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<DataContractValue>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContracts {
        #[prost(message, repeated, tag = "1")]
        pub data_contract_entries: ::prost::alloc::vec::Vec<DataContractEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        DataContracts(DataContracts),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractHistoryRequest {
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractHistoryResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_data_contract_history_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_data_contract_history_response::Result>,
}
/// Nested message and enum types in `GetDataContractHistoryResponse`.
pub mod get_data_contract_history_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractHistory {
        #[prost(message, repeated, tag = "1")]
        pub data_contract_entries: ::prost::alloc::vec::Vec<DataContractHistoryEntry>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        DataContractHistory(DataContractHistory),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDocumentsRequest {
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
    #[prost(oneof = "get_documents_request::Start", tags = "6, 7")]
    pub start: ::core::option::Option<get_documents_request::Start>,
}
/// Nested message and enum types in `GetDocumentsRequest`.
pub mod get_documents_request {
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDocumentsResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_documents_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<get_documents_response::Result>,
}
/// Nested message and enum types in `GetDocumentsResponse`.
pub mod get_documents_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
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
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesByPublicKeyHashesRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    #[serde(with = "crate::deserialization::vec_base64string")]
    pub public_key_hashes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesByPublicKeyHashesResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(
        oneof = "get_identities_by_public_key_hashes_response::Result",
        tags = "1, 2"
    )]
    pub result: ::core::option::Option<
        get_identities_by_public_key_hashes_response::Result,
    >,
}
/// Nested message and enum types in `GetIdentitiesByPublicKeyHashesResponse`.
pub mod get_identities_by_public_key_hashes_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Identities {
        #[prost(bytes = "vec", repeated, tag = "1")]
        pub identities: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Identities(Identities),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityByPublicKeyHashesRequest {
    #[prost(bytes = "vec", tag = "1")]
    #[serde(with = "serde_bytes")]
    pub public_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityByPublicKeyHashesResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "get_identity_by_public_key_hashes_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<
        get_identity_by_public_key_hashes_response::Result,
    >,
}
/// Nested message and enum types in `GetIdentityByPublicKeyHashesResponse`.
pub mod get_identity_by_public_key_hashes_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(bytes, tag = "1")]
        Identity(::prost::alloc::vec::Vec<u8>),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitForStateTransitionResultRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub state_transition_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitForStateTransitionResultResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(oneof = "wait_for_state_transition_result_response::Result", tags = "1, 2")]
    pub result: ::core::option::Option<
        wait_for_state_transition_result_response::Result,
    >,
}
/// Nested message and enum types in `WaitForStateTransitionResultResponse`.
pub mod wait_for_state_transition_result_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Error(super::StateTransitionBroadcastError),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsRequest {
    #[prost(int64, tag = "1")]
    pub height: i64,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsResponse {
    #[prost(message, optional, tag = "1")]
    pub block: ::core::option::Option<ConsensusParamsBlock>,
    #[prost(message, optional, tag = "2")]
    pub evidence: ::core::option::Option<ConsensusParamsEvidence>,
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
        ///  rpc getIdentitiesKeys (GetIdentitiesKeysRequest) returns (GetIdentitiesKeysResponse);
        pub async fn get_identity_balance(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityRequest>,
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
            request: impl tonic::IntoRequest<super::GetIdentityRequest>,
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
        pub async fn get_identity_by_public_key_hashes(
            &mut self,
            request: impl tonic::IntoRequest<super::GetIdentityByPublicKeyHashesRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetIdentityByPublicKeyHashesResponse>,
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
                "/org.dash.platform.dapi.v0.Platform/getIdentityByPublicKeyHashes",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Platform",
                        "getIdentityByPublicKeyHashes",
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
    }
}
