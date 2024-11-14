// package: org.dash.platform.dapi.v0
// file: core.proto

import * as core_pb from "./core_pb";
import {grpc} from "@improbable-eng/grpc-web";

type CoregetBlockchainStatus = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetBlockchainStatusRequest;
  readonly responseType: typeof core_pb.GetBlockchainStatusResponse;
};

type CoregetMasternodeStatus = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetMasternodeStatusRequest;
  readonly responseType: typeof core_pb.GetMasternodeStatusResponse;
};

type CoregetBlock = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetBlockRequest;
  readonly responseType: typeof core_pb.GetBlockResponse;
};

type CoregetBestBlockHeight = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetBestBlockHeightRequest;
  readonly responseType: typeof core_pb.GetBestBlockHeightResponse;
};

type CorebroadcastTransaction = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.BroadcastTransactionRequest;
  readonly responseType: typeof core_pb.BroadcastTransactionResponse;
};

type CoregetTransaction = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetTransactionRequest;
  readonly responseType: typeof core_pb.GetTransactionResponse;
};

type CoregetEstimatedTransactionFee = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetEstimatedTransactionFeeRequest;
  readonly responseType: typeof core_pb.GetEstimatedTransactionFeeResponse;
};

type CoresubscribeToBlockHeadersWithChainLocks = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: true;
  readonly requestType: typeof core_pb.BlockHeadersWithChainLocksRequest;
  readonly responseType: typeof core_pb.BlockHeadersWithChainLocksResponse;
};

type CoresubscribeToTransactionsWithProofs = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: true;
  readonly requestType: typeof core_pb.TransactionsWithProofsRequest;
  readonly responseType: typeof core_pb.TransactionsWithProofsResponse;
};

type CoresubscribeToMasternodeList = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: true;
  readonly requestType: typeof core_pb.MasternodeListRequest;
  readonly responseType: typeof core_pb.MasternodeListResponse;
};

export class Core {
  static readonly serviceName: string;
  static readonly getBlockchainStatus: CoregetBlockchainStatus;
  static readonly getMasternodeStatus: CoregetMasternodeStatus;
  static readonly getBlock: CoregetBlock;
  static readonly getBestBlockHeight: CoregetBestBlockHeight;
  static readonly broadcastTransaction: CorebroadcastTransaction;
  static readonly getTransaction: CoregetTransaction;
  static readonly getEstimatedTransactionFee: CoregetEstimatedTransactionFee;
  static readonly subscribeToBlockHeadersWithChainLocks: CoresubscribeToBlockHeadersWithChainLocks;
  static readonly subscribeToTransactionsWithProofs: CoresubscribeToTransactionsWithProofs;
  static readonly subscribeToMasternodeList: CoresubscribeToMasternodeList;
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

export class CoreClient {
  readonly serviceHost: string;

  constructor(serviceHost: string, options?: grpc.RpcOptions);
  getBlockchainStatus(
    requestMessage: core_pb.GetBlockchainStatusRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetBlockchainStatusResponse|null) => void
  ): UnaryResponse;
  getBlockchainStatus(
    requestMessage: core_pb.GetBlockchainStatusRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetBlockchainStatusResponse|null) => void
  ): UnaryResponse;
  getMasternodeStatus(
    requestMessage: core_pb.GetMasternodeStatusRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetMasternodeStatusResponse|null) => void
  ): UnaryResponse;
  getMasternodeStatus(
    requestMessage: core_pb.GetMasternodeStatusRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetMasternodeStatusResponse|null) => void
  ): UnaryResponse;
  getBlock(
    requestMessage: core_pb.GetBlockRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetBlockResponse|null) => void
  ): UnaryResponse;
  getBlock(
    requestMessage: core_pb.GetBlockRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetBlockResponse|null) => void
  ): UnaryResponse;
  getBestBlockHeight(
    requestMessage: core_pb.GetBestBlockHeightRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetBestBlockHeightResponse|null) => void
  ): UnaryResponse;
  getBestBlockHeight(
    requestMessage: core_pb.GetBestBlockHeightRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetBestBlockHeightResponse|null) => void
  ): UnaryResponse;
  broadcastTransaction(
    requestMessage: core_pb.BroadcastTransactionRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.BroadcastTransactionResponse|null) => void
  ): UnaryResponse;
  broadcastTransaction(
    requestMessage: core_pb.BroadcastTransactionRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.BroadcastTransactionResponse|null) => void
  ): UnaryResponse;
  getTransaction(
    requestMessage: core_pb.GetTransactionRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetTransactionResponse|null) => void
  ): UnaryResponse;
  getTransaction(
    requestMessage: core_pb.GetTransactionRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetTransactionResponse|null) => void
  ): UnaryResponse;
  getEstimatedTransactionFee(
    requestMessage: core_pb.GetEstimatedTransactionFeeRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetEstimatedTransactionFeeResponse|null) => void
  ): UnaryResponse;
  getEstimatedTransactionFee(
    requestMessage: core_pb.GetEstimatedTransactionFeeRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetEstimatedTransactionFeeResponse|null) => void
  ): UnaryResponse;
  subscribeToBlockHeadersWithChainLocks(requestMessage: core_pb.BlockHeadersWithChainLocksRequest, metadata?: grpc.Metadata): ResponseStream<core_pb.BlockHeadersWithChainLocksResponse>;
  subscribeToTransactionsWithProofs(requestMessage: core_pb.TransactionsWithProofsRequest, metadata?: grpc.Metadata): ResponseStream<core_pb.TransactionsWithProofsResponse>;
  subscribeToMasternodeList(requestMessage: core_pb.MasternodeListRequest, metadata?: grpc.Metadata): ResponseStream<core_pb.MasternodeListResponse>;
}

