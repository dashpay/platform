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
    comments = "Source: transactions_filter_stream.proto")
public final class TransactionsFilterStreamGrpc {

  private TransactionsFilterStreamGrpc() {}

  public static final String SERVICE_NAME = "org.dash.platform.dapi.TransactionsFilterStream";

  // Static method descriptors that strictly reflect the proto.
  @io.grpc.ExperimentalApi("https://github.com/grpc/grpc-java/issues/1901")
  public static final io.grpc.MethodDescriptor<org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest,
      org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse> METHOD_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS =
      io.grpc.MethodDescriptor.<org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest, org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse>newBuilder()
          .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
          .setFullMethodName(generateFullMethodName(
              "org.dash.platform.dapi.TransactionsFilterStream", "subscribeToTransactionsWithProofs"))
          .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest.getDefaultInstance()))
          .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
              org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse.getDefaultInstance()))
          .build();

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static TransactionsFilterStreamStub newStub(io.grpc.Channel channel) {
    return new TransactionsFilterStreamStub(channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static TransactionsFilterStreamBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    return new TransactionsFilterStreamBlockingStub(channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static TransactionsFilterStreamFutureStub newFutureStub(
      io.grpc.Channel channel) {
    return new TransactionsFilterStreamFutureStub(channel);
  }

  /**
   */
  public static abstract class TransactionsFilterStreamImplBase implements io.grpc.BindableService {

    /**
     */
    public void subscribeToTransactionsWithProofs(org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse> responseObserver) {
      asyncUnimplementedUnaryCall(METHOD_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS, responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            METHOD_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS,
            asyncServerStreamingCall(
              new MethodHandlers<
                org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest,
                org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse>(
                  this, METHODID_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS)))
          .build();
    }
  }

  /**
   */
  public static final class TransactionsFilterStreamStub extends io.grpc.stub.AbstractStub<TransactionsFilterStreamStub> {
    private TransactionsFilterStreamStub(io.grpc.Channel channel) {
      super(channel);
    }

    private TransactionsFilterStreamStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected TransactionsFilterStreamStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new TransactionsFilterStreamStub(channel, callOptions);
    }

    /**
     */
    public void subscribeToTransactionsWithProofs(org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse> responseObserver) {
      asyncServerStreamingCall(
          getChannel().newCall(METHOD_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS, getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class TransactionsFilterStreamBlockingStub extends io.grpc.stub.AbstractStub<TransactionsFilterStreamBlockingStub> {
    private TransactionsFilterStreamBlockingStub(io.grpc.Channel channel) {
      super(channel);
    }

    private TransactionsFilterStreamBlockingStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected TransactionsFilterStreamBlockingStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new TransactionsFilterStreamBlockingStub(channel, callOptions);
    }

    /**
     */
    public java.util.Iterator<org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse> subscribeToTransactionsWithProofs(
        org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest request) {
      return blockingServerStreamingCall(
          getChannel(), METHOD_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS, getCallOptions(), request);
    }
  }

  /**
   */
  public static final class TransactionsFilterStreamFutureStub extends io.grpc.stub.AbstractStub<TransactionsFilterStreamFutureStub> {
    private TransactionsFilterStreamFutureStub(io.grpc.Channel channel) {
      super(channel);
    }

    private TransactionsFilterStreamFutureStub(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected TransactionsFilterStreamFutureStub build(io.grpc.Channel channel,
        io.grpc.CallOptions callOptions) {
      return new TransactionsFilterStreamFutureStub(channel, callOptions);
    }
  }

  private static final int METHODID_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS = 0;

  private static final class MethodHandlers<Req, Resp> implements
      io.grpc.stub.ServerCalls.UnaryMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ServerStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.ClientStreamingMethod<Req, Resp>,
      io.grpc.stub.ServerCalls.BidiStreamingMethod<Req, Resp> {
    private final TransactionsFilterStreamImplBase serviceImpl;
    private final int methodId;

    MethodHandlers(TransactionsFilterStreamImplBase serviceImpl, int methodId) {
      this.serviceImpl = serviceImpl;
      this.methodId = methodId;
    }

    @java.lang.Override
    @java.lang.SuppressWarnings("unchecked")
    public void invoke(Req request, io.grpc.stub.StreamObserver<Resp> responseObserver) {
      switch (methodId) {
        case METHODID_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS:
          serviceImpl.subscribeToTransactionsWithProofs((org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.TransactionsFilterStreamOuterClass.TransactionsWithProofsResponse>) responseObserver);
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

  private static final class TransactionsFilterStreamDescriptorSupplier implements io.grpc.protobuf.ProtoFileDescriptorSupplier {
    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return org.dash.platform.dapi.TransactionsFilterStreamOuterClass.getDescriptor();
    }
  }

  private static volatile io.grpc.ServiceDescriptor serviceDescriptor;

  public static io.grpc.ServiceDescriptor getServiceDescriptor() {
    io.grpc.ServiceDescriptor result = serviceDescriptor;
    if (result == null) {
      synchronized (TransactionsFilterStreamGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new TransactionsFilterStreamDescriptorSupplier())
              .addMethod(METHOD_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS)
              .build();
        }
      }
    }
    return result;
  }
}
