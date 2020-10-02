package org.dash.platform.dapi.v0;

import static io.grpc.stub.ClientCalls.asyncUnaryCall;
import static io.grpc.stub.ClientCalls.asyncServerStreamingCall;
import static io.grpc.stub.ClientCalls.asyncClientStreamingCall;
import static io.grpc.stub.ClientCalls.asyncBidiStreamingCall;
import static io.grpc.stub.ClientCalls.blockingUnaryCall;
import static io.grpc.stub.ClientCalls.blockingServerStreamingCall;
import static io.grpc.stub.ClientCalls.futureUnaryCall;
import static io.grpc.MethodDescriptor.generateFullMethodName;
import static io.grpc.stub.ServerCalls.asyncUnaryCall;
import static io.grpc.stub.ServerCalls.asyncServerStreamingCall;
import static io.grpc.stub.ServerCalls.asyncClientStreamingCall;
import static io.grpc.stub.ServerCalls.asyncBidiStreamingCall;
import static io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall;
import static io.grpc.stub.ServerCalls.asyncUnimplementedStreamingCall;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler",
    comments = "Source: platform.proto")
public final class PlatformGrpc {

  private PlatformGrpc() {}

  public static final String SERVICE_NAME = "org.dash.platform.dapi.v0.Platform";

  // Static method descriptors that strictly reflect the proto.
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> METHOD_BROADCAST_STATE_TRANSITION =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest, org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "broadcastStateTransition"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> METHOD_GET_IDENTITY =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getIdentity"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> METHOD_GET_DATA_CONTRACT =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getDataContract"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> METHOD_GET_DOCUMENTS =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getDocuments"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse> METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getIdentityByFirstPublicKey"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse> METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getIdentityIdByFirstPublicKey"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse> METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getIdentitiesByPublicKeyHashes"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse> METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Platform", "getIdentityIdsByPublicKeyHashes"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse.getDefaultInstance()))
          .build();

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static PlatformStub newStub(io.grpc.Channel channel) {
    return new PlatformStub(channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static PlatformBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    return new PlatformBlockingStub(channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static PlatformFutureStub newFutureStub(
      io.grpc.Channel channel) {
    return new PlatformFutureStub(channel);
  }

  /**
   */
  public static abstract class PlatformImplBase implements io.grpc.BindableService {

    /**
     */
    public void broadcastStateTransition(org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_BROADCAST_STATE_TRANSITION, responseObserver);
    }

    /**
     */
    public void getIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_IDENTITY, responseObserver);
    }

    /**
     */
    public void getDataContract(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_DATA_CONTRACT, responseObserver);
    }

    /**
     */
    public void getDocuments(org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_DOCUMENTS, responseObserver);
    }

    /**
     */
    public void getIdentityByFirstPublicKey(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY, responseObserver);
    }

    /**
     */
    public void getIdentityIdByFirstPublicKey(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY, responseObserver);
    }

    /**
     */
    public void getIdentitiesByPublicKeyHashes(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES, responseObserver);
    }

    /**
     */
    public void getIdentityIdsByPublicKeyHashes(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES, responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            METHOD_BROADCAST_STATE_TRANSITION,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse>(
                  this, METHODID_BROADCAST_STATE_TRANSITION)))
          .addMethod(
            METHOD_GET_IDENTITY,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse>(
                  this, METHODID_GET_IDENTITY)))
          .addMethod(
            METHOD_GET_DATA_CONTRACT,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse>(
                  this, METHODID_GET_DATA_CONTRACT)))
          .addMethod(
            METHOD_GET_DOCUMENTS,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse>(
                  this, METHODID_GET_DOCUMENTS)))
          .addMethod(
            METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse>(
                  this, METHODID_GET_IDENTITY_BY_FIRST_PUBLIC_KEY)))
          .addMethod(
            METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse>(
                  this, METHODID_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY)))
          .addMethod(
            METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse>(
                  this, METHODID_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES)))
          .addMethod(
            METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse>(
                  this, METHODID_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES)))
          .build();
    }
  }

  /**
   */
  public static final class PlatformStub extends io.grpc.stub.AbstractStub<PlatformStub> {
    private PlatformStub(io.grpc.Channel channel) {
      super(channel);
    }

    private PlatformStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PlatformStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new PlatformStub(channel, callOptions);
    }

    /**
     */
    public void broadcastStateTransition(org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_BROADCAST_STATE_TRANSITION, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getDataContract(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_DATA_CONTRACT, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getDocuments(org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_DOCUMENTS, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityByFirstPublicKey(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityIdByFirstPublicKey(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentitiesByPublicKeyHashes(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityIdsByPublicKeyHashes(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES, getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class PlatformBlockingStub extends io.grpc.stub.AbstractStub<PlatformBlockingStub> {
    private PlatformBlockingStub(io.grpc.Channel channel) {
      super(channel);
    }

    private PlatformBlockingStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PlatformBlockingStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new PlatformBlockingStub(channel, callOptions);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse broadcastStateTransition(org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_BROADCAST_STATE_TRANSITION, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse getIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_IDENTITY, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse getDataContract(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_DATA_CONTRACT, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse getDocuments(org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_DOCUMENTS, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse getIdentityByFirstPublicKey(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse getIdentityIdByFirstPublicKey(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse getIdentitiesByPublicKeyHashes(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse getIdentityIdsByPublicKeyHashes(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES, getCallOptions(), request);
    }
  }

  /**
   */
  public static final class PlatformFutureStub extends io.grpc.stub.AbstractStub<PlatformFutureStub> {
    private PlatformFutureStub(io.grpc.Channel channel) {
      super(channel);
    }

    private PlatformFutureStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PlatformFutureStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new PlatformFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> broadcastStateTransition(
        org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_BROADCAST_STATE_TRANSITION, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> getIdentity(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> getDataContract(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_DATA_CONTRACT, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> getDocuments(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_DOCUMENTS, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse> getIdentityByFirstPublicKey(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse> getIdentityIdByFirstPublicKey(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse> getIdentitiesByPublicKeyHashes(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse> getIdentityIdsByPublicKeyHashes(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES, getCallOptions()), request);
    }
  }

  private static final int METHODID_BROADCAST_STATE_TRANSITION = 0;
  private static final int METHODID_GET_IDENTITY = 1;
  private static final int METHODID_GET_DATA_CONTRACT = 2;
  private static final int METHODID_GET_DOCUMENTS = 3;
  private static final int METHODID_GET_IDENTITY_BY_FIRST_PUBLIC_KEY = 4;
  private static final int METHODID_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY = 5;
  private static final int METHODID_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES = 6;
  private static final int METHODID_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES = 7;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final PlatformImplBase serviceImpl;
    private final int methodId;

    MethodHandlers(PlatformImplBase serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_BROADCAST_STATE_TRANSITION:
          serviceImpl.broadcastStateTransition((org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY:
          serviceImpl.getIdentity((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse>) responseObserver);
          break;
        case METHODID_GET_DATA_CONTRACT:
          serviceImpl.getDataContract((org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse>) responseObserver);
          break;
        case METHODID_GET_DOCUMENTS:
          serviceImpl.getDocuments((org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_BY_FIRST_PUBLIC_KEY:
          serviceImpl.getIdentityByFirstPublicKey((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByFirstPublicKeyResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY:
          serviceImpl.getIdentityIdByFirstPublicKey((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdByFirstPublicKeyResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES:
          serviceImpl.getIdentitiesByPublicKeyHashes((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesByPublicKeyHashesResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES:
          serviceImpl.getIdentityIdsByPublicKeyHashes((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityIdsByPublicKeyHashesResponse>) responseObserver);
          break;
        default:
          throw new AssertionError();
      }
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public io.grpc.stub.StreamObserver<Req> invoke(
        io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        default:
          throw new AssertionError();
      }
    }
  }

  private static final class PlatformDescriptorSupplier implements io.grpc.protobuf.ProtoFileDescriptorSupplier {
    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return org.dash.platform.dapi.v0.PlatformOuterClass.getDescriptor();
    }
  }

  private static volatile io.grpc.ServiceDescriptor serviceDescriptor;

  public static io.grpc.ServiceDescriptor getServiceDescriptor() {
    io.grpc.ServiceDescriptor result = serviceDescriptor;
    if (result == null) {
      synchronized (PlatformGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new PlatformDescriptorSupplier())
              .addMethod(METHOD_BROADCAST_STATE_TRANSITION)
              .addMethod(METHOD_GET_IDENTITY)
              .addMethod(METHOD_GET_DATA_CONTRACT)
              .addMethod(METHOD_GET_DOCUMENTS)
              .addMethod(METHOD_GET_IDENTITY_BY_FIRST_PUBLIC_KEY)
              .addMethod(METHOD_GET_IDENTITY_ID_BY_FIRST_PUBLIC_KEY)
              .addMethod(METHOD_GET_IDENTITIES_BY_PUBLIC_KEY_HASHES)
              .addMethod(METHOD_GET_IDENTITY_IDS_BY_PUBLIC_KEY_HASHES)
              .build();
        }
      }
    }
    return result;
  }
}
