package org.dash.platform.dapi.v0;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.42.1)",
    comments = "Source: core.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class CoreGrpc {

  private CoreGrpc() {}

  public static final String SERVICE_NAME = "org.dash.platform.dapi.v0.Core";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse> getGetBlockchainStatusMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getBlockchainStatus",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse> getGetBlockchainStatusMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse> getGetBlockchainStatusMethod;
    if ((getGetBlockchainStatusMethod = CoreGrpc.getGetBlockchainStatusMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getGetBlockchainStatusMethod = CoreGrpc.getGetBlockchainStatusMethod) == null) {
          CoreGrpc.getGetBlockchainStatusMethod = getGetBlockchainStatusMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getBlockchainStatus"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("getBlockchainStatus"))
              .build();
        }
      }
    }
    return getGetBlockchainStatusMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse> getGetMasternodeStatusMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getMasternodeStatus",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse> getGetMasternodeStatusMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse> getGetMasternodeStatusMethod;
    if ((getGetMasternodeStatusMethod = CoreGrpc.getGetMasternodeStatusMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getGetMasternodeStatusMethod = CoreGrpc.getGetMasternodeStatusMethod) == null) {
          CoreGrpc.getGetMasternodeStatusMethod = getGetMasternodeStatusMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getMasternodeStatus"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("getMasternodeStatus"))
              .build();
        }
      }
    }
    return getGetMasternodeStatusMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse> getGetBlockMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getBlock",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse> getGetBlockMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse> getGetBlockMethod;
    if ((getGetBlockMethod = CoreGrpc.getGetBlockMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getGetBlockMethod = CoreGrpc.getGetBlockMethod) == null) {
          CoreGrpc.getGetBlockMethod = getGetBlockMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getBlock"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("getBlock"))
              .build();
        }
      }
    }
    return getGetBlockMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse> getGetBestBlockHeightMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getBestBlockHeight",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse> getGetBestBlockHeightMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse> getGetBestBlockHeightMethod;
    if ((getGetBestBlockHeightMethod = CoreGrpc.getGetBestBlockHeightMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getGetBestBlockHeightMethod = CoreGrpc.getGetBestBlockHeightMethod) == null) {
          CoreGrpc.getGetBestBlockHeightMethod = getGetBestBlockHeightMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getBestBlockHeight"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("getBestBlockHeight"))
              .build();
        }
      }
    }
    return getGetBestBlockHeightMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse> getBroadcastTransactionMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "broadcastTransaction",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse> getBroadcastTransactionMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest, org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse> getBroadcastTransactionMethod;
    if ((getBroadcastTransactionMethod = CoreGrpc.getBroadcastTransactionMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getBroadcastTransactionMethod = CoreGrpc.getBroadcastTransactionMethod) == null) {
          CoreGrpc.getBroadcastTransactionMethod = getBroadcastTransactionMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest, org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "broadcastTransaction"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("broadcastTransaction"))
              .build();
        }
      }
    }
    return getBroadcastTransactionMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse> getGetTransactionMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getTransaction",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse> getGetTransactionMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse> getGetTransactionMethod;
    if ((getGetTransactionMethod = CoreGrpc.getGetTransactionMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getGetTransactionMethod = CoreGrpc.getGetTransactionMethod) == null) {
          CoreGrpc.getGetTransactionMethod = getGetTransactionMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getTransaction"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("getTransaction"))
              .build();
        }
      }
    }
    return getGetTransactionMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse> getGetEstimatedTransactionFeeMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getEstimatedTransactionFee",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse> getGetEstimatedTransactionFeeMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse> getGetEstimatedTransactionFeeMethod;
    if ((getGetEstimatedTransactionFeeMethod = CoreGrpc.getGetEstimatedTransactionFeeMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getGetEstimatedTransactionFeeMethod = CoreGrpc.getGetEstimatedTransactionFeeMethod) == null) {
          CoreGrpc.getGetEstimatedTransactionFeeMethod = getGetEstimatedTransactionFeeMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest, org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getEstimatedTransactionFee"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("getEstimatedTransactionFee"))
              .build();
        }
      }
    }
    return getGetEstimatedTransactionFeeMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> getSubscribeToBlockHeadersWithChainLocksMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "subscribeToBlockHeadersWithChainLocks",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> getSubscribeToBlockHeadersWithChainLocksMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest, org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> getSubscribeToBlockHeadersWithChainLocksMethod;
    if ((getSubscribeToBlockHeadersWithChainLocksMethod = CoreGrpc.getSubscribeToBlockHeadersWithChainLocksMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getSubscribeToBlockHeadersWithChainLocksMethod = CoreGrpc.getSubscribeToBlockHeadersWithChainLocksMethod) == null) {
          CoreGrpc.getSubscribeToBlockHeadersWithChainLocksMethod = getSubscribeToBlockHeadersWithChainLocksMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest, org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "subscribeToBlockHeadersWithChainLocks"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("subscribeToBlockHeadersWithChainLocks"))
              .build();
        }
      }
    }
    return getSubscribeToBlockHeadersWithChainLocksMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse> getSubscribeToTransactionsWithProofsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "subscribeToTransactionsWithProofs",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse> getSubscribeToTransactionsWithProofsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest, org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse> getSubscribeToTransactionsWithProofsMethod;
    if ((getSubscribeToTransactionsWithProofsMethod = CoreGrpc.getSubscribeToTransactionsWithProofsMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getSubscribeToTransactionsWithProofsMethod = CoreGrpc.getSubscribeToTransactionsWithProofsMethod) == null) {
          CoreGrpc.getSubscribeToTransactionsWithProofsMethod = getSubscribeToTransactionsWithProofsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest, org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "subscribeToTransactionsWithProofs"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("subscribeToTransactionsWithProofs"))
              .build();
        }
      }
    }
    return getSubscribeToTransactionsWithProofsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse> getSubscribeToMasternodeListMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "subscribeToMasternodeList",
      requestType = org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest.class,
      responseType = org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest,
      org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse> getSubscribeToMasternodeListMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest, org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse> getSubscribeToMasternodeListMethod;
    if ((getSubscribeToMasternodeListMethod = CoreGrpc.getSubscribeToMasternodeListMethod) == null) {
      synchronized (CoreGrpc.class) {
        if ((getSubscribeToMasternodeListMethod = CoreGrpc.getSubscribeToMasternodeListMethod) == null) {
          CoreGrpc.getSubscribeToMasternodeListMethod = getSubscribeToMasternodeListMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest, org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.SERVER_STREAMING)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "subscribeToMasternodeList"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse.getDefaultInstance()))
              .setSchemaDescriptor(new CoreMethodDescriptorSupplier("subscribeToMasternodeList"))
              .build();
        }
      }
    }
    return getSubscribeToMasternodeListMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static CoreStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<CoreStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<CoreStub>() {
        @java.lang.Override
        public CoreStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new CoreStub(channel, callOptions);
        }
      };
    return CoreStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static CoreBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<CoreBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<CoreBlockingStub>() {
        @java.lang.Override
        public CoreBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new CoreBlockingStub(channel, callOptions);
        }
      };
    return CoreBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static CoreFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<CoreFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<CoreFutureStub>() {
        @java.lang.Override
        public CoreFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new CoreFutureStub(channel, callOptions);
        }
      };
    return CoreFutureStub.newStub(factory, channel);
  }

  /**
   */
  public static abstract class CoreImplBase implements io.grpc.BindableService {

    /**
     */
    public void getBlockchainStatus(org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetBlockchainStatusMethod(), responseObserver);
    }

    /**
     */
    public void getMasternodeStatus(org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetMasternodeStatusMethod(), responseObserver);
    }

    /**
     */
    public void getBlock(org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetBlockMethod(), responseObserver);
    }

    /**
     */
    public void getBestBlockHeight(org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetBestBlockHeightMethod(), responseObserver);
    }

    /**
     */
    public void broadcastTransaction(org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getBroadcastTransactionMethod(), responseObserver);
    }

    /**
     */
    public void getTransaction(org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTransactionMethod(), responseObserver);
    }

    /**
     */
    public void getEstimatedTransactionFee(org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetEstimatedTransactionFeeMethod(), responseObserver);
    }

    /**
     */
    public void subscribeToBlockHeadersWithChainLocks(org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getSubscribeToBlockHeadersWithChainLocksMethod(), responseObserver);
    }

    /**
     */
    public void subscribeToTransactionsWithProofs(org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getSubscribeToTransactionsWithProofsMethod(), responseObserver);
    }

    /**
     */
    public void subscribeToMasternodeList(org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getSubscribeToMasternodeListMethod(), responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            getGetBlockchainStatusMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse>(
                  this, METHODID_GET_BLOCKCHAIN_STATUS)))
          .addMethod(
            getGetMasternodeStatusMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse>(
                  this, METHODID_GET_MASTERNODE_STATUS)))
          .addMethod(
            getGetBlockMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse>(
                  this, METHODID_GET_BLOCK)))
          .addMethod(
            getGetBestBlockHeightMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse>(
                  this, METHODID_GET_BEST_BLOCK_HEIGHT)))
          .addMethod(
            getBroadcastTransactionMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse>(
                  this, METHODID_BROADCAST_TRANSACTION)))
          .addMethod(
            getGetTransactionMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse>(
                  this, METHODID_GET_TRANSACTION)))
          .addMethod(
            getGetEstimatedTransactionFeeMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse>(
                  this, METHODID_GET_ESTIMATED_TRANSACTION_FEE)))
          .addMethod(
            getSubscribeToBlockHeadersWithChainLocksMethod(),
            io.grpc.stub.ServerCalls.asyncServerStreamingCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse>(
                  this, METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS)))
          .addMethod(
            getSubscribeToTransactionsWithProofsMethod(),
            io.grpc.stub.ServerCalls.asyncServerStreamingCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse>(
                  this, METHODID_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS)))
          .addMethod(
            getSubscribeToMasternodeListMethod(),
            io.grpc.stub.ServerCalls.asyncServerStreamingCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest,
                org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse>(
                  this, METHODID_SUBSCRIBE_TO_MASTERNODE_LIST)))
          .build();
    }
  }

  /**
   */
  public static final class CoreStub extends io.grpc.stub.AbstractAsyncStub<CoreStub> {
    private CoreStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected CoreStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new CoreStub(channel, callOptions);
    }

    /**
     */
    public void getBlockchainStatus(org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetBlockchainStatusMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getMasternodeStatus(org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetMasternodeStatusMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getBlock(org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetBlockMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getBestBlockHeight(org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetBestBlockHeightMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void broadcastTransaction(org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getBroadcastTransactionMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTransaction(org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTransactionMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getEstimatedTransactionFee(org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetEstimatedTransactionFeeMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void subscribeToBlockHeadersWithChainLocks(org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncServerStreamingCall(
          getChannel().newCall(getSubscribeToBlockHeadersWithChainLocksMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void subscribeToTransactionsWithProofs(org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncServerStreamingCall(
          getChannel().newCall(getSubscribeToTransactionsWithProofsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void subscribeToMasternodeList(org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncServerStreamingCall(
          getChannel().newCall(getSubscribeToMasternodeListMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class CoreBlockingStub extends io.grpc.stub.AbstractBlockingStub<CoreBlockingStub> {
    private CoreBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected CoreBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new CoreBlockingStub(channel, callOptions);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse getBlockchainStatus(org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetBlockchainStatusMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse getMasternodeStatus(org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetMasternodeStatusMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse getBlock(org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetBlockMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse getBestBlockHeight(org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetBestBlockHeightMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse broadcastTransaction(org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getBroadcastTransactionMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse getTransaction(org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTransactionMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse getEstimatedTransactionFee(org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetEstimatedTransactionFeeMethod(), getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse> subscribeToBlockHeadersWithChainLocks(
        org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest request) {
      return io.grpc.stub.ClientCalls.blockingServerStreamingCall(
          getChannel(), getSubscribeToBlockHeadersWithChainLocksMethod(), getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse> subscribeToTransactionsWithProofs(
        org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest request) {
      return io.grpc.stub.ClientCalls.blockingServerStreamingCall(
          getChannel(), getSubscribeToTransactionsWithProofsMethod(), getCallOptions(), request);
    }

    /**
     */
    public java.util.Iterator<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse> subscribeToMasternodeList(
        org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest request) {
      return io.grpc.stub.ClientCalls.blockingServerStreamingCall(
          getChannel(), getSubscribeToMasternodeListMethod(), getCallOptions(), request);
    }
  }

  /**
   */
  public static final class CoreFutureStub extends io.grpc.stub.AbstractFutureStub<CoreFutureStub> {
    private CoreFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected CoreFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new CoreFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse> getBlockchainStatus(
        org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetBlockchainStatusMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse> getMasternodeStatus(
        org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetMasternodeStatusMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse> getBlock(
        org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetBlockMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse> getBestBlockHeight(
        org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetBestBlockHeightMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse> broadcastTransaction(
        org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getBroadcastTransactionMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse> getTransaction(
        org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTransactionMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse> getEstimatedTransactionFee(
        org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetEstimatedTransactionFeeMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_GET_BLOCKCHAIN_STATUS = 0;
  private static final int METHODID_GET_MASTERNODE_STATUS = 1;
  private static final int METHODID_GET_BLOCK = 2;
  private static final int METHODID_GET_BEST_BLOCK_HEIGHT = 3;
  private static final int METHODID_BROADCAST_TRANSACTION = 4;
  private static final int METHODID_GET_TRANSACTION = 5;
  private static final int METHODID_GET_ESTIMATED_TRANSACTION_FEE = 6;
  private static final int METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS = 7;
  private static final int METHODID_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS = 8;
  private static final int METHODID_SUBSCRIBE_TO_MASTERNODE_LIST = 9;

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
        case METHODID_GET_BLOCKCHAIN_STATUS:
          serviceImpl.getBlockchainStatus((org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockchainStatusResponse>) responseObserver);
          break;
        case METHODID_GET_MASTERNODE_STATUS:
          serviceImpl.getMasternodeStatus((org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetMasternodeStatusResponse>) responseObserver);
          break;
        case METHODID_GET_BLOCK:
          serviceImpl.getBlock((org.dash.platform.dapi.v0.CoreOuterClass.GetBlockRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBlockResponse>) responseObserver);
          break;
        case METHODID_GET_BEST_BLOCK_HEIGHT:
          serviceImpl.getBestBlockHeight((org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetBestBlockHeightResponse>) responseObserver);
          break;
        case METHODID_BROADCAST_TRANSACTION:
          serviceImpl.broadcastTransaction((org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BroadcastTransactionResponse>) responseObserver);
          break;
        case METHODID_GET_TRANSACTION:
          serviceImpl.getTransaction((org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetTransactionResponse>) responseObserver);
          break;
        case METHODID_GET_ESTIMATED_TRANSACTION_FEE:
          serviceImpl.getEstimatedTransactionFee((org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.GetEstimatedTransactionFeeResponse>) responseObserver);
          break;
        case METHODID_SUBSCRIBE_TO_BLOCK_HEADERS_WITH_CHAIN_LOCKS:
          serviceImpl.subscribeToBlockHeadersWithChainLocks((org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.BlockHeadersWithChainLocksResponse>) responseObserver);
          break;
        case METHODID_SUBSCRIBE_TO_TRANSACTIONS_WITH_PROOFS:
          serviceImpl.subscribeToTransactionsWithProofs((org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.TransactionsWithProofsResponse>) responseObserver);
          break;
        case METHODID_SUBSCRIBE_TO_MASTERNODE_LIST:
          serviceImpl.subscribeToMasternodeList((org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.CoreOuterClass.MasternodeListResponse>) responseObserver);
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

  private static abstract class CoreBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    CoreBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return org.dash.platform.dapi.v0.CoreOuterClass.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("Core");
    }
  }

  private static final class CoreFileDescriptorSupplier
      extends CoreBaseDescriptorSupplier {
    CoreFileDescriptorSupplier() {}
  }

  private static final class CoreMethodDescriptorSupplier
      extends CoreBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final String methodName;

    CoreMethodDescriptorSupplier(String methodName) {
      this.methodName = methodName;
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.MethodDescriptor getMethodDescriptor() {
      return getServiceDescriptor().findMethodByName(methodName);
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
              .setSchemaDescriptor(new CoreFileDescriptorSupplier())
              .addMethod(getGetBlockchainStatusMethod())
              .addMethod(getGetMasternodeStatusMethod())
              .addMethod(getGetBlockMethod())
              .addMethod(getGetBestBlockHeightMethod())
              .addMethod(getBroadcastTransactionMethod())
              .addMethod(getGetTransactionMethod())
              .addMethod(getGetEstimatedTransactionFeeMethod())
              .addMethod(getSubscribeToBlockHeadersWithChainLocksMethod())
              .addMethod(getSubscribeToTransactionsWithProofsMethod())
              .addMethod(getSubscribeToMasternodeListMethod())
              .build();
        }
      }
    }
    return result;
  }
}
