// package: org.dash.platform.drive.v0
// file: drive.proto

import * as drive_pb from "./drive_pb";
import {grpc} from "@improbable-eng/grpc-web";

type DriveInternalgetProofs = {
  readonly methodName: string;
  readonly service: typeof DriveInternal;
  readonly requestStream: false;
  readonly responseStream: false;
  readonly requestType: typeof drive_pb.GetProofsRequest;
  readonly responseType: typeof drive_pb.GetProofsResponse;
};

export class DriveInternal {
  static readonly serviceName: string;
  static readonly getProofs: DriveInternalgetProofs;
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

export class DriveInternalClient {
  readonly serviceHost: string;

  constructor(serviceHost: string, options?: grpc.RpcOptions);
  getProofs(
    requestMessage: drive_pb.GetProofsRequest,
    metadata: grpc.Metadata,
    callback: (error: ServiceError|null, responseMessage: drive_pb.GetProofsResponse|null) => void
  ): UnaryResponse;
  getProofs(
    requestMessage: drive_pb.GetProofsRequest,
    callback: (error: ServiceError|null, responseMessage: drive_pb.GetProofsResponse|null) => void
  ): UnaryResponse;
}

