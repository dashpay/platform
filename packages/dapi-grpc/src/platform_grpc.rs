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

pub trait Platform {
    fn broadcast_state_transition(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::BroadcastStateTransitionRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::BroadcastStateTransitionResponse>) -> ::grpc::Result<()>;

    fn get_identity(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::GetIdentityRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::GetIdentityResponse>) -> ::grpc::Result<()>;

    fn get_data_contract(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::GetDataContractRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::GetDataContractResponse>) -> ::grpc::Result<()>;

    fn get_documents(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::GetDocumentsRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::GetDocumentsResponse>) -> ::grpc::Result<()>;

    fn get_identities_by_public_key_hashes(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::GetIdentitiesByPublicKeyHashesRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::GetIdentitiesByPublicKeyHashesResponse>) -> ::grpc::Result<()>;

    fn wait_for_state_transition_result(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::WaitForStateTransitionResultRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::WaitForStateTransitionResultResponse>) -> ::grpc::Result<()>;

    fn get_consensus_params(&self, o: ::grpc::ServerHandlerContext, req: ::grpc::ServerRequestSingle<super::platform::GetConsensusParamsRequest>, resp: ::grpc::ServerResponseUnarySink<super::platform::GetConsensusParamsResponse>) -> ::grpc::Result<()>;
}

// client

pub struct PlatformClient {
    grpc_client: ::std::sync::Arc<::grpc::Client>,
}

impl ::grpc::ClientStub for PlatformClient {
    fn with_client(grpc_client: ::std::sync::Arc<::grpc::Client>) -> Self {
        PlatformClient {
            grpc_client: grpc_client,
        }
    }
}

impl PlatformClient {
    pub fn broadcast_state_transition(&self, o: ::grpc::RequestOptions, req: super::platform::BroadcastStateTransitionRequest) -> ::grpc::SingleResponse<super::platform::BroadcastStateTransitionResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/broadcastStateTransition"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_identity(&self, o: ::grpc::RequestOptions, req: super::platform::GetIdentityRequest) -> ::grpc::SingleResponse<super::platform::GetIdentityResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getIdentity"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_data_contract(&self, o: ::grpc::RequestOptions, req: super::platform::GetDataContractRequest) -> ::grpc::SingleResponse<super::platform::GetDataContractResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getDataContract"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_documents(&self, o: ::grpc::RequestOptions, req: super::platform::GetDocumentsRequest) -> ::grpc::SingleResponse<super::platform::GetDocumentsResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getDocuments"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_identities_by_public_key_hashes(&self, o: ::grpc::RequestOptions, req: super::platform::GetIdentitiesByPublicKeyHashesRequest) -> ::grpc::SingleResponse<super::platform::GetIdentitiesByPublicKeyHashesResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn wait_for_state_transition_result(&self, o: ::grpc::RequestOptions, req: super::platform::WaitForStateTransitionResultRequest) -> ::grpc::SingleResponse<super::platform::WaitForStateTransitionResultResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }

    pub fn get_consensus_params(&self, o: ::grpc::RequestOptions, req: super::platform::GetConsensusParamsRequest) -> ::grpc::SingleResponse<super::platform::GetConsensusParamsResponse> {
        let descriptor = ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
            name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getConsensusParams"),
            streaming: ::grpc::rt::GrpcStreaming::Unary,
            req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
            resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
        });
        self.grpc_client.call_unary(o, req, descriptor)
    }
}

// server

pub struct PlatformServer;


impl PlatformServer {
    pub fn new_service_def<H : Platform + 'static + Sync + Send + 'static>(handler: H) -> ::grpc::rt::ServerServiceDefinition {
        let handler_arc = ::std::sync::Arc::new(handler);
        ::grpc::rt::ServerServiceDefinition::new("/org.dash.platform.dapi.v0.Platform",
                                                 vec![
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/broadcastStateTransition"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).broadcast_state_transition(ctx, req, resp))
                                                         },
                                                     ),
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getIdentity"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_identity(ctx, req, resp))
                                                         },
                                                     ),
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getDataContract"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_data_contract(ctx, req, resp))
                                                         },
                                                     ),
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getDocuments"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_documents(ctx, req, resp))
                                                         },
                                                     ),
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getIdentitiesByPublicKeyHashes"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_identities_by_public_key_hashes(ctx, req, resp))
                                                         },
                                                     ),
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/waitForStateTransitionResult"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).wait_for_state_transition_result(ctx, req, resp))
                                                         },
                                                     ),
                                                     ::grpc::rt::ServerMethod::new(
                                                         ::grpc::rt::ArcOrStatic::Static(&::grpc::rt::MethodDescriptor {
                                                             name: ::grpc::rt::StringOrStatic::Static("/org.dash.platform.dapi.v0.Platform/getConsensusParams"),
                                                             streaming: ::grpc::rt::GrpcStreaming::Unary,
                                                             req_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                             resp_marshaller: ::grpc::rt::ArcOrStatic::Static(&::grpc_protobuf::MarshallerProtobuf),
                                                         }),
                                                         {
                                                             let handler_copy = handler_arc.clone();
                                                             ::grpc::rt::MethodHandlerUnary::new(move |ctx, req, resp| (*handler_copy).get_consensus_params(ctx, req, resp))
                                                         },
                                                     ),
                                                 ],
        )
    }
}
