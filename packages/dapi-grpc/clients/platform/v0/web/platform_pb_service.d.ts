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

type PlatformgetIdentityKeys = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityKeysRequest;
  readonly responseType: typeof platform_pb.GetIdentityKeysResponse;
};

type PlatformgetIdentitiesContractKeys = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentitiesContractKeysRequest;
  readonly responseType: typeof platform_pb.GetIdentitiesContractKeysResponse;
};

type PlatformgetIdentityNonce = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityNonceRequest;
  readonly responseType: typeof platform_pb.GetIdentityNonceResponse;
};

type PlatformgetIdentityContractNonce = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityContractNonceRequest;
  readonly responseType: typeof platform_pb.GetIdentityContractNonceResponse;
};

type PlatformgetIdentityBalance = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityBalanceRequest;
  readonly responseType: typeof platform_pb.GetIdentityBalanceResponse;
};

type PlatformgetIdentitiesBalances = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentitiesBalancesRequest;
  readonly responseType: typeof platform_pb.GetIdentitiesBalancesResponse;
};

type PlatformgetIdentityBalanceAndRevision = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityBalanceAndRevisionRequest;
  readonly responseType: typeof platform_pb.GetIdentityBalanceAndRevisionResponse;
};

type PlatformgetEvonodesProposedEpochBlocksByIds = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetEvonodesProposedEpochBlocksByIdsRequest;
  readonly responseType: typeof platform_pb.GetEvonodesProposedEpochBlocksResponse;
};

type PlatformgetEvonodesProposedEpochBlocksByRange = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetEvonodesProposedEpochBlocksByRangeRequest;
  readonly responseType: typeof platform_pb.GetEvonodesProposedEpochBlocksResponse;
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

type PlatformgetIdentityByPublicKeyHash = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityByPublicKeyHashRequest;
  readonly responseType: typeof platform_pb.GetIdentityByPublicKeyHashResponse;
};

type PlatformgetIdentityByNonUniquePublicKeyHash = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityByNonUniquePublicKeyHashRequest;
  readonly responseType: typeof platform_pb.GetIdentityByNonUniquePublicKeyHashResponse;
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

type PlatformgetFinalizedEpochInfos = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetFinalizedEpochInfosRequest;
  readonly responseType: typeof platform_pb.GetFinalizedEpochInfosResponse;
};

type PlatformgetContestedResources = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetContestedResourcesRequest;
  readonly responseType: typeof platform_pb.GetContestedResourcesResponse;
};

type PlatformgetContestedResourceVoteState = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetContestedResourceVoteStateRequest;
  readonly responseType: typeof platform_pb.GetContestedResourceVoteStateResponse;
};

type PlatformgetContestedResourceVotersForIdentity = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetContestedResourceVotersForIdentityRequest;
  readonly responseType: typeof platform_pb.GetContestedResourceVotersForIdentityResponse;
};

type PlatformgetContestedResourceIdentityVotes = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetContestedResourceIdentityVotesRequest;
  readonly responseType: typeof platform_pb.GetContestedResourceIdentityVotesResponse;
};

type PlatformgetVotePollsByEndDate = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetVotePollsByEndDateRequest;
  readonly responseType: typeof platform_pb.GetVotePollsByEndDateResponse;
};

type PlatformgetPrefundedSpecializedBalance = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetPrefundedSpecializedBalanceRequest;
  readonly responseType: typeof platform_pb.GetPrefundedSpecializedBalanceResponse;
};

type PlatformgetTotalCreditsInPlatform = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTotalCreditsInPlatformRequest;
  readonly responseType: typeof platform_pb.GetTotalCreditsInPlatformResponse;
};

type PlatformgetPathElements = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetPathElementsRequest;
  readonly responseType: typeof platform_pb.GetPathElementsResponse;
};

type PlatformgetStatus = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetStatusRequest;
  readonly responseType: typeof platform_pb.GetStatusResponse;
};

type PlatformgetCurrentQuorumsInfo = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetCurrentQuorumsInfoRequest;
  readonly responseType: typeof platform_pb.GetCurrentQuorumsInfoResponse;
};

type PlatformgetIdentityTokenBalances = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityTokenBalancesRequest;
  readonly responseType: typeof platform_pb.GetIdentityTokenBalancesResponse;
};

type PlatformgetIdentitiesTokenBalances = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentitiesTokenBalancesRequest;
  readonly responseType: typeof platform_pb.GetIdentitiesTokenBalancesResponse;
};

type PlatformgetIdentityTokenInfos = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentityTokenInfosRequest;
  readonly responseType: typeof platform_pb.GetIdentityTokenInfosResponse;
};

type PlatformgetIdentitiesTokenInfos = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetIdentitiesTokenInfosRequest;
  readonly responseType: typeof platform_pb.GetIdentitiesTokenInfosResponse;
};

type PlatformgetTokenStatuses = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTokenStatusesRequest;
  readonly responseType: typeof platform_pb.GetTokenStatusesResponse;
};

type PlatformgetTokenDirectPurchasePrices = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTokenDirectPurchasePricesRequest;
  readonly responseType: typeof platform_pb.GetTokenDirectPurchasePricesResponse;
};

type PlatformgetTokenContractInfo = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTokenContractInfoRequest;
  readonly responseType: typeof platform_pb.GetTokenContractInfoResponse;
};

type PlatformgetTokenPreProgrammedDistributions = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTokenPreProgrammedDistributionsRequest;
  readonly responseType: typeof platform_pb.GetTokenPreProgrammedDistributionsResponse;
};

type PlatformgetTokenPerpetualDistributionLastClaim = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTokenPerpetualDistributionLastClaimRequest;
  readonly responseType: typeof platform_pb.GetTokenPerpetualDistributionLastClaimResponse;
};

type PlatformgetTokenTotalSupply = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetTokenTotalSupplyRequest;
  readonly responseType: typeof platform_pb.GetTokenTotalSupplyResponse;
};

type PlatformgetGroupInfo = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetGroupInfoRequest;
  readonly responseType: typeof platform_pb.GetGroupInfoResponse;
};

type PlatformgetGroupInfos = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetGroupInfosRequest;
  readonly responseType: typeof platform_pb.GetGroupInfosResponse;
};

type PlatformgetGroupActions = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetGroupActionsRequest;
  readonly responseType: typeof platform_pb.GetGroupActionsResponse;
};

type PlatformgetGroupActionSigners = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof platform_pb.GetGroupActionSignersRequest;
  readonly responseType: typeof platform_pb.GetGroupActionSignersResponse;
};

type PlatformSubscribePlatformEvents = {
  readonly methodName: string;
  readonly service: typeof Platform;
  readonly requestStream: false;
  readonly responseStream: true;
  readonly requestType: typeof platform_pb.PlatformSubscriptionRequest;
  readonly responseType: typeof platform_pb.PlatformSubscriptionResponse;
};

export class Platform {
  static readonly serviceName: string;
  static readonly broadcastStateTransition: PlatformbroadcastStateTransition;
  static readonly getIdentity: PlatformgetIdentity;
  static readonly getIdentityKeys: PlatformgetIdentityKeys;
  static readonly getIdentitiesContractKeys: PlatformgetIdentitiesContractKeys;
  static readonly getIdentityNonce: PlatformgetIdentityNonce;
  static readonly getIdentityContractNonce: PlatformgetIdentityContractNonce;
  static readonly getIdentityBalance: PlatformgetIdentityBalance;
  static readonly getIdentitiesBalances: PlatformgetIdentitiesBalances;
  static readonly getIdentityBalanceAndRevision: PlatformgetIdentityBalanceAndRevision;
  static readonly getEvonodesProposedEpochBlocksByIds: PlatformgetEvonodesProposedEpochBlocksByIds;
  static readonly getEvonodesProposedEpochBlocksByRange: PlatformgetEvonodesProposedEpochBlocksByRange;
  static readonly getDataContract: PlatformgetDataContract;
  static readonly getDataContractHistory: PlatformgetDataContractHistory;
  static readonly getDataContracts: PlatformgetDataContracts;
  static readonly getDocuments: PlatformgetDocuments;
  static readonly getIdentityByPublicKeyHash: PlatformgetIdentityByPublicKeyHash;
  static readonly getIdentityByNonUniquePublicKeyHash: PlatformgetIdentityByNonUniquePublicKeyHash;
  static readonly waitForStateTransitionResult: PlatformwaitForStateTransitionResult;
  static readonly getConsensusParams: PlatformgetConsensusParams;
  static readonly getProtocolVersionUpgradeState: PlatformgetProtocolVersionUpgradeState;
  static readonly getProtocolVersionUpgradeVoteStatus: PlatformgetProtocolVersionUpgradeVoteStatus;
  static readonly getEpochsInfo: PlatformgetEpochsInfo;
  static readonly getFinalizedEpochInfos: PlatformgetFinalizedEpochInfos;
  static readonly getContestedResources: PlatformgetContestedResources;
  static readonly getContestedResourceVoteState: PlatformgetContestedResourceVoteState;
  static readonly getContestedResourceVotersForIdentity: PlatformgetContestedResourceVotersForIdentity;
  static readonly getContestedResourceIdentityVotes: PlatformgetContestedResourceIdentityVotes;
  static readonly getVotePollsByEndDate: PlatformgetVotePollsByEndDate;
  static readonly getPrefundedSpecializedBalance: PlatformgetPrefundedSpecializedBalance;
  static readonly getTotalCreditsInPlatform: PlatformgetTotalCreditsInPlatform;
  static readonly getPathElements: PlatformgetPathElements;
  static readonly getStatus: PlatformgetStatus;
  static readonly getCurrentQuorumsInfo: PlatformgetCurrentQuorumsInfo;
  static readonly getIdentityTokenBalances: PlatformgetIdentityTokenBalances;
  static readonly getIdentitiesTokenBalances: PlatformgetIdentitiesTokenBalances;
  static readonly getIdentityTokenInfos: PlatformgetIdentityTokenInfos;
  static readonly getIdentitiesTokenInfos: PlatformgetIdentitiesTokenInfos;
  static readonly getTokenStatuses: PlatformgetTokenStatuses;
  static readonly getTokenDirectPurchasePrices: PlatformgetTokenDirectPurchasePrices;
  static readonly getTokenContractInfo: PlatformgetTokenContractInfo;
  static readonly getTokenPreProgrammedDistributions: PlatformgetTokenPreProgrammedDistributions;
  static readonly getTokenPerpetualDistributionLastClaim: PlatformgetTokenPerpetualDistributionLastClaim;
  static readonly getTokenTotalSupply: PlatformgetTokenTotalSupply;
  static readonly getGroupInfo: PlatformgetGroupInfo;
  static readonly getGroupInfos: PlatformgetGroupInfos;
  static readonly getGroupActions: PlatformgetGroupActions;
  static readonly getGroupActionSigners: PlatformgetGroupActionSigners;
  static readonly SubscribePlatformEvents: PlatformSubscribePlatformEvents;
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
  getIdentityKeys(
    requestMessage: platform_pb.GetIdentityKeysRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityKeysResponse|null) => void
  ): UnaryResponse;
  getIdentityKeys(
    requestMessage: platform_pb.GetIdentityKeysRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityKeysResponse|null) => void
  ): UnaryResponse;
  getIdentitiesContractKeys(
    requestMessage: platform_pb.GetIdentitiesContractKeysRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesContractKeysResponse|null) => void
  ): UnaryResponse;
  getIdentitiesContractKeys(
    requestMessage: platform_pb.GetIdentitiesContractKeysRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesContractKeysResponse|null) => void
  ): UnaryResponse;
  getIdentityNonce(
    requestMessage: platform_pb.GetIdentityNonceRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityNonceResponse|null) => void
  ): UnaryResponse;
  getIdentityNonce(
    requestMessage: platform_pb.GetIdentityNonceRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityNonceResponse|null) => void
  ): UnaryResponse;
  getIdentityContractNonce(
    requestMessage: platform_pb.GetIdentityContractNonceRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityContractNonceResponse|null) => void
  ): UnaryResponse;
  getIdentityContractNonce(
    requestMessage: platform_pb.GetIdentityContractNonceRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityContractNonceResponse|null) => void
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
  getIdentitiesBalances(
    requestMessage: platform_pb.GetIdentitiesBalancesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesBalancesResponse|null) => void
  ): UnaryResponse;
  getIdentitiesBalances(
    requestMessage: platform_pb.GetIdentitiesBalancesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesBalancesResponse|null) => void
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
  getEvonodesProposedEpochBlocksByIds(
    requestMessage: platform_pb.GetEvonodesProposedEpochBlocksByIdsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetEvonodesProposedEpochBlocksResponse|null) => void
  ): UnaryResponse;
  getEvonodesProposedEpochBlocksByIds(
    requestMessage: platform_pb.GetEvonodesProposedEpochBlocksByIdsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetEvonodesProposedEpochBlocksResponse|null) => void
  ): UnaryResponse;
  getEvonodesProposedEpochBlocksByRange(
    requestMessage: platform_pb.GetEvonodesProposedEpochBlocksByRangeRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetEvonodesProposedEpochBlocksResponse|null) => void
  ): UnaryResponse;
  getEvonodesProposedEpochBlocksByRange(
    requestMessage: platform_pb.GetEvonodesProposedEpochBlocksByRangeRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetEvonodesProposedEpochBlocksResponse|null) => void
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
  getIdentityByPublicKeyHash(
    requestMessage: platform_pb.GetIdentityByPublicKeyHashRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityByPublicKeyHashResponse|null) => void
  ): UnaryResponse;
  getIdentityByPublicKeyHash(
    requestMessage: platform_pb.GetIdentityByPublicKeyHashRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityByPublicKeyHashResponse|null) => void
  ): UnaryResponse;
  getIdentityByNonUniquePublicKeyHash(
    requestMessage: platform_pb.GetIdentityByNonUniquePublicKeyHashRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityByNonUniquePublicKeyHashResponse|null) => void
  ): UnaryResponse;
  getIdentityByNonUniquePublicKeyHash(
    requestMessage: platform_pb.GetIdentityByNonUniquePublicKeyHashRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityByNonUniquePublicKeyHashResponse|null) => void
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
  getFinalizedEpochInfos(
    requestMessage: platform_pb.GetFinalizedEpochInfosRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetFinalizedEpochInfosResponse|null) => void
  ): UnaryResponse;
  getFinalizedEpochInfos(
    requestMessage: platform_pb.GetFinalizedEpochInfosRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetFinalizedEpochInfosResponse|null) => void
  ): UnaryResponse;
  getContestedResources(
    requestMessage: platform_pb.GetContestedResourcesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourcesResponse|null) => void
  ): UnaryResponse;
  getContestedResources(
    requestMessage: platform_pb.GetContestedResourcesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourcesResponse|null) => void
  ): UnaryResponse;
  getContestedResourceVoteState(
    requestMessage: platform_pb.GetContestedResourceVoteStateRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourceVoteStateResponse|null) => void
  ): UnaryResponse;
  getContestedResourceVoteState(
    requestMessage: platform_pb.GetContestedResourceVoteStateRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourceVoteStateResponse|null) => void
  ): UnaryResponse;
  getContestedResourceVotersForIdentity(
    requestMessage: platform_pb.GetContestedResourceVotersForIdentityRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourceVotersForIdentityResponse|null) => void
  ): UnaryResponse;
  getContestedResourceVotersForIdentity(
    requestMessage: platform_pb.GetContestedResourceVotersForIdentityRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourceVotersForIdentityResponse|null) => void
  ): UnaryResponse;
  getContestedResourceIdentityVotes(
    requestMessage: platform_pb.GetContestedResourceIdentityVotesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourceIdentityVotesResponse|null) => void
  ): UnaryResponse;
  getContestedResourceIdentityVotes(
    requestMessage: platform_pb.GetContestedResourceIdentityVotesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetContestedResourceIdentityVotesResponse|null) => void
  ): UnaryResponse;
  getVotePollsByEndDate(
    requestMessage: platform_pb.GetVotePollsByEndDateRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetVotePollsByEndDateResponse|null) => void
  ): UnaryResponse;
  getVotePollsByEndDate(
    requestMessage: platform_pb.GetVotePollsByEndDateRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetVotePollsByEndDateResponse|null) => void
  ): UnaryResponse;
  getPrefundedSpecializedBalance(
    requestMessage: platform_pb.GetPrefundedSpecializedBalanceRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetPrefundedSpecializedBalanceResponse|null) => void
  ): UnaryResponse;
  getPrefundedSpecializedBalance(
    requestMessage: platform_pb.GetPrefundedSpecializedBalanceRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetPrefundedSpecializedBalanceResponse|null) => void
  ): UnaryResponse;
  getTotalCreditsInPlatform(
    requestMessage: platform_pb.GetTotalCreditsInPlatformRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTotalCreditsInPlatformResponse|null) => void
  ): UnaryResponse;
  getTotalCreditsInPlatform(
    requestMessage: platform_pb.GetTotalCreditsInPlatformRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTotalCreditsInPlatformResponse|null) => void
  ): UnaryResponse;
  getPathElements(
    requestMessage: platform_pb.GetPathElementsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetPathElementsResponse|null) => void
  ): UnaryResponse;
  getPathElements(
    requestMessage: platform_pb.GetPathElementsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetPathElementsResponse|null) => void
  ): UnaryResponse;
  getStatus(
    requestMessage: platform_pb.GetStatusRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetStatusResponse|null) => void
  ): UnaryResponse;
  getStatus(
    requestMessage: platform_pb.GetStatusRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetStatusResponse|null) => void
  ): UnaryResponse;
  getCurrentQuorumsInfo(
    requestMessage: platform_pb.GetCurrentQuorumsInfoRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetCurrentQuorumsInfoResponse|null) => void
  ): UnaryResponse;
  getCurrentQuorumsInfo(
    requestMessage: platform_pb.GetCurrentQuorumsInfoRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetCurrentQuorumsInfoResponse|null) => void
  ): UnaryResponse;
  getIdentityTokenBalances(
    requestMessage: platform_pb.GetIdentityTokenBalancesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityTokenBalancesResponse|null) => void
  ): UnaryResponse;
  getIdentityTokenBalances(
    requestMessage: platform_pb.GetIdentityTokenBalancesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityTokenBalancesResponse|null) => void
  ): UnaryResponse;
  getIdentitiesTokenBalances(
    requestMessage: platform_pb.GetIdentitiesTokenBalancesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesTokenBalancesResponse|null) => void
  ): UnaryResponse;
  getIdentitiesTokenBalances(
    requestMessage: platform_pb.GetIdentitiesTokenBalancesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesTokenBalancesResponse|null) => void
  ): UnaryResponse;
  getIdentityTokenInfos(
    requestMessage: platform_pb.GetIdentityTokenInfosRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityTokenInfosResponse|null) => void
  ): UnaryResponse;
  getIdentityTokenInfos(
    requestMessage: platform_pb.GetIdentityTokenInfosRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentityTokenInfosResponse|null) => void
  ): UnaryResponse;
  getIdentitiesTokenInfos(
    requestMessage: platform_pb.GetIdentitiesTokenInfosRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesTokenInfosResponse|null) => void
  ): UnaryResponse;
  getIdentitiesTokenInfos(
    requestMessage: platform_pb.GetIdentitiesTokenInfosRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetIdentitiesTokenInfosResponse|null) => void
  ): UnaryResponse;
  getTokenStatuses(
    requestMessage: platform_pb.GetTokenStatusesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenStatusesResponse|null) => void
  ): UnaryResponse;
  getTokenStatuses(
    requestMessage: platform_pb.GetTokenStatusesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenStatusesResponse|null) => void
  ): UnaryResponse;
  getTokenDirectPurchasePrices(
    requestMessage: platform_pb.GetTokenDirectPurchasePricesRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenDirectPurchasePricesResponse|null) => void
  ): UnaryResponse;
  getTokenDirectPurchasePrices(
    requestMessage: platform_pb.GetTokenDirectPurchasePricesRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenDirectPurchasePricesResponse|null) => void
  ): UnaryResponse;
  getTokenContractInfo(
    requestMessage: platform_pb.GetTokenContractInfoRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenContractInfoResponse|null) => void
  ): UnaryResponse;
  getTokenContractInfo(
    requestMessage: platform_pb.GetTokenContractInfoRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenContractInfoResponse|null) => void
  ): UnaryResponse;
  getTokenPreProgrammedDistributions(
    requestMessage: platform_pb.GetTokenPreProgrammedDistributionsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenPreProgrammedDistributionsResponse|null) => void
  ): UnaryResponse;
  getTokenPreProgrammedDistributions(
    requestMessage: platform_pb.GetTokenPreProgrammedDistributionsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenPreProgrammedDistributionsResponse|null) => void
  ): UnaryResponse;
  getTokenPerpetualDistributionLastClaim(
    requestMessage: platform_pb.GetTokenPerpetualDistributionLastClaimRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenPerpetualDistributionLastClaimResponse|null) => void
  ): UnaryResponse;
  getTokenPerpetualDistributionLastClaim(
    requestMessage: platform_pb.GetTokenPerpetualDistributionLastClaimRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenPerpetualDistributionLastClaimResponse|null) => void
  ): UnaryResponse;
  getTokenTotalSupply(
    requestMessage: platform_pb.GetTokenTotalSupplyRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenTotalSupplyResponse|null) => void
  ): UnaryResponse;
  getTokenTotalSupply(
    requestMessage: platform_pb.GetTokenTotalSupplyRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetTokenTotalSupplyResponse|null) => void
  ): UnaryResponse;
  getGroupInfo(
    requestMessage: platform_pb.GetGroupInfoRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupInfoResponse|null) => void
  ): UnaryResponse;
  getGroupInfo(
    requestMessage: platform_pb.GetGroupInfoRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupInfoResponse|null) => void
  ): UnaryResponse;
  getGroupInfos(
    requestMessage: platform_pb.GetGroupInfosRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupInfosResponse|null) => void
  ): UnaryResponse;
  getGroupInfos(
    requestMessage: platform_pb.GetGroupInfosRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupInfosResponse|null) => void
  ): UnaryResponse;
  getGroupActions(
    requestMessage: platform_pb.GetGroupActionsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupActionsResponse|null) => void
  ): UnaryResponse;
  getGroupActions(
    requestMessage: platform_pb.GetGroupActionsRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupActionsResponse|null) => void
  ): UnaryResponse;
  getGroupActionSigners(
    requestMessage: platform_pb.GetGroupActionSignersRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupActionSignersResponse|null) => void
  ): UnaryResponse;
  getGroupActionSigners(
    requestMessage: platform_pb.GetGroupActionSignersRequest,
    callback: (error: ServiceError|null, responseMessage: platform_pb.GetGroupActionSignersResponse|null) => void
  ): UnaryResponse;
  subscribePlatformEvents(requestMessage: platform_pb.PlatformSubscriptionRequest, metadata?: grpc.Metadata): ResponseStream<platform_pb.PlatformSubscriptionResponse>;
}

