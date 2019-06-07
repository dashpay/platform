package org.dash.platform.dapi;

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

  public static final String SERVICE_NAME = "org.dash.platform.dapi.Core";

  // Static method descriptors that strictly reflect the proto.
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest,
      org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse> METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest, org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.Core", "subscribeToBlockHeadersWithChainLocks"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse.getDefaultInstance()))
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
    public void subscribeToBlockHeadersWithChainLocks(org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS, responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS,
            asyncServerStreamingCall(
              new MethodHandlers<
                org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest,
                org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse>(
                  this, METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS)))
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
    public void subscribeToBlockHeadersWithChainLocks(org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse> responseObserver) {
      asyncServerStreamingCall(
          getChannel().newCall(METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS, getCallOptions()), request, responseObserver);
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
    public java.util.Iterator<org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse> subscribeToBlockHeadersWithChainLocks(
        org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest request) {
      return blockingServerStreamingCall(
          getChannel(), METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS, getCallOptions(), request);
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
  }

  private static final int METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS = 0;

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
        case METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS:
          serviceImpl.subscribeToBlockHeadersWithChainLocks((org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.CoreOuterClass.BlockHeadersWithChainLocksResponse>) responseObserver);
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
      return org.dash.platform.dapi.CoreOuterClass.getDescriptor();
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
              .addMethod(METHOD_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS)
              .build();
        }
      }
    }
    return result;
  }
}
