// This file is generated. Do not edit
// @generated

// https://github.com/Manishearth/rust-clippy/issues/702
#![allow(unknown_lints)]
#![allow(clippy::all)]

#![cfg_attr(rustfmt, rustfmt_skip)]

#![allow(box_pointers)]
#![allow(dead_code)]
#![allow(missing_docs)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(trivial_casts)]
#![allow(unsafe_code)]
#![allow(unused_imports)]
#![allow(unused_results)]


// server interface

pub trait Core {
    fn get_status(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::GetStatusRequest>, resp: ::grpc::ServerResponseUnarySink<super::core::GetStatusResponse>) -> ::grpc::Result<()>;

    fn get_block(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::GetBlockRequest>, resp: ::grpc::ServerResponseUnarySink<super::core::GetBlockResponse>) -> ::grpc::Result<()>;

    fn broadcast_transaction(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::BroadcastTransactionRequest>, resp: ::grpc::ServerResponseUnarySink<super::core::BroadcastTransactionResponse>) -> ::grpc::Result<()>;

    fn get_transaction(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::GetTransactionRequest>, resp: ::grpc::ServerResponseUnarySink<super::core::GetTransactionResponse>) -> ::grpc::Result<()>;

    fn get_estimated_transaction_fee(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::GetEstimatedTransactionFeeRequest>, resp: ::grpc::ServerResponseUnarySink<super::core::GetEstimatedTransactionFeeResponse>) -> ::grpc::Result<()>;

    fn subscribe_to_block_headers_with_chain_locks(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::BlockHeadersWithChainLocksRequest>, resp: ::grpc::ServerResponseSink<super::core::BlockHeadersWithChainLocksResponse>) -> ::grpc::Result<()>;

    fn subscribe_to_transactions_with_proofs(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::core::TransactionsWithProofsRequest>, resp: ::grpc::ServerResponseSink<super::core::TransactionsWithProofsResponse>) -> ::grpc::Result<()>;
}

// client

pub struct CoreClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
}

impl ::grpc::ClientStub for CoreClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        CoreClient {
            grpc_client: grpc_client,
        }
    }
}

impl CoreClient {
    pub fn get_status(&self, o: ::grpc::RequestOptions, req: super::core::GetStatusRequest) -> ::grpc::SingleResponse<super::core::GetStatusResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getStatus"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_block(&self, o: ::grpc::RequestOptions, req: super::core::GetBlockRequest) -> ::grpc::SingleResponse<super::core::GetBlockResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getBlock"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn broadcast_transaction(&self, o: ::grpc::RequestOptions, req: super::core::BroadcastTransactionRequest) -> ::grpc::SingleResponse<super::core::BroadcastTransactionResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/broadcastTransaction"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_transaction(&self, o: ::grpc::RequestOptions, req: super::core::GetTransactionRequest) -> ::grpc::SingleResponse<super::core::GetTransactionResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getTransaction"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_estimated_transaction_fee(&self, o: ::grpc::RequestOptions, req: super::core::GetEstimatedTransactionFeeRequest) -> ::grpc::SingleResponse<super::core::GetEstimatedTransactionFeeResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn subscribe_to_block_headers_with_chain_locks(&self, o: ::grpc::RequestOptions, req: super::core::BlockHeadersWithChainLocksRequest) -> ::grpc::StreamingResponse<super::core::BlockHeadersWithChainLocksResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }

    pub fn subscribe_to_transactions_with_proofs(&self, o: ::grpc::RequestOptions, req: super::core::TransactionsWithProofsRequest) -> ::grpc::StreamingResponse<super::core::TransactionsWithProofsResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs"),
            streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_server_streaming(o, req, descriptor)
    }
}

// server

pub struct CoreServer;


impl CoreServer {
    pub fn new_service_def<H : Core + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/org.dash.platform.dapi.v0.Core",
            vec![
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getStatus"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_status(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getBlock"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_block(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/broadcastTransaction"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).broadcast_transaction(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getTransaction"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_transaction(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/getEstimatedTransactionFee"),
                        streaming: ::grpc::rt::GrpcStreaming::Unary,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_estimated_transaction_fee(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/subscribeToBlockHeadersWithChainLocks"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).subscribe_to_block_headers_with_chain_locks(ctx, req, resp))
                    },
                ),
                ::grpc::rt::ServerMethod::new(
                    ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                        name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Core/subscribeToTransactionsWithProofs"),
                        streaming: ::grpc::rt::GrpcStreaming::ServerStreaming,
                        req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                        resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                    }),
                    {
                        let handler_copy = handler_arc.clone();
                        ::grpc::rt::MethodHandlerServerStreaming::new(move |ctx, req, resp| (*handler_copy).subscribe_to_transactions_with_proofs(ctx, req, resp))
                    },
                ),
            ],
        )
    }
}
