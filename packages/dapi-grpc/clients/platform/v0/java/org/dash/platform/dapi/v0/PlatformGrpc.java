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

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse> getGetIdentitiesBalancesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentitiesBalances",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse> getGetIdentitiesBalancesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse> getGetIdentitiesBalancesMethod;
    if ((getGetIdentitiesBalancesMethod = PlatformGrpc.getGetIdentitiesBalancesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentitiesBalancesMethod = PlatformGrpc.getGetIdentitiesBalancesMethod) == null) {
          PlatformGrpc.getGetIdentitiesBalancesMethod = getGetIdentitiesBalancesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentitiesBalances"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentitiesBalances"))
              .build();
        }
      }
    }
    return getGetIdentitiesBalancesMethod;
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

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getGetEvonodesProposedEpochBlocksByIdsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getEvonodesProposedEpochBlocksByIds",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getGetEvonodesProposedEpochBlocksByIdsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getGetEvonodesProposedEpochBlocksByIdsMethod;
    if ((getGetEvonodesProposedEpochBlocksByIdsMethod = PlatformGrpc.getGetEvonodesProposedEpochBlocksByIdsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetEvonodesProposedEpochBlocksByIdsMethod = PlatformGrpc.getGetEvonodesProposedEpochBlocksByIdsMethod) == null) {
          PlatformGrpc.getGetEvonodesProposedEpochBlocksByIdsMethod = getGetEvonodesProposedEpochBlocksByIdsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getEvonodesProposedEpochBlocksByIds"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getEvonodesProposedEpochBlocksByIds"))
              .build();
        }
      }
    }
    return getGetEvonodesProposedEpochBlocksByIdsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getGetEvonodesProposedEpochBlocksByRangeMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getEvonodesProposedEpochBlocksByRange",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getGetEvonodesProposedEpochBlocksByRangeMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getGetEvonodesProposedEpochBlocksByRangeMethod;
    if ((getGetEvonodesProposedEpochBlocksByRangeMethod = PlatformGrpc.getGetEvonodesProposedEpochBlocksByRangeMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetEvonodesProposedEpochBlocksByRangeMethod = PlatformGrpc.getGetEvonodesProposedEpochBlocksByRangeMethod) == null) {
          PlatformGrpc.getGetEvonodesProposedEpochBlocksByRangeMethod = getGetEvonodesProposedEpochBlocksByRangeMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getEvonodesProposedEpochBlocksByRange"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getEvonodesProposedEpochBlocksByRange"))
              .build();
        }
      }
    }
    return getGetEvonodesProposedEpochBlocksByRangeMethod;
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

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse> getGetContestedResourcesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getContestedResources",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse> getGetContestedResourcesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse> getGetContestedResourcesMethod;
    if ((getGetContestedResourcesMethod = PlatformGrpc.getGetContestedResourcesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetContestedResourcesMethod = PlatformGrpc.getGetContestedResourcesMethod) == null) {
          PlatformGrpc.getGetContestedResourcesMethod = getGetContestedResourcesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getContestedResources"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getContestedResources"))
              .build();
        }
      }
    }
    return getGetContestedResourcesMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse> getGetContestedResourceVoteStateMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getContestedResourceVoteState",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse> getGetContestedResourceVoteStateMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse> getGetContestedResourceVoteStateMethod;
    if ((getGetContestedResourceVoteStateMethod = PlatformGrpc.getGetContestedResourceVoteStateMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetContestedResourceVoteStateMethod = PlatformGrpc.getGetContestedResourceVoteStateMethod) == null) {
          PlatformGrpc.getGetContestedResourceVoteStateMethod = getGetContestedResourceVoteStateMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getContestedResourceVoteState"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getContestedResourceVoteState"))
              .build();
        }
      }
    }
    return getGetContestedResourceVoteStateMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse> getGetContestedResourceVotersForIdentityMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getContestedResourceVotersForIdentity",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse> getGetContestedResourceVotersForIdentityMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse> getGetContestedResourceVotersForIdentityMethod;
    if ((getGetContestedResourceVotersForIdentityMethod = PlatformGrpc.getGetContestedResourceVotersForIdentityMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetContestedResourceVotersForIdentityMethod = PlatformGrpc.getGetContestedResourceVotersForIdentityMethod) == null) {
          PlatformGrpc.getGetContestedResourceVotersForIdentityMethod = getGetContestedResourceVotersForIdentityMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getContestedResourceVotersForIdentity"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getContestedResourceVotersForIdentity"))
              .build();
        }
      }
    }
    return getGetContestedResourceVotersForIdentityMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse> getGetContestedResourceIdentityVotesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getContestedResourceIdentityVotes",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse> getGetContestedResourceIdentityVotesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse> getGetContestedResourceIdentityVotesMethod;
    if ((getGetContestedResourceIdentityVotesMethod = PlatformGrpc.getGetContestedResourceIdentityVotesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetContestedResourceIdentityVotesMethod = PlatformGrpc.getGetContestedResourceIdentityVotesMethod) == null) {
          PlatformGrpc.getGetContestedResourceIdentityVotesMethod = getGetContestedResourceIdentityVotesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getContestedResourceIdentityVotes"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getContestedResourceIdentityVotes"))
              .build();
        }
      }
    }
    return getGetContestedResourceIdentityVotesMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse> getGetVotePollsByEndDateMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getVotePollsByEndDate",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse> getGetVotePollsByEndDateMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse> getGetVotePollsByEndDateMethod;
    if ((getGetVotePollsByEndDateMethod = PlatformGrpc.getGetVotePollsByEndDateMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetVotePollsByEndDateMethod = PlatformGrpc.getGetVotePollsByEndDateMethod) == null) {
          PlatformGrpc.getGetVotePollsByEndDateMethod = getGetVotePollsByEndDateMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getVotePollsByEndDate"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getVotePollsByEndDate"))
              .build();
        }
      }
    }
    return getGetVotePollsByEndDateMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse> getGetPrefundedSpecializedBalanceMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getPrefundedSpecializedBalance",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse> getGetPrefundedSpecializedBalanceMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse> getGetPrefundedSpecializedBalanceMethod;
    if ((getGetPrefundedSpecializedBalanceMethod = PlatformGrpc.getGetPrefundedSpecializedBalanceMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetPrefundedSpecializedBalanceMethod = PlatformGrpc.getGetPrefundedSpecializedBalanceMethod) == null) {
          PlatformGrpc.getGetPrefundedSpecializedBalanceMethod = getGetPrefundedSpecializedBalanceMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getPrefundedSpecializedBalance"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getPrefundedSpecializedBalance"))
              .build();
        }
      }
    }
    return getGetPrefundedSpecializedBalanceMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse> getGetTotalCreditsInPlatformMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getTotalCreditsInPlatform",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse> getGetTotalCreditsInPlatformMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse> getGetTotalCreditsInPlatformMethod;
    if ((getGetTotalCreditsInPlatformMethod = PlatformGrpc.getGetTotalCreditsInPlatformMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetTotalCreditsInPlatformMethod = PlatformGrpc.getGetTotalCreditsInPlatformMethod) == null) {
          PlatformGrpc.getGetTotalCreditsInPlatformMethod = getGetTotalCreditsInPlatformMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getTotalCreditsInPlatform"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getTotalCreditsInPlatform"))
              .build();
        }
      }
    }
    return getGetTotalCreditsInPlatformMethod;
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

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse> getGetStatusMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getStatus",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse> getGetStatusMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse> getGetStatusMethod;
    if ((getGetStatusMethod = PlatformGrpc.getGetStatusMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetStatusMethod = PlatformGrpc.getGetStatusMethod) == null) {
          PlatformGrpc.getGetStatusMethod = getGetStatusMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getStatus"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getStatus"))
              .build();
        }
      }
    }
    return getGetStatusMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse> getGetCurrentQuorumsInfoMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getCurrentQuorumsInfo",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse> getGetCurrentQuorumsInfoMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse> getGetCurrentQuorumsInfoMethod;
    if ((getGetCurrentQuorumsInfoMethod = PlatformGrpc.getGetCurrentQuorumsInfoMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetCurrentQuorumsInfoMethod = PlatformGrpc.getGetCurrentQuorumsInfoMethod) == null) {
          PlatformGrpc.getGetCurrentQuorumsInfoMethod = getGetCurrentQuorumsInfoMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getCurrentQuorumsInfo"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getCurrentQuorumsInfo"))
              .build();
        }
      }
    }
    return getGetCurrentQuorumsInfoMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse> getGetIdentityTokenBalancesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityTokenBalances",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse> getGetIdentityTokenBalancesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse> getGetIdentityTokenBalancesMethod;
    if ((getGetIdentityTokenBalancesMethod = PlatformGrpc.getGetIdentityTokenBalancesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityTokenBalancesMethod = PlatformGrpc.getGetIdentityTokenBalancesMethod) == null) {
          PlatformGrpc.getGetIdentityTokenBalancesMethod = getGetIdentityTokenBalancesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityTokenBalances"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityTokenBalances"))
              .build();
        }
      }
    }
    return getGetIdentityTokenBalancesMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse> getGetIdentitiesTokenBalancesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentitiesTokenBalances",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse> getGetIdentitiesTokenBalancesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse> getGetIdentitiesTokenBalancesMethod;
    if ((getGetIdentitiesTokenBalancesMethod = PlatformGrpc.getGetIdentitiesTokenBalancesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentitiesTokenBalancesMethod = PlatformGrpc.getGetIdentitiesTokenBalancesMethod) == null) {
          PlatformGrpc.getGetIdentitiesTokenBalancesMethod = getGetIdentitiesTokenBalancesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentitiesTokenBalances"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentitiesTokenBalances"))
              .build();
        }
      }
    }
    return getGetIdentitiesTokenBalancesMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse> getGetIdentityTokenInfosMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentityTokenInfos",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse> getGetIdentityTokenInfosMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse> getGetIdentityTokenInfosMethod;
    if ((getGetIdentityTokenInfosMethod = PlatformGrpc.getGetIdentityTokenInfosMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentityTokenInfosMethod = PlatformGrpc.getGetIdentityTokenInfosMethod) == null) {
          PlatformGrpc.getGetIdentityTokenInfosMethod = getGetIdentityTokenInfosMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentityTokenInfos"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentityTokenInfos"))
              .build();
        }
      }
    }
    return getGetIdentityTokenInfosMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse> getGetIdentitiesTokenInfosMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getIdentitiesTokenInfos",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse> getGetIdentitiesTokenInfosMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse> getGetIdentitiesTokenInfosMethod;
    if ((getGetIdentitiesTokenInfosMethod = PlatformGrpc.getGetIdentitiesTokenInfosMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetIdentitiesTokenInfosMethod = PlatformGrpc.getGetIdentitiesTokenInfosMethod) == null) {
          PlatformGrpc.getGetIdentitiesTokenInfosMethod = getGetIdentitiesTokenInfosMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getIdentitiesTokenInfos"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getIdentitiesTokenInfos"))
              .build();
        }
      }
    }
    return getGetIdentitiesTokenInfosMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse> getGetTokenStatusesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getTokenStatuses",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse> getGetTokenStatusesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse> getGetTokenStatusesMethod;
    if ((getGetTokenStatusesMethod = PlatformGrpc.getGetTokenStatusesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetTokenStatusesMethod = PlatformGrpc.getGetTokenStatusesMethod) == null) {
          PlatformGrpc.getGetTokenStatusesMethod = getGetTokenStatusesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getTokenStatuses"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getTokenStatuses"))
              .build();
        }
      }
    }
    return getGetTokenStatusesMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse> getGetTokenDirectPurchasePricesMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getTokenDirectPurchasePrices",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse> getGetTokenDirectPurchasePricesMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse> getGetTokenDirectPurchasePricesMethod;
    if ((getGetTokenDirectPurchasePricesMethod = PlatformGrpc.getGetTokenDirectPurchasePricesMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetTokenDirectPurchasePricesMethod = PlatformGrpc.getGetTokenDirectPurchasePricesMethod) == null) {
          PlatformGrpc.getGetTokenDirectPurchasePricesMethod = getGetTokenDirectPurchasePricesMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getTokenDirectPurchasePrices"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getTokenDirectPurchasePrices"))
              .build();
        }
      }
    }
    return getGetTokenDirectPurchasePricesMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse> getGetTokenPreProgrammedDistributionsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getTokenPreProgrammedDistributions",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse> getGetTokenPreProgrammedDistributionsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse> getGetTokenPreProgrammedDistributionsMethod;
    if ((getGetTokenPreProgrammedDistributionsMethod = PlatformGrpc.getGetTokenPreProgrammedDistributionsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetTokenPreProgrammedDistributionsMethod = PlatformGrpc.getGetTokenPreProgrammedDistributionsMethod) == null) {
          PlatformGrpc.getGetTokenPreProgrammedDistributionsMethod = getGetTokenPreProgrammedDistributionsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getTokenPreProgrammedDistributions"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getTokenPreProgrammedDistributions"))
              .build();
        }
      }
    }
    return getGetTokenPreProgrammedDistributionsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse> getGetTokenTotalSupplyMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getTokenTotalSupply",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse> getGetTokenTotalSupplyMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse> getGetTokenTotalSupplyMethod;
    if ((getGetTokenTotalSupplyMethod = PlatformGrpc.getGetTokenTotalSupplyMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetTokenTotalSupplyMethod = PlatformGrpc.getGetTokenTotalSupplyMethod) == null) {
          PlatformGrpc.getGetTokenTotalSupplyMethod = getGetTokenTotalSupplyMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getTokenTotalSupply"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getTokenTotalSupply"))
              .build();
        }
      }
    }
    return getGetTokenTotalSupplyMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse> getGetGroupInfoMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getGroupInfo",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse> getGetGroupInfoMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse> getGetGroupInfoMethod;
    if ((getGetGroupInfoMethod = PlatformGrpc.getGetGroupInfoMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetGroupInfoMethod = PlatformGrpc.getGetGroupInfoMethod) == null) {
          PlatformGrpc.getGetGroupInfoMethod = getGetGroupInfoMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getGroupInfo"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getGroupInfo"))
              .build();
        }
      }
    }
    return getGetGroupInfoMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse> getGetGroupInfosMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getGroupInfos",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse> getGetGroupInfosMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse> getGetGroupInfosMethod;
    if ((getGetGroupInfosMethod = PlatformGrpc.getGetGroupInfosMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetGroupInfosMethod = PlatformGrpc.getGetGroupInfosMethod) == null) {
          PlatformGrpc.getGetGroupInfosMethod = getGetGroupInfosMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getGroupInfos"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getGroupInfos"))
              .build();
        }
      }
    }
    return getGetGroupInfosMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse> getGetGroupActionsMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getGroupActions",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse> getGetGroupActionsMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse> getGetGroupActionsMethod;
    if ((getGetGroupActionsMethod = PlatformGrpc.getGetGroupActionsMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetGroupActionsMethod = PlatformGrpc.getGetGroupActionsMethod) == null) {
          PlatformGrpc.getGetGroupActionsMethod = getGetGroupActionsMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getGroupActions"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getGroupActions"))
              .build();
        }
      }
    }
    return getGetGroupActionsMethod;
  }

  private static volatile io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse> getGetGroupActionSignersMethod;

  @io.grpc.stub.annotations.RpcMethod(
      fullMethodName = SERVICE_NAME + '/' + "getGroupActionSigners",
      requestType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest.class,
      responseType = org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse.class,
      methodType = io.grpc.MethodDescriptor.MethodType.UNARY)
  public static io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest,
      org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse> getGetGroupActionSignersMethod() {
    io.grpc.MethodDescriptor<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse> getGetGroupActionSignersMethod;
    if ((getGetGroupActionSignersMethod = PlatformGrpc.getGetGroupActionSignersMethod) == null) {
      synchronized (PlatformGrpc.class) {
        if ((getGetGroupActionSignersMethod = PlatformGrpc.getGetGroupActionSignersMethod) == null) {
          PlatformGrpc.getGetGroupActionSignersMethod = getGetGroupActionSignersMethod =
              io.grpc.MethodDescriptor.<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest, org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse>newBuilder()
              .setType(io.grpc.MethodDescriptor.MethodType.UNARY)
              .setFullMethodName(generateFullMethodName(SERVICE_NAME, "getGroupActionSigners"))
              .setSampledToLocalTracing(true)
              .setRequestMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest.getDefaultInstance()))
              .setResponseMarshaller(io.grpc.protobuf.ProtoUtils.marshaller(
                  org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse.getDefaultInstance()))
              .setSchemaDescriptor(new PlatformMethodDescriptorSupplier("getGroupActionSigners"))
              .build();
        }
      }
    }
    return getGetGroupActionSignersMethod;
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
    public void getIdentitiesBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentitiesBalancesMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityBalanceAndRevision(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityBalanceAndRevisionMethod(), responseObserver);
    }

    /**
     */
    public void getEvonodesProposedEpochBlocksByIds(org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetEvonodesProposedEpochBlocksByIdsMethod(), responseObserver);
    }

    /**
     */
    public void getEvonodesProposedEpochBlocksByRange(org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetEvonodesProposedEpochBlocksByRangeMethod(), responseObserver);
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
     * <pre>
     * What votes are currently happening for a specific contested index
     * </pre>
     */
    public void getContestedResources(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetContestedResourcesMethod(), responseObserver);
    }

    /**
     * <pre>
     * What's the state of a contested resource vote? (ie who is winning?)
     * </pre>
     */
    public void getContestedResourceVoteState(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetContestedResourceVoteStateMethod(), responseObserver);
    }

    /**
     * <pre>
     * Who voted for a contested resource to go to a specific identity?
     * </pre>
     */
    public void getContestedResourceVotersForIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetContestedResourceVotersForIdentityMethod(), responseObserver);
    }

    /**
     * <pre>
     * How did an identity vote?
     * </pre>
     */
    public void getContestedResourceIdentityVotes(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetContestedResourceIdentityVotesMethod(), responseObserver);
    }

    /**
     * <pre>
     * What vote polls will end soon?
     * </pre>
     */
    public void getVotePollsByEndDate(org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetVotePollsByEndDateMethod(), responseObserver);
    }

    /**
     */
    public void getPrefundedSpecializedBalance(org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetPrefundedSpecializedBalanceMethod(), responseObserver);
    }

    /**
     */
    public void getTotalCreditsInPlatform(org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTotalCreditsInPlatformMethod(), responseObserver);
    }

    /**
     */
    public void getPathElements(org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetPathElementsMethod(), responseObserver);
    }

    /**
     */
    public void getStatus(org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetStatusMethod(), responseObserver);
    }

    /**
     */
    public void getCurrentQuorumsInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetCurrentQuorumsInfoMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityTokenBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityTokenBalancesMethod(), responseObserver);
    }

    /**
     */
    public void getIdentitiesTokenBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentitiesTokenBalancesMethod(), responseObserver);
    }

    /**
     */
    public void getIdentityTokenInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentityTokenInfosMethod(), responseObserver);
    }

    /**
     */
    public void getIdentitiesTokenInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetIdentitiesTokenInfosMethod(), responseObserver);
    }

    /**
     */
    public void getTokenStatuses(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTokenStatusesMethod(), responseObserver);
    }

    /**
     */
    public void getTokenDirectPurchasePrices(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTokenDirectPurchasePricesMethod(), responseObserver);
    }

    /**
     */
    public void getTokenPreProgrammedDistributions(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTokenPreProgrammedDistributionsMethod(), responseObserver);
    }

    /**
     */
    public void getTokenTotalSupply(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetTokenTotalSupplyMethod(), responseObserver);
    }

    /**
     */
    public void getGroupInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetGroupInfoMethod(), responseObserver);
    }

    /**
     */
    public void getGroupInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetGroupInfosMethod(), responseObserver);
    }

    /**
     */
    public void getGroupActions(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetGroupActionsMethod(), responseObserver);
    }

    /**
     */
    public void getGroupActionSigners(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse> responseObserver) {
      io.grpc.stub.ServerCalls.asyncUnimplementedUnaryCall(getGetGroupActionSignersMethod(), responseObserver);
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
            getGetIdentitiesBalancesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse>(
                  this, METHODID_GET_IDENTITIES_BALANCES)))
          .addMethod(
            getGetIdentityBalanceAndRevisionMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse>(
                  this, METHODID_GET_IDENTITY_BALANCE_AND_REVISION)))
          .addMethod(
            getGetEvonodesProposedEpochBlocksByIdsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse>(
                  this, METHODID_GET_EVONODES_PROPOSED_EPOCH_BLOCKS_BY_IDS)))
          .addMethod(
            getGetEvonodesProposedEpochBlocksByRangeMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse>(
                  this, METHODID_GET_EVONODES_PROPOSED_EPOCH_BLOCKS_BY_RANGE)))
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
            getGetContestedResourcesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse>(
                  this, METHODID_GET_CONTESTED_RESOURCES)))
          .addMethod(
            getGetContestedResourceVoteStateMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse>(
                  this, METHODID_GET_CONTESTED_RESOURCE_VOTE_STATE)))
          .addMethod(
            getGetContestedResourceVotersForIdentityMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse>(
                  this, METHODID_GET_CONTESTED_RESOURCE_VOTERS_FOR_IDENTITY)))
          .addMethod(
            getGetContestedResourceIdentityVotesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse>(
                  this, METHODID_GET_CONTESTED_RESOURCE_IDENTITY_VOTES)))
          .addMethod(
            getGetVotePollsByEndDateMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse>(
                  this, METHODID_GET_VOTE_POLLS_BY_END_DATE)))
          .addMethod(
            getGetPrefundedSpecializedBalanceMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse>(
                  this, METHODID_GET_PREFUNDED_SPECIALIZED_BALANCE)))
          .addMethod(
            getGetTotalCreditsInPlatformMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse>(
                  this, METHODID_GET_TOTAL_CREDITS_IN_PLATFORM)))
          .addMethod(
            getGetPathElementsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse>(
                  this, METHODID_GET_PATH_ELEMENTS)))
          .addMethod(
            getGetStatusMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse>(
                  this, METHODID_GET_STATUS)))
          .addMethod(
            getGetCurrentQuorumsInfoMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse>(
                  this, METHODID_GET_CURRENT_QUORUMS_INFO)))
          .addMethod(
            getGetIdentityTokenBalancesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse>(
                  this, METHODID_GET_IDENTITY_TOKEN_BALANCES)))
          .addMethod(
            getGetIdentitiesTokenBalancesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse>(
                  this, METHODID_GET_IDENTITIES_TOKEN_BALANCES)))
          .addMethod(
            getGetIdentityTokenInfosMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse>(
                  this, METHODID_GET_IDENTITY_TOKEN_INFOS)))
          .addMethod(
            getGetIdentitiesTokenInfosMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse>(
                  this, METHODID_GET_IDENTITIES_TOKEN_INFOS)))
          .addMethod(
            getGetTokenStatusesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse>(
                  this, METHODID_GET_TOKEN_STATUSES)))
          .addMethod(
            getGetTokenDirectPurchasePricesMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse>(
                  this, METHODID_GET_TOKEN_DIRECT_PURCHASE_PRICES)))
          .addMethod(
            getGetTokenPreProgrammedDistributionsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse>(
                  this, METHODID_GET_TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS)))
          .addMethod(
            getGetTokenTotalSupplyMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse>(
                  this, METHODID_GET_TOKEN_TOTAL_SUPPLY)))
          .addMethod(
            getGetGroupInfoMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse>(
                  this, METHODID_GET_GROUP_INFO)))
          .addMethod(
            getGetGroupInfosMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse>(
                  this, METHODID_GET_GROUP_INFOS)))
          .addMethod(
            getGetGroupActionsMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse>(
                  this, METHODID_GET_GROUP_ACTIONS)))
          .addMethod(
            getGetGroupActionSignersMethod(),
            io.grpc.stub.ServerCalls.asyncUnaryCall(
              new MethodHandlers<
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest,
                org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse>(
                  this, METHODID_GET_GROUP_ACTION_SIGNERS)))
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
    public void getIdentitiesBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentitiesBalancesMethod(), getCallOptions()), request, responseObserver);
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
    public void getEvonodesProposedEpochBlocksByIds(org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetEvonodesProposedEpochBlocksByIdsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getEvonodesProposedEpochBlocksByRange(org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetEvonodesProposedEpochBlocksByRangeMethod(), getCallOptions()), request, responseObserver);
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
     * <pre>
     * What votes are currently happening for a specific contested index
     * </pre>
     */
    public void getContestedResources(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetContestedResourcesMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     * <pre>
     * What's the state of a contested resource vote? (ie who is winning?)
     * </pre>
     */
    public void getContestedResourceVoteState(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetContestedResourceVoteStateMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     * <pre>
     * Who voted for a contested resource to go to a specific identity?
     * </pre>
     */
    public void getContestedResourceVotersForIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetContestedResourceVotersForIdentityMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     * <pre>
     * How did an identity vote?
     * </pre>
     */
    public void getContestedResourceIdentityVotes(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetContestedResourceIdentityVotesMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     * <pre>
     * What vote polls will end soon?
     * </pre>
     */
    public void getVotePollsByEndDate(org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetVotePollsByEndDateMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getPrefundedSpecializedBalance(org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetPrefundedSpecializedBalanceMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTotalCreditsInPlatform(org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTotalCreditsInPlatformMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getPathElements(org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetPathElementsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getStatus(org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetStatusMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getCurrentQuorumsInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetCurrentQuorumsInfoMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityTokenBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityTokenBalancesMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentitiesTokenBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentitiesTokenBalancesMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentityTokenInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentityTokenInfosMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getIdentitiesTokenInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetIdentitiesTokenInfosMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTokenStatuses(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTokenStatusesMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTokenDirectPurchasePrices(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTokenDirectPurchasePricesMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTokenPreProgrammedDistributions(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTokenPreProgrammedDistributionsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getTokenTotalSupply(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetTokenTotalSupplyMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getGroupInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetGroupInfoMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getGroupInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetGroupInfosMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getGroupActions(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetGroupActionsMethod(), getCallOptions()), request, responseObserver);
    }

    /**
     */
    public void getGroupActionSigners(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest request,
        io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse> responseObserver) {
      io.grpc.stub.ClientCalls.asyncUnaryCall(
          getChannel().newCall(getGetGroupActionSignersMethod(), getCallOptions()), request, responseObserver);
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
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse getIdentitiesBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentitiesBalancesMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse getIdentityBalanceAndRevision(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityBalanceAndRevisionMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse getEvonodesProposedEpochBlocksByIds(org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetEvonodesProposedEpochBlocksByIdsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse getEvonodesProposedEpochBlocksByRange(org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetEvonodesProposedEpochBlocksByRangeMethod(), getCallOptions(), request);
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
     * <pre>
     * What votes are currently happening for a specific contested index
     * </pre>
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse getContestedResources(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetContestedResourcesMethod(), getCallOptions(), request);
    }

    /**
     * <pre>
     * What's the state of a contested resource vote? (ie who is winning?)
     * </pre>
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse getContestedResourceVoteState(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetContestedResourceVoteStateMethod(), getCallOptions(), request);
    }

    /**
     * <pre>
     * Who voted for a contested resource to go to a specific identity?
     * </pre>
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse getContestedResourceVotersForIdentity(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetContestedResourceVotersForIdentityMethod(), getCallOptions(), request);
    }

    /**
     * <pre>
     * How did an identity vote?
     * </pre>
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse getContestedResourceIdentityVotes(org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetContestedResourceIdentityVotesMethod(), getCallOptions(), request);
    }

    /**
     * <pre>
     * What vote polls will end soon?
     * </pre>
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse getVotePollsByEndDate(org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetVotePollsByEndDateMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse getPrefundedSpecializedBalance(org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetPrefundedSpecializedBalanceMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse getTotalCreditsInPlatform(org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTotalCreditsInPlatformMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse getPathElements(org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetPathElementsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse getStatus(org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetStatusMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse getCurrentQuorumsInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetCurrentQuorumsInfoMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse getIdentityTokenBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityTokenBalancesMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse getIdentitiesTokenBalances(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentitiesTokenBalancesMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse getIdentityTokenInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentityTokenInfosMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse getIdentitiesTokenInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetIdentitiesTokenInfosMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse getTokenStatuses(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTokenStatusesMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse getTokenDirectPurchasePrices(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTokenDirectPurchasePricesMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse getTokenPreProgrammedDistributions(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTokenPreProgrammedDistributionsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse getTokenTotalSupply(org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetTokenTotalSupplyMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse getGroupInfo(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetGroupInfoMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse getGroupInfos(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetGroupInfosMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse getGroupActions(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetGroupActionsMethod(), getCallOptions(), request);
    }

    /**
     */
    public org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse getGroupActionSigners(org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest request) {
      return io.grpc.stub.ClientCalls.blockingUnaryCall(
          getChannel(), getGetGroupActionSignersMethod(), getCallOptions(), request);
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
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse> getIdentitiesBalances(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentitiesBalancesMethod(), getCallOptions()), request);
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
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getEvonodesProposedEpochBlocksByIds(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetEvonodesProposedEpochBlocksByIdsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse> getEvonodesProposedEpochBlocksByRange(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetEvonodesProposedEpochBlocksByRangeMethod(), getCallOptions()), request);
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
     * <pre>
     * What votes are currently happening for a specific contested index
     * </pre>
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse> getContestedResources(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetContestedResourcesMethod(), getCallOptions()), request);
    }

    /**
     * <pre>
     * What's the state of a contested resource vote? (ie who is winning?)
     * </pre>
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse> getContestedResourceVoteState(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetContestedResourceVoteStateMethod(), getCallOptions()), request);
    }

    /**
     * <pre>
     * Who voted for a contested resource to go to a specific identity?
     * </pre>
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse> getContestedResourceVotersForIdentity(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetContestedResourceVotersForIdentityMethod(), getCallOptions()), request);
    }

    /**
     * <pre>
     * How did an identity vote?
     * </pre>
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse> getContestedResourceIdentityVotes(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetContestedResourceIdentityVotesMethod(), getCallOptions()), request);
    }

    /**
     * <pre>
     * What vote polls will end soon?
     * </pre>
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse> getVotePollsByEndDate(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetVotePollsByEndDateMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse> getPrefundedSpecializedBalance(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetPrefundedSpecializedBalanceMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse> getTotalCreditsInPlatform(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTotalCreditsInPlatformMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse> getPathElements(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetPathElementsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse> getStatus(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetStatusMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse> getCurrentQuorumsInfo(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetCurrentQuorumsInfoMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse> getIdentityTokenBalances(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityTokenBalancesMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse> getIdentitiesTokenBalances(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentitiesTokenBalancesMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse> getIdentityTokenInfos(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentityTokenInfosMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse> getIdentitiesTokenInfos(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetIdentitiesTokenInfosMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse> getTokenStatuses(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTokenStatusesMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse> getTokenDirectPurchasePrices(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTokenDirectPurchasePricesMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse> getTokenPreProgrammedDistributions(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTokenPreProgrammedDistributionsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse> getTokenTotalSupply(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetTokenTotalSupplyMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse> getGroupInfo(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetGroupInfoMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse> getGroupInfos(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetGroupInfosMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse> getGroupActions(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetGroupActionsMethod(), getCallOptions()), request);
    }

    /**
     */
    public com.google.common.util.concurrent.ListenableFuture<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse> getGroupActionSigners(
        org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest request) {
      return io.grpc.stub.ClientCalls.futureUnaryCall(
          getChannel().newCall(getGetGroupActionSignersMethod(), getCallOptions()), request);
    }
  }

  private static final int METHODID_BROADCAST_STATE_TRANSITION = 0;
  private static final int METHODID_GET_IDENTITY = 1;
  private static final int METHODID_GET_IDENTITY_KEYS = 2;
  private static final int METHODID_GET_IDENTITIES_CONTRACT_KEYS = 3;
  private static final int METHODID_GET_IDENTITY_NONCE = 4;
  private static final int METHODID_GET_IDENTITY_CONTRACT_NONCE = 5;
  private static final int METHODID_GET_IDENTITY_BALANCE = 6;
  private static final int METHODID_GET_IDENTITIES_BALANCES = 7;
  private static final int METHODID_GET_IDENTITY_BALANCE_AND_REVISION = 8;
  private static final int METHODID_GET_EVONODES_PROPOSED_EPOCH_BLOCKS_BY_IDS = 9;
  private static final int METHODID_GET_EVONODES_PROPOSED_EPOCH_BLOCKS_BY_RANGE = 10;
  private static final int METHODID_GET_DATA_CONTRACT = 11;
  private static final int METHODID_GET_DATA_CONTRACT_HISTORY = 12;
  private static final int METHODID_GET_DATA_CONTRACTS = 13;
  private static final int METHODID_GET_DOCUMENTS = 14;
  private static final int METHODID_GET_IDENTITY_BY_PUBLIC_KEY_HASH = 15;
  private static final int METHODID_WAIT_FOR_STATE_TRANSITION_RESULT = 16;
  private static final int METHODID_GET_CONSENSUS_PARAMS = 17;
  private static final int METHODID_GET_PROTOCOL_VERSION_UPGRADE_STATE = 18;
  private static final int METHODID_GET_PROTOCOL_VERSION_UPGRADE_VOTE_STATUS = 19;
  private static final int METHODID_GET_EPOCHS_INFO = 20;
  private static final int METHODID_GET_CONTESTED_RESOURCES = 21;
  private static final int METHODID_GET_CONTESTED_RESOURCE_VOTE_STATE = 22;
  private static final int METHODID_GET_CONTESTED_RESOURCE_VOTERS_FOR_IDENTITY = 23;
  private static final int METHODID_GET_CONTESTED_RESOURCE_IDENTITY_VOTES = 24;
  private static final int METHODID_GET_VOTE_POLLS_BY_END_DATE = 25;
  private static final int METHODID_GET_PREFUNDED_SPECIALIZED_BALANCE = 26;
  private static final int METHODID_GET_TOTAL_CREDITS_IN_PLATFORM = 27;
  private static final int METHODID_GET_PATH_ELEMENTS = 28;
  private static final int METHODID_GET_STATUS = 29;
  private static final int METHODID_GET_CURRENT_QUORUMS_INFO = 30;
  private static final int METHODID_GET_IDENTITY_TOKEN_BALANCES = 31;
  private static final int METHODID_GET_IDENTITIES_TOKEN_BALANCES = 32;
  private static final int METHODID_GET_IDENTITY_TOKEN_INFOS = 33;
  private static final int METHODID_GET_IDENTITIES_TOKEN_INFOS = 34;
  private static final int METHODID_GET_TOKEN_STATUSES = 35;
  private static final int METHODID_GET_TOKEN_DIRECT_PURCHASE_PRICES = 36;
  private static final int METHODID_GET_TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS = 37;
  private static final int METHODID_GET_TOKEN_TOTAL_SUPPLY = 38;
  private static final int METHODID_GET_GROUP_INFO = 39;
  private static final int METHODID_GET_GROUP_INFOS = 40;
  private static final int METHODID_GET_GROUP_ACTIONS = 41;
  private static final int METHODID_GET_GROUP_ACTION_SIGNERS = 42;

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
        case METHODID_GET_IDENTITIES_BALANCES:
          serviceImpl.getIdentitiesBalances((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesBalancesResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_BALANCE_AND_REVISION:
          serviceImpl.getIdentityBalanceAndRevision((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityBalanceAndRevisionResponse>) responseObserver);
          break;
        case METHODID_GET_EVONODES_PROPOSED_EPOCH_BLOCKS_BY_IDS:
          serviceImpl.getEvonodesProposedEpochBlocksByIds((org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByIdsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse>) responseObserver);
          break;
        case METHODID_GET_EVONODES_PROPOSED_EPOCH_BLOCKS_BY_RANGE:
          serviceImpl.getEvonodesProposedEpochBlocksByRange((org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksByRangeRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetEvonodesProposedEpochBlocksResponse>) responseObserver);
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
        case METHODID_GET_CONTESTED_RESOURCES:
          serviceImpl.getContestedResources((org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourcesResponse>) responseObserver);
          break;
        case METHODID_GET_CONTESTED_RESOURCE_VOTE_STATE:
          serviceImpl.getContestedResourceVoteState((org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVoteStateResponse>) responseObserver);
          break;
        case METHODID_GET_CONTESTED_RESOURCE_VOTERS_FOR_IDENTITY:
          serviceImpl.getContestedResourceVotersForIdentity((org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceVotersForIdentityResponse>) responseObserver);
          break;
        case METHODID_GET_CONTESTED_RESOURCE_IDENTITY_VOTES:
          serviceImpl.getContestedResourceIdentityVotes((org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetContestedResourceIdentityVotesResponse>) responseObserver);
          break;
        case METHODID_GET_VOTE_POLLS_BY_END_DATE:
          serviceImpl.getVotePollsByEndDate((org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetVotePollsByEndDateResponse>) responseObserver);
          break;
        case METHODID_GET_PREFUNDED_SPECIALIZED_BALANCE:
          serviceImpl.getPrefundedSpecializedBalance((org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPrefundedSpecializedBalanceResponse>) responseObserver);
          break;
        case METHODID_GET_TOTAL_CREDITS_IN_PLATFORM:
          serviceImpl.getTotalCreditsInPlatform((org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTotalCreditsInPlatformResponse>) responseObserver);
          break;
        case METHODID_GET_PATH_ELEMENTS:
          serviceImpl.getPathElements((org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetPathElementsResponse>) responseObserver);
          break;
        case METHODID_GET_STATUS:
          serviceImpl.getStatus((org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetStatusResponse>) responseObserver);
          break;
        case METHODID_GET_CURRENT_QUORUMS_INFO:
          serviceImpl.getCurrentQuorumsInfo((org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetCurrentQuorumsInfoResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_TOKEN_BALANCES:
          serviceImpl.getIdentityTokenBalances((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenBalancesResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITIES_TOKEN_BALANCES:
          serviceImpl.getIdentitiesTokenBalances((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenBalancesResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITY_TOKEN_INFOS:
          serviceImpl.getIdentityTokenInfos((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentityTokenInfosResponse>) responseObserver);
          break;
        case METHODID_GET_IDENTITIES_TOKEN_INFOS:
          serviceImpl.getIdentitiesTokenInfos((org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetIdentitiesTokenInfosResponse>) responseObserver);
          break;
        case METHODID_GET_TOKEN_STATUSES:
          serviceImpl.getTokenStatuses((org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenStatusesResponse>) responseObserver);
          break;
        case METHODID_GET_TOKEN_DIRECT_PURCHASE_PRICES:
          serviceImpl.getTokenDirectPurchasePrices((org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenDirectPurchasePricesResponse>) responseObserver);
          break;
        case METHODID_GET_TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS:
          serviceImpl.getTokenPreProgrammedDistributions((org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenPreProgrammedDistributionsResponse>) responseObserver);
          break;
        case METHODID_GET_TOKEN_TOTAL_SUPPLY:
          serviceImpl.getTokenTotalSupply((org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetTokenTotalSupplyResponse>) responseObserver);
          break;
        case METHODID_GET_GROUP_INFO:
          serviceImpl.getGroupInfo((org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfoResponse>) responseObserver);
          break;
        case METHODID_GET_GROUP_INFOS:
          serviceImpl.getGroupInfos((org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupInfosResponse>) responseObserver);
          break;
        case METHODID_GET_GROUP_ACTIONS:
          serviceImpl.getGroupActions((org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionsResponse>) responseObserver);
          break;
        case METHODID_GET_GROUP_ACTION_SIGNERS:
          serviceImpl.getGroupActionSigners((org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersRequest) request,
              (io.grpc.stub.StreamObserver<org.dash.platform.dapi.v0.PlatformOuterClass.GetGroupActionSignersResponse>) responseObserver);
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
              .addMethod(getGetIdentitiesBalancesMethod())
              .addMethod(getGetIdentityBalanceAndRevisionMethod())
              .addMethod(getGetEvonodesProposedEpochBlocksByIdsMethod())
              .addMethod(getGetEvonodesProposedEpochBlocksByRangeMethod())
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
              .addMethod(getGetContestedResourcesMethod())
              .addMethod(getGetContestedResourceVoteStateMethod())
              .addMethod(getGetContestedResourceVotersForIdentityMethod())
              .addMethod(getGetContestedResourceIdentityVotesMethod())
              .addMethod(getGetVotePollsByEndDateMethod())
              .addMethod(getGetPrefundedSpecializedBalanceMethod())
              .addMethod(getGetTotalCreditsInPlatformMethod())
              .addMethod(getGetPathElementsMethod())
              .addMethod(getGetStatusMethod())
              .addMethod(getGetCurrentQuorumsInfoMethod())
              .addMethod(getGetIdentityTokenBalancesMethod())
              .addMethod(getGetIdentitiesTokenBalancesMethod())
              .addMethod(getGetIdentityTokenInfosMethod())
              .addMethod(getGetIdentitiesTokenInfosMethod())
              .addMethod(getGetTokenStatusesMethod())
              .addMethod(getGetTokenDirectPurchasePricesMethod())
              .addMethod(getGetTokenPreProgrammedDistributionsMethod())
              .addMethod(getGetTokenTotalSupplyMethod())
              .addMethod(getGetGroupInfoMethod())
              .addMethod(getGetGroupInfosMethod())
              .addMethod(getGetGroupActionsMethod())
              .addMethod(getGetGroupActionSignersMethod())
              .build();
        }
      }
    }
    return result;
  }
}
