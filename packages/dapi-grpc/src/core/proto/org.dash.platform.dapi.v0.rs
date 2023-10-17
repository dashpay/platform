#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetStatusRequest {}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetStatusResponse {
    #[prost(message, optional, tag = "1")]
    pub version: ::core::option::Option<get_status_response::Version>,
    #[prost(message, optional, tag = "2")]
    pub time: ::core::option::Option<get_status_response::Time>,
    #[prost(enumeration = "get_status_response::Status", tag = "3")]
    pub status: i32,
    #[prost(double, tag = "4")]
    pub sync_progress: f64,
    #[prost(message, optional, tag = "5")]
    pub chain: ::core::option::Option<get_status_response::Chain>,
    #[prost(message, optional, tag = "6")]
    pub masternode: ::core::option::Option<get_status_response::Masternode>,
    #[prost(message, optional, tag = "7")]
    pub network: ::core::option::Option<get_status_response::Network>,
}
/// Nested message and enum types in `GetStatusResponse`.
pub mod get_status_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Version {
        #[prost(uint32, tag = "1")]
        pub protocol: u32,
        #[prost(uint32, tag = "2")]
        pub software: u32,
        #[prost(string, tag = "3")]
        pub agent: ::prost::alloc::string::String,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Time {
        #[prost(uint32, tag = "1")]
        pub now: u32,
        #[prost(int32, tag = "2")]
        pub offset: i32,
        #[prost(uint32, tag = "3")]
        pub median: u32,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Chain {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(uint32, tag = "2")]
        pub headers_count: u32,
        #[prost(uint32, tag = "3")]
        pub blocks_count: u32,
        #[prost(bytes = "vec", tag = "4")]
        pub best_block_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(double, tag = "5")]
        pub difficulty: f64,
        #[prost(bytes = "vec", tag = "6")]
        pub chain_work: ::prost::alloc::vec::Vec<u8>,
        #[prost(bool, tag = "7")]
        pub is_synced: bool,
        #[prost(double, tag = "8")]
        pub sync_progress: f64,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Masternode {
        #[prost(enumeration = "masternode::Status", tag = "1")]
        pub status: i32,
        #[prost(bytes = "vec", tag = "2")]
        pub pro_tx_hash: ::prost::alloc::vec::Vec<u8>,
        #[prost(uint32, tag = "3")]
        pub pose_penalty: u32,
        #[prost(bool, tag = "4")]
        pub is_synced: bool,
        #[prost(double, tag = "5")]
        pub sync_progress: f64,
    }
    /// Nested message and enum types in `Masternode`.
    pub mod masternode {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
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
        pub enum Status {
            Unknown = 0,
            WaitingForProtx = 1,
            PoseBanned = 2,
            Removed = 3,
            OperatorKeyChanged = 4,
            ProtxIpChanged = 5,
            Ready = 6,
            Error = 7,
        }
        impl Status {
            /// String value of the enum field names used in the ProtoBuf definition.
            ///
            /// The values are not transformed in any way and thus are considered stable
            /// (if the ProtoBuf definition does not change) and safe for programmatic use.
            pub fn as_str_name(&self) -> &'static str {
                match self {
                    Status::Unknown => "UNKNOWN",
                    Status::WaitingForProtx => "WAITING_FOR_PROTX",
                    Status::PoseBanned => "POSE_BANNED",
                    Status::Removed => "REMOVED",
                    Status::OperatorKeyChanged => "OPERATOR_KEY_CHANGED",
                    Status::ProtxIpChanged => "PROTX_IP_CHANGED",
                    Status::Ready => "READY",
                    Status::Error => "ERROR",
                }
            }
            /// Creates an enum from field names used in the ProtoBuf definition.
            pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
                match value {
                    "UNKNOWN" => Some(Self::Unknown),
                    "WAITING_FOR_PROTX" => Some(Self::WaitingForProtx),
                    "POSE_BANNED" => Some(Self::PoseBanned),
                    "REMOVED" => Some(Self::Removed),
                    "OPERATOR_KEY_CHANGED" => Some(Self::OperatorKeyChanged),
                    "PROTX_IP_CHANGED" => Some(Self::ProtxIpChanged),
                    "READY" => Some(Self::Ready),
                    "ERROR" => Some(Self::Error),
                    _ => None,
                }
            }
        }
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct NetworkFee {
        #[prost(double, tag = "1")]
        pub relay: f64,
        #[prost(double, tag = "2")]
        pub incremental: f64,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Network {
        #[prost(uint32, tag = "1")]
        pub peers_count: u32,
        #[prost(message, optional, tag = "2")]
        pub fee: ::core::option::Option<NetworkFee>,
    }
    #[derive(::serde::Serialize, ::serde::Deserialize)]
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
    pub enum Status {
        NotStarted = 0,
        Syncing = 1,
        Ready = 2,
        Error = 3,
    }
    impl Status {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Status::NotStarted => "NOT_STARTED",
                Status::Syncing => "SYNCING",
                Status::Ready => "READY",
                Status::Error => "ERROR",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "NOT_STARTED" => Some(Self::NotStarted),
                "SYNCING" => Some(Self::Syncing),
                "READY" => Some(Self::Ready),
                "ERROR" => Some(Self::Error),
                _ => None,
            }
        }
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockRequest {
    #[prost(oneof = "get_block_request::Block", tags = "1, 2")]
    pub block: ::core::option::Option<get_block_request::Block>,
}
/// Nested message and enum types in `GetBlockRequest`.
pub mod get_block_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Block {
        #[prost(uint32, tag = "1")]
        Height(u32),
        #[prost(string, tag = "2")]
        Hash(::prost::alloc::string::String),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetBlockResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub block: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastTransactionRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub transaction: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub allow_high_fees: bool,
    #[prost(bool, tag = "3")]
    pub bypass_limits: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BroadcastTransactionResponse {
    #[prost(string, tag = "1")]
    pub transaction_id: ::prost::alloc::string::String,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTransactionRequest {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetTransactionResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub transaction: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub block_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "3")]
    pub height: u32,
    #[prost(uint32, tag = "4")]
    pub confirmations: u32,
    #[prost(bool, tag = "5")]
    pub is_instant_locked: bool,
    #[prost(bool, tag = "6")]
    pub is_chain_locked: bool,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockHeadersWithChainLocksRequest {
    #[prost(uint32, tag = "3")]
    pub count: u32,
    #[prost(oneof = "block_headers_with_chain_locks_request::FromBlock", tags = "1, 2")]
    pub from_block: ::core::option::Option<
        block_headers_with_chain_locks_request::FromBlock,
    >,
}
/// Nested message and enum types in `BlockHeadersWithChainLocksRequest`.
pub mod block_headers_with_chain_locks_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum FromBlock {
        #[prost(bytes, tag = "1")]
        FromBlockHash(::prost::alloc::vec::Vec<u8>),
        #[prost(uint32, tag = "2")]
        FromBlockHeight(u32),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockHeadersWithChainLocksResponse {
    #[prost(oneof = "block_headers_with_chain_locks_response::Responses", tags = "1, 2")]
    pub responses: ::core::option::Option<
        block_headers_with_chain_locks_response::Responses,
    >,
}
/// Nested message and enum types in `BlockHeadersWithChainLocksResponse`.
pub mod block_headers_with_chain_locks_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Responses {
        #[prost(message, tag = "1")]
        BlockHeaders(super::BlockHeaders),
        #[prost(bytes, tag = "2")]
        ChainLock(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockHeaders {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub headers: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEstimatedTransactionFeeRequest {
    #[prost(uint32, tag = "1")]
    pub blocks: u32,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetEstimatedTransactionFeeResponse {
    #[prost(double, tag = "1")]
    pub fee: f64,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionsWithProofsRequest {
    #[prost(message, optional, tag = "1")]
    pub bloom_filter: ::core::option::Option<BloomFilter>,
    #[prost(uint32, tag = "4")]
    pub count: u32,
    #[prost(bool, tag = "5")]
    pub send_transaction_hashes: bool,
    #[prost(oneof = "transactions_with_proofs_request::FromBlock", tags = "2, 3")]
    pub from_block: ::core::option::Option<transactions_with_proofs_request::FromBlock>,
}
/// Nested message and enum types in `TransactionsWithProofsRequest`.
pub mod transactions_with_proofs_request {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum FromBlock {
        #[prost(bytes, tag = "2")]
        FromBlockHash(::prost::alloc::vec::Vec<u8>),
        #[prost(uint32, tag = "3")]
        FromBlockHeight(u32),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BloomFilter {
    #[prost(bytes = "vec", tag = "1")]
    pub v_data: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub n_hash_funcs: u32,
    #[prost(uint32, tag = "3")]
    pub n_tweak: u32,
    #[prost(uint32, tag = "4")]
    pub n_flags: u32,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionsWithProofsResponse {
    #[prost(oneof = "transactions_with_proofs_response::Responses", tags = "1, 2, 3")]
    pub responses: ::core::option::Option<transactions_with_proofs_response::Responses>,
}
/// Nested message and enum types in `TransactionsWithProofsResponse`.
pub mod transactions_with_proofs_response {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Responses {
        #[prost(message, tag = "1")]
        RawTransactions(super::RawTransactions),
        #[prost(message, tag = "2")]
        InstantSendLockMessages(super::InstantSendLockMessages),
        #[prost(bytes, tag = "3")]
        RawMerkleBlock(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RawTransactions {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub transactions: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct InstantSendLockMessages {
    #[prost(bytes = "vec", repeated, tag = "1")]
    pub messages: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
/// Generated client implementations.
pub mod core_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct CoreClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl CoreClient<tonic::transport::Channel> {
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
    impl<T> CoreClient<T>
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
        ) -> CoreClient<InterceptedService<T, F>>
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
            CoreClient::new(InterceptedService::new(inner, interceptor))
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
        pub async fn get_status(
            &mut self,
            request: impl tonic::IntoRequest<super::GetStatusRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetStatusResponse>,
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
                "/org.dash.platform.dapi.v0.Core/getStatus",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("org.dash.platform.dapi.v0.Core", "getStatus"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_block(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetBlockResponse>,
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
                "/org.dash.platform.dapi.v0.Core/getBlock",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("org.dash.platform.dapi.v0.Core", "getBlock"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn broadcast_transaction(
            &mut self,
            request: impl tonic::IntoRequest<super::BroadcastTransactionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::BroadcastTransactionResponse>,
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
                "/org.dash.platform.dapi.v0.Core/broadcastTransaction",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Core",
                        "broadcastTransaction",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_transaction(
            &mut self,
            request: impl tonic::IntoRequest<super::GetTransactionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetTransactionResponse>,
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
                "/org.dash.platform.dapi.v0.Core/getTransaction",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("org.dash.platform.dapi.v0.Core", "getTransaction"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn get_estimated_transaction_fee(
            &mut self,
            request: impl tonic::IntoRequest<super::GetEstimatedTransactionFeeRequest>,
        ) -> std::result::Result<
            tonic::Response<super::GetEstimatedTransactionFeeResponse>,
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
                "/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Core",
                        "getEstimatedTransactionFee",
                    ),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn subscribe_to_block_headers_with_chain_locks(
            &mut self,
            request: impl tonic::IntoRequest<super::BlockHeadersWithChainLocksRequest>,
        ) -> std::result::Result<
            tonic::Response<
                tonic::codec::Streaming<super::BlockHeadersWithChainLocksResponse>,
            >,
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
                "/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Core",
                        "subscribeToBlockHeadersWithChainLocks",
                    ),
                );
            self.inner.server_streaming(req, path, codec).await
        }
        pub async fn subscribe_to_transactions_with_proofs(
            &mut self,
            request: impl tonic::IntoRequest<super::TransactionsWithProofsRequest>,
        ) -> std::result::Result<
            tonic::Response<
                tonic::codec::Streaming<super::TransactionsWithProofsResponse>,
            >,
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
                "/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "org.dash.platform.dapi.v0.Core",
                        "subscribeToTransactionsWithProofs",
                    ),
                );
            self.inner.server_streaming(req, path, codec).await
        }
    }
}
