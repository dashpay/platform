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
    comments = "Source: core.proto")
public final class CoreGrpc {

  private CoreGrpc() {}

  public static final String SERVICE_NAME = "org.dash.platform.dapi.v0.Core";

  // Static method descriptors that strictly reflect the proto.
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse> METHOD_GET_LAST_USER_STATE_TRANSITION_HASH =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest, org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Core", "getLastUserStateTransitionHash"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest, org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Core", "subscribeToBlockHeadersWithChainLocks"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse> METHOD_UPDATE_STATE =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest, org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Core", "updateState"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse.getDefaultInstance()))
          .build();
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse> METHOD_FETCH_IDENTITY =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest, org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.v0.Core", "fetchIdentity"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse.getDefaultInstance()))
          .build();

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static CoreStub newStub(io.grpc.Channel channel) {
    return new CoreStub(channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static CoreBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    return new CoreBlockingStub(channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static CoreFutureStub newFutureStub(
      io.grpc.Channel channel) {
    return new CoreFutureStub(channel);
  }

  /**
   */
  public static abstract class CoreImplBase implements io.grpc.BindableService {

    /**
     */
    public void getLastUserStateTransitionHash(org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_GET_LAST_USER_STATE_TRANSITION_HASH, responseObserver);
    }

    /**
     */
    public void subscribeToBlockHeadersWithChainLocks(org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS, responseObserver);
    }

    /**
     */
    public void updateState(org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_UPDATE_STATE, responseObserver);
    }

    /**
     */
    public void fetchIdentity(org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_FETCH_IDENTITY, responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            METHOD_GET_LAST_USER_STATE_TRANSITION_HASH,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse>(
                  this, METHODID_GET_LAST_USER_STATE_TRANSITION_HASH)))
          .addMethod(
            METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS,
            asyncServerStreamingCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse>(
                  this, METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS)))
          .addMethod(
            METHOD_UPDATE_STATE,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse>(
                  this, METHODID_UPDATE_STATE)))
          .addMethod(
            METHOD_FETCH_IDENTITY,
            asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse>(
                  this, METHODID_FETCH_IDENTITY)))
          .build();
    }
  }

  /**
   */
  public static final class CoreStub extends io.grpc.stub.AbstractStub<CoreStub> {
    private CoreStub(io.grpc.Channel channel) {
      super(channel);
    }

    private CoreStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected CoreStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new CoreStub(channel, callOptions);
    }

    /**
     */
    public void getLastUserStateTransitionHash(org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_GET_LAST_USER_STATE_TRANSITION_HASH, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void subscribeToBlockHeadersWithChainLocks(org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> responseObserver) {
      asyncServerStreamingCall(
          getChannel().newCall(METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void updateState(org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_UPDATE_STATE, getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void fetchIdentity(org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse> responseObserver) {
      asyncUnaryCall(
          getChannel().newCall(METHOD_FETCH_IDENTITY, getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class CoreBlockingStub extends io.grpc.stub.AbstractStub<CoreBlockingStub> {
    private CoreBlockingStub(io.grpc.Channel channel) {
      super(channel);
    }

    private CoreBlockingStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected CoreBlockingStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new CoreBlockingStub(channel, callOptions);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse getLastUserStateTransitionHash(org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_GET_LAST_USER_STATE_TRANSITION_HASH, getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> subscribeToBlockHeadersWithChainLocks(
        org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest request) {
      return blockingServerStreamingCall(
          getChannel(), METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse updateState(org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_UPDATE_STATE, getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse fetchIdentity(org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest request) {
      return blockingUnaryCall(
          getChannel(), METHOD_FETCH_IDENTITY, getCallOptions(), request);
    }
  }

  /**
   */
  public static final class CoreFutureStub extends io.grpc.stub.AbstractStub<CoreFutureStub> {
    private CoreFutureStub(io.grpc.Channel channel) {
      super(channel);
    }

    private CoreFutureStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected CoreFutureStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new CoreFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse> getLastUserStateTransitionHash(
        org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_GET_LAST_USER_STATE_TRANSITION_HASH, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse> updateState(
        org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_UPDATE_STATE, getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse> fetchIdentity(
        org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest request) {
      return futureUnaryCall(
          getChannel().newCall(METHOD_FETCH_IDENTITY, getCallOptions()), request);
    }
  }

  private static final int METHODID_GET_LAST_USER_STATE_TRANSITION_HASH = 0;
  private static final int METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS = 1;
  private static final int METHODID_UPDATE_STATE = 2;
  private static final int METHODID_FETCH_IDENTITY = 3;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final CoreImplBase serviceImpl;
    private final int methodId;

    MethodHandlers(CoreImplBase serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_GET_LAST_USER_STATE_TRANSITION_HASH:
          serviceImpl.getLastUserStateTransitionHash((org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.LastUserStateTransitionHashResponse>) responseObserver);
          break;
        case METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS:
          serviceImpl.subscribeToBlockHeadersWithChainLocks((org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse>) responseObserver);
          break;
        case METHODID_UPDATE_STATE:
          serviceImpl.updateState((org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.UpdateStateResponse>) responseObserver);
          break;
        case METHODID_FETCH_IDENTITY:
          serviceImpl.fetchIdentity((org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.FetchIdentityResponse>) responseObserver);
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

  private static final class CoreDescriptorSupplier implements io.grpc.protobuf.ProtoFileDescriptorSupplier {
    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return org.dash.platform.dapi.v0.CoreOuterClass.getDescriptor();
    }
  }

  private static volatile io.grpc.ServiceDescriptor serviceDescriptor;

  public static io.grpc.ServiceDescriptor getServiceDescriptor() {
    io.grpc.ServiceDescriptor result = serviceDescriptor;
    if (result == null) {
      synchronized (CoreGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new CoreDescriptorSupplier())
              .addMethod(METHOD_GET_LAST_USER_STATE_TRANSITION_HASH)
              .addMethod(METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS)
              .addMethod(METHOD_UPDATE_STATE)
              .addMethod(METHOD_FETCH_IDENTITY)
              .build();
        }
      }
    }
    return result;
  }
}
