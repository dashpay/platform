// package: org.dash.platform.dapi.v0
// file: core.proto

import * as core_pb from "./core_pb";
import {grpc} from "@improbable-eng/grpc-web";

type CoregetStatus = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetStatusRequest;
  readonly responseType: typeof core_pb.GetStatusResponse;
};

type CoregetBlock = {
  readonly methodName: string;
  readonly service: typeof Core;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof core_pb.GetBlockRequest;
  readonly responseType: typeof core_pb.GetBlockResponse;
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

export class Core {
  static readonly serviceName: string;
  static readonly getStatus: CoregetStatus;
  static readonly getBlock: CoregetBlock;
  static readonly broadcastTransaction: CorebroadcastTransaction;
  static readonly getTransaction: CoregetTransaction;
  static readonly getEstimatedTransactionFee: CoregetEstimatedTransactionFee;
  static readonly subscribeToBlockHeadersWithChainLocks: CoresubscribeToBlockHeadersWithChainLocks;
  static readonly subscribeToTransactionsWithProofs: CoresubscribeToTransactionsWithProofs;
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
  getStatus(
    requestMessage: core_pb.GetStatusRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetStatusResponse|null) => void
  ): UnaryResponse;
  getStatus(
    requestMessage: core_pb.GetStatusRequest,
    callback: (error: ServiceError|null, responseMessage: core_pb.GetStatusResponse|null) => void
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
}

