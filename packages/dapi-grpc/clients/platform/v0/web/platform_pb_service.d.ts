// package: org.dash.platform.dapi.v0
// file: platform.proto

import * as platform_pb from "./platform_pb";
import {grpc} from "@improbable-eng/grpc-web";

type PlatformbroadcastStateTransition = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.BroadcastStateTransitionRequest;
  readonly responseType: typeof platform_pb.BroadcastStateTransitionResponse;
};

type PlatformgetIdentity = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetSingleItemRequest;
  readonly responseType: typeof platform_pb.SingleItemResponse;
};

type PlatformgetIdentityBalance = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetSingleItemRequest;
  readonly responseType: typeof platform_pb.SingleItemResponse;
};

type PlatformgetIdentityBalanceAndRevision = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetSingleItemRequest;
  readonly responseType: typeof platform_pb.SingleItemResponse;
};

type PlatformgetDataContract = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetSingleItemRequest;
  readonly responseType: typeof platform_pb.SingleItemResponse;
};

type PlatformgetDocuments = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetDocumentsRequest;
  readonly responseType: typeof platform_pb.MultiItemResponse;
};

type PlatformgetIdentitiesByPublicKeyHashes = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetMultiItemRequest;
  readonly responseType: typeof platform_pb.MultiItemResponse;
};

type PlatformwaitForStateTransitionResult = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.WaitForStateTransitionResultRequest;
  readonly responseType: typeof platform_pb.WaitForStateTransitionResultResponse;
};

type PlatformgetConsensusParams = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetConsensusParamsRequest;
  readonly responseType: typeof platform_pb.GetConsensusParamsResponse;
};

export class Platform {
  static readonly serviceName: string;
  static readonly broadcastStateTransition: PlatformbroadcastStateTransition;
  static readonly getIdentity: PlatformgetIdentity;
  static readonly getIdentityBalance: PlatformgetIdentityBalance;
  static readonly getIdentityBalanceAndRevision: PlatformgetIdentityBalanceAndRevision;
  static readonly getDataContract: PlatformgetDataContract;
  static readonly getDocuments: PlatformgetDocuments;
  static readonly getIdentitiesByPublicKeyHashes: PlatformgetIdentitiesByPublicKeyHashes;
  static readonly waitForStateTransitionResult: PlatformwaitForStateTransitionResult;
  static readonly getConsensusParams: PlatformgetConsensusParams;
}

export type ServiceError = { message: string, code: number; metadata: grpc.Metadata }
export type Status = { details: string, code: number; metadata: grpc.Metadata }

interface UnaryResponse {
  cancel(): void;
}
interface ResponseStream<T> {
  cancel(): void;
  on(type: 'data', handler: (message: T) => void): ResponseStream<T>;
  on(type: 'end', handler: (status?: Status) => void): ResponseStream<T>;
  on(type: 'status', handler: (status: Status) => void): ResponseStream<T>;
}
interface RequestStream<T> {
  write(message: T): RequestStream<T>;
  end(): void;
  cancel(): void;
  on(type: 'end', handler: (status?: Status) => void): RequestStream<T>;
  on(type: 'status', handler: (status: Status) => void): RequestStream<T>;
}
interface BidirectionalStream<ReqT, ResT> {
  write(message: ReqT): BidirectionalStream<ReqT, ResT>;
  end(): void;
  cancel(): void;
  on(type: 'data', handler: (message: ResT) => void): BidirectionalStream<ReqT, ResT>;
  on(type: 'end', handler: (status?: Status) => void): BidirectionalStream<ReqT, ResT>;
  on(type: 'status', handler: (status: Status) => void): BidirectionalStream<ReqT, ResT>;
}

export class PlatformClient {
  readonly serviceHost: string;

  constructor(serviceHost: string, options?: grpc.RpcOptions);
  broadcastStateTransition(
    requestMessage: platform_pb.BroadcastStateTransitionRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.BroadcastStateTransitionResponse|null) => void
  ): UnaryResponse;
  broadcastStateTransition(
    requestMessage: platform_pb.BroadcastStateTransitionRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.BroadcastStateTransitionResponse|null) => void
  ): UnaryResponse;
  getIdentity(
    requestMessage: platform_pb.GetSingleItemRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getIdentity(
    requestMessage: platform_pb.GetSingleItemRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getIdentityBalance(
    requestMessage: platform_pb.GetSingleItemRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getIdentityBalance(
    requestMessage: platform_pb.GetSingleItemRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getIdentityBalanceAndRevision(
    requestMessage: platform_pb.GetSingleItemRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getIdentityBalanceAndRevision(
    requestMessage: platform_pb.GetSingleItemRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getDataContract(
    requestMessage: platform_pb.GetSingleItemRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getDataContract(
    requestMessage: platform_pb.GetSingleItemRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.SingleItemResponse|null) => void
  ): UnaryResponse;
  getDocuments(
    requestMessage: platform_pb.GetDocumentsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.MultiItemResponse|null) => void
  ): UnaryResponse;
  getDocuments(
    requestMessage: platform_pb.GetDocumentsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.MultiItemResponse|null) => void
  ): UnaryResponse;
  getIdentitiesByPublicKeyHashes(
    requestMessage: platform_pb.GetMultiItemRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.MultiItemResponse|null) => void
  ): UnaryResponse;
  getIdentitiesByPublicKeyHashes(
    requestMessage: platform_pb.GetMultiItemRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.MultiItemResponse|null) => void
  ): UnaryResponse;
  waitForStateTransitionResult(
    requestMessage: platform_pb.WaitForStateTransitionResultRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.WaitForStateTransitionResultResponse|null) => void
  ): UnaryResponse;
  waitForStateTransitionResult(
    requestMessage: platform_pb.WaitForStateTransitionResultRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.WaitForStateTransitionResultResponse|null) => void
  ): UnaryResponse;
  getConsensusParams(
    requestMessage: platform_pb.GetConsensusParamsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetConsensusParamsResponse|null) => void
  ): UnaryResponse;
  getConsensusParams(
    requestMessage: platform_pb.GetConsensusParamsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetConsensusParamsResponse|null) => void
  ): UnaryResponse;
}

