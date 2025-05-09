// package: org.dash.platform.drive.v0
// file: drive.proto

import * as jspb from "google-protobuf";
import * as platform_v0_platform_pb from "./platform/v0/platform_pb";

export class GetProofsRequest extends jspb.Message {
  getStateTransition(): Uint8Array | string;
  getStateTransition_asU8(): Uint8Array;
  getStateTransition_asB64(): string;
  setStateTransition(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetProofsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetProofsRequest): GetProofsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetProofsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetProofsRequest;
  static deserializeBinaryFromReader(message: GetProofsRequest, reader: jspb.BinaryReader): GetProofsRequest;
}

export namespace GetProofsRequest {
  export type AsObject = {
    stateTransition: Uint8Array | string,
  }
}

export class GetProofsResponse extends jspb.Message {
  hasProof(): boolean;
  clearProof(): void;
  getProof(): platform_v0_platform_pb.Proof | undefined;
  setProof(value?: platform_v0_platform_pb.Proof): void;

  hasMetadata(): boolean;
  clearMetadata(): void;
  getMetadata(): platform_v0_platform_pb.ResponseMetadata | undefined;
  setMetadata(value?: platform_v0_platform_pb.ResponseMetadata): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetProofsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetProofsResponse): GetProofsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetProofsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetProofsResponse;
  static deserializeBinaryFromReader(message: GetProofsResponse, reader: jspb.BinaryReader): GetProofsResponse;
}

export namespace GetProofsResponse {
  export type AsObject = {
    proof?: platform_v0_platform_pb.Proof.AsObject,
    metadata?: platform_v0_platform_pb.ResponseMetadata.AsObject,
  }
}

