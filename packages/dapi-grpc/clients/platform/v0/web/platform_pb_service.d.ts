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
  readonly requestType: typeof platform_pb.GetIdentityRequest;
  readonly responseType: typeof platform_pb.GetIdentityResponse;
};

type PlatformgetIdentities = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentitiesRequest;
  readonly responseType: typeof platform_pb.GetIdentitiesResponse;
};

type PlatformgetIdentityKeys = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityKeysRequest;
  readonly responseType: typeof platform_pb.GetIdentityKeysResponse;
};

type PlatformgetIdentityBalance = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityBalanceRequest;
  readonly responseType: typeof platform_pb.GetIdentityBalanceResponse;
};

type PlatformgetIdentityBalanceAndRevision = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityBalanceAndRevisionRequest;
  readonly responseType: typeof platform_pb.GetIdentityBalanceAndRevisionResponse;
};

type PlatformgetProofs = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetProofsRequest;
  readonly responseType: typeof platform_pb.GetProofsResponse;
};

type PlatformgetDataContract = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetDataContractRequest;
  readonly responseType: typeof platform_pb.GetDataContractResponse;
};

type PlatformgetDataContractHistory = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetDataContractHistoryRequest;
  readonly responseType: typeof platform_pb.GetDataContractHistoryResponse;
};

type PlatformgetDataContracts = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetDataContractsRequest;
  readonly responseType: typeof platform_pb.GetDataContractsResponse;
};

type PlatformgetDocuments = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetDocumentsRequest;
  readonly responseType: typeof platform_pb.GetDocumentsResponse;
};

type PlatformgetIdentitiesByPublicKeyHashes = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentitiesByPublicKeyHashesRequest;
  readonly responseType: typeof platform_pb.GetIdentitiesByPublicKeyHashesResponse;
};

type PlatformgetIdentityByPublicKeyHash = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityByPublicKeyHashRequest;
  readonly responseType: typeof platform_pb.GetIdentityByPublicKeyHashResponse;
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

type PlatformgetProtocolVersionUpgradeState = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetProtocolVersionUpgradeStateRequest;
  readonly responseType: typeof platform_pb.GetProtocolVersionUpgradeStateResponse;
};

type PlatformgetProtocolVersionUpgradeVoteStatus = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetProtocolVersionUpgradeVoteStatusRequest;
  readonly responseType: typeof platform_pb.GetProtocolVersionUpgradeVoteStatusResponse;
};

type PlatformgetEpochsInfo = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetEpochsInfoRequest;
  readonly responseType: typeof platform_pb.GetEpochsInfoResponse;
};

export class Platform {
  static readonly serviceName: string;
  static readonly broadcastStateTransition: PlatformbroadcastStateTransition;
  static readonly getIdentity: PlatformgetIdentity;
  static readonly getIdentities: PlatformgetIdentities;
  static readonly getIdentityKeys: PlatformgetIdentityKeys;
  static readonly getIdentityBalance: PlatformgetIdentityBalance;
  static readonly getIdentityBalanceAndRevision: PlatformgetIdentityBalanceAndRevision;
  static readonly getProofs: PlatformgetProofs;
  static readonly getDataContract: PlatformgetDataContract;
  static readonly getDataContractHistory: PlatformgetDataContractHistory;
  static readonly getDataContracts: PlatformgetDataContracts;
  static readonly getDocuments: PlatformgetDocuments;
  static readonly getIdentitiesByPublicKeyHashes: PlatformgetIdentitiesByPublicKeyHashes;
  static readonly getIdentityByPublicKeyHash: PlatformgetIdentityByPublicKeyHash;
  static readonly waitForStateTransitionResult: PlatformwaitForStateTransitionResult;
  static readonly getConsensusParams: PlatformgetConsensusParams;
  static readonly getProtocolVersionUpgradeState: PlatformgetProtocolVersionUpgradeState;
  static readonly getProtocolVersionUpgradeVoteStatus: PlatformgetProtocolVersionUpgradeVoteStatus;
  static readonly getEpochsInfo: PlatformgetEpochsInfo;
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
    requestMessage: platform_pb.GetIdentityRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityResponse|null) => void
  ): UnaryResponse;
  getIdentity(
    requestMessage: platform_pb.GetIdentityRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityResponse|null) => void
  ): UnaryResponse;
  getIdentities(
    requestMessage: platform_pb.GetIdentitiesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesResponse|null) => void
  ): UnaryResponse;
  getIdentities(
    requestMessage: platform_pb.GetIdentitiesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesResponse|null) => void
  ): UnaryResponse;
  getIdentityKeys(
    requestMessage: platform_pb.GetIdentityKeysRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityKeysResponse|null) => void
  ): UnaryResponse;
  getIdentityKeys(
    requestMessage: platform_pb.GetIdentityKeysRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityKeysResponse|null) => void
  ): UnaryResponse;
  getIdentityBalance(
    requestMessage: platform_pb.GetIdentityBalanceRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityBalanceResponse|null) => void
  ): UnaryResponse;
  getIdentityBalance(
    requestMessage: platform_pb.GetIdentityBalanceRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityBalanceResponse|null) => void
  ): UnaryResponse;
  getIdentityBalanceAndRevision(
    requestMessage: platform_pb.GetIdentityBalanceAndRevisionRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityBalanceAndRevisionResponse|null) => void
  ): UnaryResponse;
  getIdentityBalanceAndRevision(
    requestMessage: platform_pb.GetIdentityBalanceAndRevisionRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityBalanceAndRevisionResponse|null) => void
  ): UnaryResponse;
  getProofs(
    requestMessage: platform_pb.GetProofsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetProofsResponse|null) => void
  ): UnaryResponse;
  getProofs(
    requestMessage: platform_pb.GetProofsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetProofsResponse|null) => void
  ): UnaryResponse;
  getDataContract(
    requestMessage: platform_pb.GetDataContractRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDataContractResponse|null) => void
  ): UnaryResponse;
  getDataContract(
    requestMessage: platform_pb.GetDataContractRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDataContractResponse|null) => void
  ): UnaryResponse;
  getDataContractHistory(
    requestMessage: platform_pb.GetDataContractHistoryRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDataContractHistoryResponse|null) => void
  ): UnaryResponse;
  getDataContractHistory(
    requestMessage: platform_pb.GetDataContractHistoryRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDataContractHistoryResponse|null) => void
  ): UnaryResponse;
  getDataContracts(
    requestMessage: platform_pb.GetDataContractsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDataContractsResponse|null) => void
  ): UnaryResponse;
  getDataContracts(
    requestMessage: platform_pb.GetDataContractsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDataContractsResponse|null) => void
  ): UnaryResponse;
  getDocuments(
    requestMessage: platform_pb.GetDocumentsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDocumentsResponse|null) => void
  ): UnaryResponse;
  getDocuments(
    requestMessage: platform_pb.GetDocumentsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetDocumentsResponse|null) => void
  ): UnaryResponse;
  getIdentitiesByPublicKeyHashes(
    requestMessage: platform_pb.GetIdentitiesByPublicKeyHashesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesByPublicKeyHashesResponse|null) => void
  ): UnaryResponse;
  getIdentitiesByPublicKeyHashes(
    requestMessage: platform_pb.GetIdentitiesByPublicKeyHashesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesByPublicKeyHashesResponse|null) => void
  ): UnaryResponse;
  getIdentityByPublicKeyHash(
    requestMessage: platform_pb.GetIdentityByPublicKeyHashRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityByPublicKeyHashResponse|null) => void
  ): UnaryResponse;
  getIdentityByPublicKeyHash(
    requestMessage: platform_pb.GetIdentityByPublicKeyHashRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityByPublicKeyHashResponse|null) => void
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
  getProtocolVersionUpgradeState(
    requestMessage: platform_pb.GetProtocolVersionUpgradeStateRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetProtocolVersionUpgradeStateResponse|null) => void
  ): UnaryResponse;
  getProtocolVersionUpgradeState(
    requestMessage: platform_pb.GetProtocolVersionUpgradeStateRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetProtocolVersionUpgradeStateResponse|null) => void
  ): UnaryResponse;
  getProtocolVersionUpgradeVoteStatus(
    requestMessage: platform_pb.GetProtocolVersionUpgradeVoteStatusRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetProtocolVersionUpgradeVoteStatusResponse|null) => void
  ): UnaryResponse;
  getProtocolVersionUpgradeVoteStatus(
    requestMessage: platform_pb.GetProtocolVersionUpgradeVoteStatusRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetProtocolVersionUpgradeVoteStatusResponse|null) => void
  ): UnaryResponse;
  getEpochsInfo(
    requestMessage: platform_pb.GetEpochsInfoRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetEpochsInfoResponse|null) => void
  ): UnaryResponse;
  getEpochsInfo(
    requestMessage: platform_pb.GetEpochsInfoRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetEpochsInfoResponse|null) => void
  ): UnaryResponse;
}

