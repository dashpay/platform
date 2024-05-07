package org.dash.platform.dapi.v0;

import static io.grpc.MethodDescriptor.generateFullMethodName;

/**
 */
@javax.annotation.Generated(
    value = "by gRPC proto compiler (version 1.42.1)",
    comments = "Source: platform.proto")
@io.grpc.stub.annotations.GrpcGenerated
public final class PlatformGrpc {

  private PlatformGrpc() {}

  public static final String SERVICE_NAME = "org.dash.platform.dapi.v0.Platform";

  // Static method descriptors that strictly reflect the proto.
  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> getBroadcastStateTransitionMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "broadcastStateTransition",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> getBroadcastStateTransitionMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest, org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> getBroadcastStateTransitionMethod;
    if ((getBroadcastStateTransitionMethod = PlatformGrpc.getBroadcastStateTransitionMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getBroadcastStateTransitionMethod = PlatformGrpc.getBroadcastStateTransitionMethod) == null) {
          PlatformGrpc.getBroadcastStateTransitionMethod = getBroadcastStateTransitionMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest, org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "broadcastStateTransition"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("broadcastStateTransition"))
              .build();
        }
      }
    }
    return getBroadcastStateTransitionMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> getGetIdentityMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentity",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> getGetIdentityMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> getGetIdentityMethod;
    if ((getGetIdentityMethod = PlatformGrpc.getGetIdentityMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityMethod = PlatformGrpc.getGetIdentityMethod) == null) {
          PlatformGrpc.getGetIdentityMethod = getGetIdentityMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentity"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentity"))
              .build();
        }
      }
    }
    return getGetIdentityMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse> getGetIdentityKeysMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityKeys",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse> getGetIdentityKeysMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse> getGetIdentityKeysMethod;
    if ((getGetIdentityKeysMethod = PlatformGrpc.getGetIdentityKeysMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityKeysMethod = PlatformGrpc.getGetIdentityKeysMethod) == null) {
          PlatformGrpc.getGetIdentityKeysMethod = getGetIdentityKeysMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityKeys"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityKeys"))
              .build();
        }
      }
    }
    return getGetIdentityKeysMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse> getGetIdentitiesContractKeysMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentitiesContractKeys",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse> getGetIdentitiesContractKeysMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse> getGetIdentitiesContractKeysMethod;
    if ((getGetIdentitiesContractKeysMethod = PlatformGrpc.getGetIdentitiesContractKeysMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentitiesContractKeysMethod = PlatformGrpc.getGetIdentitiesContractKeysMethod) == null) {
          PlatformGrpc.getGetIdentitiesContractKeysMethod = getGetIdentitiesContractKeysMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentitiesContractKeys"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentitiesContractKeys"))
              .build();
        }
      }
    }
    return getGetIdentitiesContractKeysMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse> getGetIdentityNonceMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityNonce",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse> getGetIdentityNonceMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse> getGetIdentityNonceMethod;
    if ((getGetIdentityNonceMethod = PlatformGrpc.getGetIdentityNonceMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityNonceMethod = PlatformGrpc.getGetIdentityNonceMethod) == null) {
          PlatformGrpc.getGetIdentityNonceMethod = getGetIdentityNonceMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityNonce"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityNonce"))
              .build();
        }
      }
    }
    return getGetIdentityNonceMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse> getGetIdentityContractNonceMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityContractNonce",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse> getGetIdentityContractNonceMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse> getGetIdentityContractNonceMethod;
    if ((getGetIdentityContractNonceMethod = PlatformGrpc.getGetIdentityContractNonceMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityContractNonceMethod = PlatformGrpc.getGetIdentityContractNonceMethod) == null) {
          PlatformGrpc.getGetIdentityContractNonceMethod = getGetIdentityContractNonceMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityContractNonce"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityContractNonce"))
              .build();
        }
      }
    }
    return getGetIdentityContractNonceMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse> getGetIdentityBalanceMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityBalance",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse> getGetIdentityBalanceMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse> getGetIdentityBalanceMethod;
    if ((getGetIdentityBalanceMethod = PlatformGrpc.getGetIdentityBalanceMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityBalanceMethod = PlatformGrpc.getGetIdentityBalanceMethod) == null) {
          PlatformGrpc.getGetIdentityBalanceMethod = getGetIdentityBalanceMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityBalance"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityBalance"))
              .build();
        }
      }
    }
    return getGetIdentityBalanceMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> getGetIdentityBalanceAndRevisionMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityBalanceAndRevision",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> getGetIdentityBalanceAndRevisionMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> getGetIdentityBalanceAndRevisionMethod;
    if ((getGetIdentityBalanceAndRevisionMethod = PlatformGrpc.getGetIdentityBalanceAndRevisionMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityBalanceAndRevisionMethod = PlatformGrpc.getGetIdentityBalanceAndRevisionMethod) == null) {
          PlatformGrpc.getGetIdentityBalanceAndRevisionMethod = getGetIdentityBalanceAndRevisionMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityBalanceAndRevision"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityBalanceAndRevision"))
              .build();
        }
      }
    }
    return getGetIdentityBalanceAndRevisionMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse> getGetProofsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getProofs",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse> getGetProofsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse> getGetProofsMethod;
    if ((getGetProofsMethod = PlatformGrpc.getGetProofsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetProofsMethod = PlatformGrpc.getGetProofsMethod) == null) {
          PlatformGrpc.getGetProofsMethod = getGetProofsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getProofs"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getProofs"))
              .build();
        }
      }
    }
    return getGetProofsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> getGetDataContractMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getDataContract",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> getGetDataContractMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> getGetDataContractMethod;
    if ((getGetDataContractMethod = PlatformGrpc.getGetDataContractMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetDataContractMethod = PlatformGrpc.getGetDataContractMethod) == null) {
          PlatformGrpc.getGetDataContractMethod = getGetDataContractMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getDataContract"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getDataContract"))
              .build();
        }
      }
    }
    return getGetDataContractMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse> getGetDataContractHistoryMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getDataContractHistory",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse> getGetDataContractHistoryMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse> getGetDataContractHistoryMethod;
    if ((getGetDataContractHistoryMethod = PlatformGrpc.getGetDataContractHistoryMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetDataContractHistoryMethod = PlatformGrpc.getGetDataContractHistoryMethod) == null) {
          PlatformGrpc.getGetDataContractHistoryMethod = getGetDataContractHistoryMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getDataContractHistory"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getDataContractHistory"))
              .build();
        }
      }
    }
    return getGetDataContractHistoryMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse> getGetDataContractsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getDataContracts",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse> getGetDataContractsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse> getGetDataContractsMethod;
    if ((getGetDataContractsMethod = PlatformGrpc.getGetDataContractsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetDataContractsMethod = PlatformGrpc.getGetDataContractsMethod) == null) {
          PlatformGrpc.getGetDataContractsMethod = getGetDataContractsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getDataContracts"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getDataContracts"))
              .build();
        }
      }
    }
    return getGetDataContractsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> getGetDocumentsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getDocuments",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> getGetDocumentsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> getGetDocumentsMethod;
    if ((getGetDocumentsMethod = PlatformGrpc.getGetDocumentsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetDocumentsMethod = PlatformGrpc.getGetDocumentsMethod) == null) {
          PlatformGrpc.getGetDocumentsMethod = getGetDocumentsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getDocuments"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getDocuments"))
              .build();
        }
      }
    }
    return getGetDocumentsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse> getGetIdentityByPublicKeyHashMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityByPublicKeyHash",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse> getGetIdentityByPublicKeyHashMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse> getGetIdentityByPublicKeyHashMethod;
    if ((getGetIdentityByPublicKeyHashMethod = PlatformGrpc.getGetIdentityByPublicKeyHashMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityByPublicKeyHashMethod = PlatformGrpc.getGetIdentityByPublicKeyHashMethod) == null) {
          PlatformGrpc.getGetIdentityByPublicKeyHashMethod = getGetIdentityByPublicKeyHashMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityByPublicKeyHash"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityByPublicKeyHash"))
              .build();
        }
      }
    }
    return getGetIdentityByPublicKeyHashMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse> getWaitForStateTransitionResultMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "waitForStateTransitionResult",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse> getWaitForStateTransitionResultMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest, org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse> getWaitForStateTransitionResultMethod;
    if ((getWaitForStateTransitionResultMethod = PlatformGrpc.getWaitForStateTransitionResultMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getWaitForStateTransitionResultMethod = PlatformGrpc.getWaitForStateTransitionResultMethod) == null) {
          PlatformGrpc.getWaitForStateTransitionResultMethod = getWaitForStateTransitionResultMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest, org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "waitForStateTransitionResult"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("waitForStateTransitionResult"))
              .build();
        }
      }
    }
    return getWaitForStateTransitionResultMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse> getGetConsensusParamsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getConsensusParams",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse> getGetConsensusParamsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse> getGetConsensusParamsMethod;
    if ((getGetConsensusParamsMethod = PlatformGrpc.getGetConsensusParamsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetConsensusParamsMethod = PlatformGrpc.getGetConsensusParamsMethod) == null) {
          PlatformGrpc.getGetConsensusParamsMethod = getGetConsensusParamsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getConsensusParams"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getConsensusParams"))
              .build();
        }
      }
    }
    return getGetConsensusParamsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse> getGetProtocolVersionUpgradeStateMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getProtocolVersionUpgradeState",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse> getGetProtocolVersionUpgradeStateMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse> getGetProtocolVersionUpgradeStateMethod;
    if ((getGetProtocolVersionUpgradeStateMethod = PlatformGrpc.getGetProtocolVersionUpgradeStateMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetProtocolVersionUpgradeStateMethod = PlatformGrpc.getGetProtocolVersionUpgradeStateMethod) == null) {
          PlatformGrpc.getGetProtocolVersionUpgradeStateMethod = getGetProtocolVersionUpgradeStateMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getProtocolVersionUpgradeState"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getProtocolVersionUpgradeState"))
              .build();
        }
      }
    }
    return getGetProtocolVersionUpgradeStateMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse> getGetProtocolVersionUpgradeVoteStatusMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getProtocolVersionUpgradeVoteStatus",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse> getGetProtocolVersionUpgradeVoteStatusMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse> getGetProtocolVersionUpgradeVoteStatusMethod;
    if ((getGetProtocolVersionUpgradeVoteStatusMethod = PlatformGrpc.getGetProtocolVersionUpgradeVoteStatusMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetProtocolVersionUpgradeVoteStatusMethod = PlatformGrpc.getGetProtocolVersionUpgradeVoteStatusMethod) == null) {
          PlatformGrpc.getGetProtocolVersionUpgradeVoteStatusMethod = getGetProtocolVersionUpgradeVoteStatusMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getProtocolVersionUpgradeVoteStatus"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getProtocolVersionUpgradeVoteStatus"))
              .build();
        }
      }
    }
    return getGetProtocolVersionUpgradeVoteStatusMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse> getGetEpochsInfoMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getEpochsInfo",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse> getGetEpochsInfoMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse> getGetEpochsInfoMethod;
    if ((getGetEpochsInfoMethod = PlatformGrpc.getGetEpochsInfoMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetEpochsInfoMethod = PlatformGrpc.getGetEpochsInfoMethod) == null) {
          PlatformGrpc.getGetEpochsInfoMethod = getGetEpochsInfoMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getEpochsInfo"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getEpochsInfo"))
              .build();
        }
      }
    }
    return getGetEpochsInfoMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> getGetPathElementsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getPathElements",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> getGetPathElementsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> getGetPathElementsMethod;
    if ((getGetPathElementsMethod = PlatformGrpc.getGetPathElementsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetPathElementsMethod = PlatformGrpc.getGetPathElementsMethod) == null) {
          PlatformGrpc.getGetPathElementsMethod = getGetPathElementsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getPathElements"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getPathElements"))
              .build();
        }
      }
    }
    return getGetPathElementsMethod;
  }

  /**
   * Creates a new async stub that supports all call types for the service
   */
  public static PlatformStub newStub(io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PlatformStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PlatformStub>() {
        @java.lang.Override
        public PlatformStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PlatformStub(channel, callOptions);
        }
      };
    return PlatformStub.newStub(factory, channel);
  }

  /**
   * Creates a new blocking-style stub that supports unary and streaming output calls on the service
   */
  public static PlatformBlockingStub newBlockingStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PlatformBlockingStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PlatformBlockingStub>() {
        @java.lang.Override
        public PlatformBlockingStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PlatformBlockingStub(channel, callOptions);
        }
      };
    return PlatformBlockingStub.newStub(factory, channel);
  }

  /**
   * Creates a new ListenableFuture-style stub that supports unary calls on the service
   */
  public static PlatformFutureStub newFutureStub(
      io.grpc.Channel channel) {
    io.grpc.stub.AbstractStub.StubFactory<PlatformFutureStub> factory =
      new io.grpc.stub.AbstractStub.StubFactory<PlatformFutureStub>() {
        @java.lang.Override
        public PlatformFutureStub newStub(io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
          return new PlatformFutureStub(channel, callOptions);
        }
      };
    return PlatformFutureStub.newStub(factory, channel);
  }

  /**
   */
  public static abstract class PlatformImplBase implements io.grpc.BindableService {

    /**
     */
    public void broadcastStateTransition(org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getBroadcastStateTransitionMethod(), responseObserver);
    }

    /**
     */
    public void getIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityKeys(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityKeysMethod(), responseObserver);
    }

    /**
     */
    public void getIdentitiesContractKeys(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentitiesContractKeysMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityNonce(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityNonceMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityContractNonce(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityContractNonceMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityBalance(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityBalanceMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityBalanceAndRevision(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityBalanceAndRevisionMethod(), responseObserver);
    }

    /**
     */
    public void getProofs(org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetProofsMethod(), responseObserver);
    }

    /**
     */
    public void getDataContract(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetDataContractMethod(), responseObserver);
    }

    /**
     */
    public void getDataContractHistory(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetDataContractHistoryMethod(), responseObserver);
    }

    /**
     */
    public void getDataContracts(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetDataContractsMethod(), responseObserver);
    }

    /**
     */
    public void getDocuments(org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetDocumentsMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityByPublicKeyHash(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityByPublicKeyHashMethod(), responseObserver);
    }

    /**
     */
    public void waitForStateTransitionResult(org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getWaitForStateTransitionResultMethod(), responseObserver);
    }

    /**
     */
    public void getConsensusParams(org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetConsensusParamsMethod(), responseObserver);
    }

    /**
     */
    public void getProtocolVersionUpgradeState(org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetProtocolVersionUpgradeStateMethod(), responseObserver);
    }

    /**
     */
    public void getProtocolVersionUpgradeVoteStatus(org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetProtocolVersionUpgradeVoteStatusMethod(), responseObserver);
    }

    /**
     */
    public void getEpochsInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetEpochsInfoMethod(), responseObserver);
    }

    /**
     */
    public void getPathElements(org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetPathElementsMethod(), responseObserver);
    }

    @java.lang.Override public final io.grpc.ServerServiceDefinition bindService() {
      return io.grpc.ServerServiceDefinition.builder(getServiceDescriptor())
          .addMethod(
            getBroadcastStateTransitionMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse>(
                  this, METHODID_BROADCAST_STATE_TRANSITION)))
          .addMethod(
            getGetIdentityMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse>(
                  this, METHODID_GET_IDENTITY)))
          .addMethod(
            getGetIdentityKeysMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse>(
                  this, METHODID_GET_IDENTITY_KEYS)))
          .addMethod(
            getGetIdentitiesContractKeysMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse>(
                  this, METHODID_GET_IDENTITIES_CONTRACT_KEYS)))
          .addMethod(
            getGetIdentityNonceMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse>(
                  this, METHODID_GET_IDENTITY_NONCE)))
          .addMethod(
            getGetIdentityContractNonceMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse>(
                  this, METHODID_GET_IDENTITY_CONTRACT_NONCE)))
          .addMethod(
            getGetIdentityBalanceMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse>(
                  this, METHODID_GET_IDENTITY_BALANCE)))
          .addMethod(
            getGetIdentityBalanceAndRevisionMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse>(
                  this, METHODID_GET_IDENTITY_BALANCE_AND_REVISION)))
          .addMethod(
            getGetProofsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse>(
                  this, METHODID_GET_PROOFS)))
          .addMethod(
            getGetDataContractMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse>(
                  this, METHODID_GET_DATA_CONTRACT)))
          .addMethod(
            getGetDataContractHistoryMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse>(
                  this, METHODID_GET_DATA_CONTRACT_HISTORY)))
          .addMethod(
            getGetDataContractsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse>(
                  this, METHODID_GET_DATA_CONTRACTS)))
          .addMethod(
            getGetDocumentsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse>(
                  this, METHODID_GET_DOCUMENTS)))
          .addMethod(
            getGetIdentityByPublicKeyHashMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse>(
                  this, METHODID_GET_IDENTITY_BY_PUBLIC_KEY_HASH)))
          .addMethod(
            getWaitForStateTransitionResultMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse>(
                  this, METHODID_WAIT_FOR_STATE_TRANSITION_RESULT)))
          .addMethod(
            getGetConsensusParamsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse>(
                  this, METHODID_GET_CONSENSUS_PARAMS)))
          .addMethod(
            getGetProtocolVersionUpgradeStateMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse>(
                  this, METHODID_GET_PROTOCOL_VERSION_UPGRADE_STATE)))
          .addMethod(
            getGetProtocolVersionUpgradeVoteStatusMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse>(
                  this, METHODID_GET_PROTOCOL_VERSION_UPGRADE_VOTE_STATUS)))
          .addMethod(
            getGetEpochsInfoMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse>(
                  this, METHODID_GET_EPOCHS_INFO)))
          .addMethod(
            getGetPathElementsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse>(
                  this, METHODID_GET_PATH_ELEMENTS)))
          .build();
    }
  }

  /**
   */
  public static final class PlatformStub extends io.grpc.stub.AbstractAsyncStub<PlatformStub> {
    private PlatformStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PlatformStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PlatformStub(channel, callOptions);
    }

    /**
     */
    public void broadcastStateTransition(org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getBroadcastStateTransitionMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityKeys(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityKeysMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentitiesContractKeys(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentitiesContractKeysMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityNonce(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityNonceMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityContractNonce(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityContractNonceMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityBalance(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityBalanceMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityBalanceAndRevision(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityBalanceAndRevisionMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getProofs(org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetProofsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getDataContract(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetDataContractMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getDataContractHistory(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetDataContractHistoryMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getDataContracts(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetDataContractsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getDocuments(org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetDocumentsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityByPublicKeyHash(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityByPublicKeyHashMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void waitForStateTransitionResult(org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getWaitForStateTransitionResultMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getConsensusParams(org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetConsensusParamsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getProtocolVersionUpgradeState(org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetProtocolVersionUpgradeStateMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getProtocolVersionUpgradeVoteStatus(org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetProtocolVersionUpgradeVoteStatusMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getEpochsInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetEpochsInfoMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getPathElements(org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetPathElementsMethod(), getCallOptions()), request, responseObserver);
    }
  }

  /**
   */
  public static final class PlatformBlockingStub extends io.grpc.stub.AbstractBlockingStub<PlatformBlockingStub> {
    private PlatformBlockingStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PlatformBlockingStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PlatformBlockingStub(channel, callOptions);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse broadcastStateTransition(org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getBroadcastStateTransitionMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse getIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse getIdentityKeys(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityKeysMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse getIdentitiesContractKeys(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentitiesContractKeysMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse getIdentityNonce(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityNonceMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse getIdentityContractNonce(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityContractNonceMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse getIdentityBalance(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityBalanceMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse getIdentityBalanceAndRevision(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityBalanceAndRevisionMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse getProofs(org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetProofsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse getDataContract(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetDataContractMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse getDataContractHistory(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetDataContractHistoryMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse getDataContracts(org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetDataContractsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse getDocuments(org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetDocumentsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse getIdentityByPublicKeyHash(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityByPublicKeyHashMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse waitForStateTransitionResult(org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getWaitForStateTransitionResultMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse getConsensusParams(org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetConsensusParamsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse getProtocolVersionUpgradeState(org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetProtocolVersionUpgradeStateMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse getProtocolVersionUpgradeVoteStatus(org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetProtocolVersionUpgradeVoteStatusMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse getEpochsInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetEpochsInfoMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse getPathElements(org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetPathElementsMethod(), getCallOptions(), request);
    }
  }

  /**
   */
  public static final class PlatformFutureStub extends io.grpc.stub.AbstractFutureStub<PlatformFutureStub> {
    private PlatformFutureStub(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      super(channel, callOptions);
    }

    @java.lang.Override
    protected PlatformFutureStub build(
        io.grpc.Channel channel, io.grpc.CallOptions callOptions) {
      return new PlatformFutureStub(channel, callOptions);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionResponse> broadcastStateTransition(
        org.dash.platform.dapi.v0.PlatformOuterClass.BroadcastStateTransitionRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getBroadcastStateTransitionMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityResponse> getIdentity(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse> getIdentityKeys(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityKeysMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse> getIdentitiesContractKeys(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentitiesContractKeysMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse> getIdentityNonce(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityNonceMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse> getIdentityContractNonce(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityContractNonceMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse> getIdentityBalance(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityBalanceMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> getIdentityBalanceAndRevision(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityBalanceAndRevisionMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse> getProofs(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetProofsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse> getDataContract(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetDataContractMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse> getDataContractHistory(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetDataContractHistoryMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse> getDataContracts(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetDataContractsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse> getDocuments(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetDocumentsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse> getIdentityByPublicKeyHash(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityByPublicKeyHashMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse> waitForStateTransitionResult(
        org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getWaitForStateTransitionResultMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse> getConsensusParams(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetConsensusParamsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse> getProtocolVersionUpgradeState(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetProtocolVersionUpgradeStateMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse> getProtocolVersionUpgradeVoteStatus(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetProtocolVersionUpgradeVoteStatusMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse> getEpochsInfo(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetEpochsInfoMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> getPathElements(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetPathElementsMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_BROADCAST_STATE_TRANSITION = 0;
  private static final int METHODID_GET_IDENTITY = 1;
  private static final int METHODID_GET_IDENTITY_KEYS = 2;
  private static final int METHODID_GET_IDENTITIES_CONTRACT_KEYS = 3;
  private static final int METHODID_GET_IDENTITY_NONCE = 4;
  private static final int METHODID_GET_IDENTITY_CONTRACT_NONCE = 5;
  private static final int METHODID_GET_IDENTITY_BALANCE = 6;
  private static final int METHODID_GET_IDENTITY_BALANCE_AND_REVISION = 7;
  private static final int METHODID_GET_PROOFS = 8;
  private static final int METHODID_GET_DATA_CONTRACT = 9;
  private static final int METHODID_GET_DATA_CONTRACT_HISTORY = 10;
  private static final int METHODID_GET_DATA_CONTRACTS = 11;
  private static final int METHODID_GET_DOCUMENTS = 12;
  private static final int METHODID_GET_IDENTITY_BY_PUBLIC_KEY_HASH = 13;
  private static final int METHODID_WAIT_FOR_STATE_TRANSITION_RESULT = 14;
  private static final int METHODID_GET_CONSENSUS_PARAMS = 15;
  private static final int METHODID_GET_PROTOCOL_VERSION_UPGRADE_STATE = 16;
  private static final int METHODID_GET_PROTOCOL_VERSION_UPGRADE_VOTE_STATUS = 17;
  private static final int METHODID_GET_EPOCHS_INFO = 18;
  private static final int METHODID_GET_PATH_ELEMENTS = 19;

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
        case METHODID_GET_IDENTITY_KEYS:
          serviceImpl.getIdentityKeys((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityKeysResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITIES_CONTRACT_KEYS:
          serviceImpl.getIdentitiesContractKeys((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesContractKeysResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_NONCE:
          serviceImpl.getIdentityNonce((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityNonceResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_CONTRACT_NONCE:
          serviceImpl.getIdentityContractNonce((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityContractNonceResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_BALANCE:
          serviceImpl.getIdentityBalance((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_BALANCE_AND_REVISION:
          serviceImpl.getIdentityBalanceAndRevision((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse>) responseObserver);
          break;
        case METHODID_GET_PROOFS:
          serviceImpl.getProofs((org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProofsResponse>) responseObserver);
          break;
        case METHODID_GET_DATA_CONTRACT:
          serviceImpl.getDataContract((org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractResponse>) responseObserver);
          break;
        case METHODID_GET_DATA_CONTRACT_HISTORY:
          serviceImpl.getDataContractHistory((org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractHistoryResponse>) responseObserver);
          break;
        case METHODID_GET_DATA_CONTRACTS:
          serviceImpl.getDataContracts((org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDataContractsResponse>) responseObserver);
          break;
        case METHODID_GET_DOCUMENTS:
          serviceImpl.getDocuments((org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetDocumentsResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_BY_PUBLIC_KEY_HASH:
          serviceImpl.getIdentityByPublicKeyHash((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityByPublicKeyHashResponse>) responseObserver);
          break;
        case METHODID_WAIT_FOR_STATE_TRANSITION_RESULT:
          serviceImpl.waitForStateTransitionResult((org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.WaitForStateTransitionResultResponse>) responseObserver);
          break;
        case METHODID_GET_CONSENSUS_PARAMS:
          serviceImpl.getConsensusParams((org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetConsensusParamsResponse>) responseObserver);
          break;
        case METHODID_GET_PROTOCOL_VERSION_UPGRADE_STATE:
          serviceImpl.getProtocolVersionUpgradeState((org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeStateResponse>) responseObserver);
          break;
        case METHODID_GET_PROTOCOL_VERSION_UPGRADE_VOTE_STATUS:
          serviceImpl.getProtocolVersionUpgradeVoteStatus((org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetProtocolVersionUpgradeVoteStatusResponse>) responseObserver);
          break;
        case METHODID_GET_EPOCHS_INFO:
          serviceImpl.getEpochsInfo((org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEpochsInfoResponse>) responseObserver);
          break;
        case METHODID_GET_PATH_ELEMENTS:
          serviceImpl.getPathElements((org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse>) responseObserver);
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

  private static abstract class PlatformBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoFileDescriptorSupplier, io.grpc.protobuf.ProtoServiceDescriptorSupplier {
    PlatformBaseDescriptorSupplier() {}

    @java.lang.Override
    public com.google.protobuf.Descriptors.FileDescriptor getFileDescriptor() {
      return org.dash.platform.dapi.v0.PlatformOuterClass.getDescriptor();
    }

    @java.lang.Override
    public com.google.protobuf.Descriptors.ServiceDescriptor getServiceDescriptor() {
      return getFileDescriptor().findServiceByName("Platform");
    }
  }

  private static final class PlatformFileDescriptorSupplier
      extends PlatformBaseDescriptorSupplier {
    PlatformFileDescriptorSupplier() {}
  }

  private static final class PlatformMethodDescriptorSupplier
      extends PlatformBaseDescriptorSupplier
      implements io.grpc.protobuf.ProtoMethodDescriptorSupplier {
    private final String methodName;

    PlatformMethodDescriptorSupplier(String methodName) {
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
      synchronized (PlatformGrpc.class) {
        result = serviceDescriptor;
        if (result == null) {
          serviceDescriptor = result = io.grpc.ServiceDescriptor.newBuilder(SERVICE_NAME)
              .setSchemaDescriptor(new PlatformFileDescriptorSupplier())
              .addMethod(getBroadcastStateTransitionMethod())
              .addMethod(getGetIdentityMethod())
              .addMethod(getGetIdentityKeysMethod())
              .addMethod(getGetIdentitiesContractKeysMethod())
              .addMethod(getGetIdentityNonceMethod())
              .addMethod(getGetIdentityContractNonceMethod())
              .addMethod(getGetIdentityBalanceMethod())
              .addMethod(getGetIdentityBalanceAndRevisionMethod())
              .addMethod(getGetProofsMethod())
              .addMethod(getGetDataContractMethod())
              .addMethod(getGetDataContractHistoryMethod())
              .addMethod(getGetDataContractsMethod())
              .addMethod(getGetDocumentsMethod())
              .addMethod(getGetIdentityByPublicKeyHashMethod())
              .addMethod(getWaitForStateTransitionResultMethod())
              .addMethod(getGetConsensusParamsMethod())
              .addMethod(getGetProtocolVersionUpgradeStateMethod())
              .addMethod(getGetProtocolVersionUpgradeVoteStatusMethod())
              .addMethod(getGetEpochsInfoMethod())
              .addMethod(getGetPathElementsMethod())
              .build();
        }
      }
    }
    return result;
  }
}
