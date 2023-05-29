#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Proof {
    #[prost(bytes = "vec", tag = "1")]
    pub grovedb_proof: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub quorum_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "4")]
    pub round: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResponseMetadata {
    #[prost(uint64, tag = "1")]
    pub height: u64,
    #[prost(uint32, tag = "2")]
    pub core_chain_locked_height: u32,
    #[prost(uint64, tag = "3")]
    pub time_ms: u64,
    #[prost(uint32, tag = "4")]
    pub protocol_version: u32,
}
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub state_transition: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastStateTransitionResponse {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub identity: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentityValue {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentityEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<IdentityValue>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Identities {
        #[prost(message, repeated, tag = "1")]
        pub identity_entries: ::prost::alloc::vec::Vec<IdentityEntry>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Identities(Identities),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceResponse {
    #[prost(message, optional, tag = "1")]
    pub balance: ::core::option::Option<u64>,
    #[prost(message, optional, tag = "2")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityBalanceAndRevisionResponse {
    #[prost(message, optional, tag = "1")]
    pub balance: ::core::option::Option<u64>,
    #[prost(message, optional, tag = "2")]
    pub revision: ::core::option::Option<u64>,
    #[prost(message, optional, tag = "3")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "4")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeyRequestType {
    #[prost(oneof = "key_request_type::Request", tags = "1, 2, 3")]
    pub request: ::core::option::Option<key_request_type::Request>,
}
/// Nested message and enum types in `KeyRequestType`.
pub mod key_request_type {
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllKeys {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpecificKeys {
    #[prost(uint32, repeated, tag = "1")]
    pub key_ids: ::prost::alloc::vec::Vec<u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SearchKey {
    #[prost(map = "uint32, message", tag = "1")]
    pub purpose_map: ::std::collections::HashMap<u32, SecurityLevelMap>,
}
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityKeysRequest {
    #[prost(bytes = "vec", tag = "1")]
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Keys {
        #[prost(bytes = "vec", repeated, tag = "1")]
        pub keys_bytes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        Keys(Keys),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKey {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKeyEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<PublicKey>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct PublicKeyEntries {
        #[prost(message, repeated, tag = "1")]
        pub public_key_entries: ::prost::alloc::vec::Vec<PublicKeyEntry>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        PublicKeys(PublicKeyEntries),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct IdentityRequest {
        #[prost(bytes = "vec", tag = "1")]
        pub identity_id: ::prost::alloc::vec::Vec<u8>,
        #[prost(enumeration = "identity_request::Type", tag = "2")]
        pub request_type: i32,
    }
    /// Nested message and enum types in `IdentityRequest`.
    pub mod identity_request {
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ContractRequest {
        #[prost(bytes = "vec", tag = "1")]
        pub contract_id: ::prost::alloc::vec::Vec<u8>,
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetProofsResponse {
    #[prost(message, optional, tag = "1")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "2")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub data_contract: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "2")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDataContractsRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub ids: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractValue {
        #[prost(bytes = "vec", tag = "1")]
        pub value: ::prost::alloc::vec::Vec<u8>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContractEntry {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(message, optional, tag = "2")]
        pub value: ::core::option::Option<DataContractValue>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct DataContracts {
        #[prost(message, repeated, tag = "1")]
        pub data_contract_entries: ::prost::alloc::vec::Vec<DataContractEntry>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(message, tag = "1")]
        DataContracts(DataContracts),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDocumentsRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub data_contract_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "2")]
    pub document_type: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "3")]
    pub r#where: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Start {
        #[prost(bytes, tag = "6")]
        StartAfter(::prost::alloc::vec::Vec<u8>),
        #[prost(bytes, tag = "7")]
        StartAt(::prost::alloc::vec::Vec<u8>),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetDocumentsResponse {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub documents: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(message, optional, tag = "2")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesByPublicKeyHashesRequest {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub public_key_hashes: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentitiesByPublicKeyHashesResponse {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub identities: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(message, optional, tag = "2")]
    pub proof: ::core::option::Option<Proof>,
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetIdentityByPublicKeyHashesRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub public_key_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
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
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        #[prost(bytes, tag = "1")]
        Identity(::prost::alloc::vec::Vec<u8>),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitForStateTransitionResultRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub state_transition_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WaitForStateTransitionResultResponse {
    #[prost(message, optional, tag = "3")]
    pub metadata: ::core::option::Option<ResponseMetadata>,
    #[prost(
        oneof = "wait_for_state_transition_result_response::Responses",
        tags = "1, 2"
    )]
    pub responses: ::core::option::Option<
        wait_for_state_transition_result_response::Responses,
    >,
}
/// Nested message and enum types in `WaitForStateTransitionResultResponse`.
pub mod wait_for_state_transition_result_response {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Responses {
        #[prost(message, tag = "1")]
        Error(super::StateTransitionBroadcastError),
        #[prost(message, tag = "2")]
        Proof(super::Proof),
    }
}
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
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsRequest {
    #[prost(int64, tag = "1")]
    pub height: i64,
    #[prost(bool, tag = "2")]
    pub prove: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConsensusParamsResponse {
    #[prost(message, optional, tag = "1")]
    pub block: ::core::option::Option<ConsensusParamsBlock>,
    #[prost(message, optional, tag = "2")]
    pub evidence: ::core::option::Option<ConsensusParamsEvidence>,
}
