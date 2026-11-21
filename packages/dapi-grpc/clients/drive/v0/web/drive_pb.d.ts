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

export class RegisterStateTransitionExecutionProofRequest extends jspb.Message {
  getStateTransitionHash(): Uint8Array | string;
  getStateTransitionHash_asU8(): Uint8Array;
  getStateTransitionHash_asB64(): string;
  setStateTransitionHash(value: Uint8Array | string): void;

  getLease(): Uint8Array | string;
  getLease_asU8(): Uint8Array;
  getLease_asB64(): string;
  setLease(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): RegisterStateTransitionExecutionProofRequest.AsObject;
  static toObject(includeInstance: boolean, msg: RegisterStateTransitionExecutionProofRequest): RegisterStateTransitionExecutionProofRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: RegisterStateTransitionExecutionProofRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): RegisterStateTransitionExecutionProofRequest;
  static deserializeBinaryFromReader(message: RegisterStateTransitionExecutionProofRequest, reader: jspb.BinaryReader): RegisterStateTransitionExecutionProofRequest;
}

export namespace RegisterStateTransitionExecutionProofRequest {
  export type AsObject = {
    stateTransitionHash: Uint8Array | string,
    lease: Uint8Array | string,
  }
}

export class RegisterStateTransitionExecutionProofResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): RegisterStateTransitionExecutionProofResponse.AsObject;
  static toObject(includeInstance: boolean, msg: RegisterStateTransitionExecutionProofResponse): RegisterStateTransitionExecutionProofResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: RegisterStateTransitionExecutionProofResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): RegisterStateTransitionExecutionProofResponse;
  static deserializeBinaryFromReader(message: RegisterStateTransitionExecutionProofResponse, reader: jspb.BinaryReader): RegisterStateTransitionExecutionProofResponse;
}

export namespace RegisterStateTransitionExecutionProofResponse {
  export type AsObject = {
  }
}

export class UnregisterStateTransitionExecutionProofRequest extends jspb.Message {
  getStateTransitionHash(): Uint8Array | string;
  getStateTransitionHash_asU8(): Uint8Array;
  getStateTransitionHash_asB64(): string;
  setStateTransitionHash(value: Uint8Array | string): void;

  getLease(): Uint8Array | string;
  getLease_asU8(): Uint8Array;
  getLease_asB64(): string;
  setLease(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): UnregisterStateTransitionExecutionProofRequest.AsObject;
  static toObject(includeInstance: boolean, msg: UnregisterStateTransitionExecutionProofRequest): UnregisterStateTransitionExecutionProofRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: UnregisterStateTransitionExecutionProofRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): UnregisterStateTransitionExecutionProofRequest;
  static deserializeBinaryFromReader(message: UnregisterStateTransitionExecutionProofRequest, reader: jspb.BinaryReader): UnregisterStateTransitionExecutionProofRequest;
}

export namespace UnregisterStateTransitionExecutionProofRequest {
  export type AsObject = {
    stateTransitionHash: Uint8Array | string,
    lease: Uint8Array | string,
  }
}

export class UnregisterStateTransitionExecutionProofResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): UnregisterStateTransitionExecutionProofResponse.AsObject;
  static toObject(includeInstance: boolean, msg: UnregisterStateTransitionExecutionProofResponse): UnregisterStateTransitionExecutionProofResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: UnregisterStateTransitionExecutionProofResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): UnregisterStateTransitionExecutionProofResponse;
  static deserializeBinaryFromReader(message: UnregisterStateTransitionExecutionProofResponse, reader: jspb.BinaryReader): UnregisterStateTransitionExecutionProofResponse;
}

export namespace UnregisterStateTransitionExecutionProofResponse {
  export type AsObject = {
  }
}

export class GetStateTransitionExecutionProofRequest extends jspb.Message {
  getStateTransitionHash(): Uint8Array | string;
  getStateTransitionHash_asU8(): Uint8Array;
  getStateTransitionHash_asB64(): string;
  setStateTransitionHash(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetStateTransitionExecutionProofRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetStateTransitionExecutionProofRequest): GetStateTransitionExecutionProofRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetStateTransitionExecutionProofRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetStateTransitionExecutionProofRequest;
  static deserializeBinaryFromReader(message: GetStateTransitionExecutionProofRequest, reader: jspb.BinaryReader): GetStateTransitionExecutionProofRequest;
}

export namespace GetStateTransitionExecutionProofRequest {
  export type AsObject = {
    stateTransitionHash: Uint8Array | string,
  }
}

export class GetStateTransitionExecutionProofResponse extends jspb.Message {
  hasBefore(): boolean;
  clearBefore(): void;
  getBefore(): GetStateTransitionExecutionProofResponse.StateAtBlock | undefined;
  setBefore(value?: GetStateTransitionExecutionProofResponse.StateAtBlock): void;

  hasAfter(): boolean;
  clearAfter(): void;
  getAfter(): GetStateTransitionExecutionProofResponse.StateAtBlock | undefined;
  setAfter(value?: GetStateTransitionExecutionProofResponse.StateAtBlock): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetStateTransitionExecutionProofResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetStateTransitionExecutionProofResponse): GetStateTransitionExecutionProofResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetStateTransitionExecutionProofResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetStateTransitionExecutionProofResponse;
  static deserializeBinaryFromReader(message: GetStateTransitionExecutionProofResponse, reader: jspb.BinaryReader): GetStateTransitionExecutionProofResponse;
}

export namespace GetStateTransitionExecutionProofResponse {
  export type AsObject = {
    before?: GetStateTransitionExecutionProofResponse.StateAtBlock.AsObject,
    after?: GetStateTransitionExecutionProofResponse.StateAtBlock.AsObject,
  }

  export class StateAtBlock extends jspb.Message {
    hasProof(): boolean;
    clearProof(): void;
    getProof(): platform_v0_platform_pb.Proof | undefined;
    setProof(value?: platform_v0_platform_pb.Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): platform_v0_platform_pb.ResponseMetadata | undefined;
    setMetadata(value?: platform_v0_platform_pb.ResponseMetadata): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): StateAtBlock.AsObject;
    static toObject(includeInstance: boolean, msg: StateAtBlock): StateAtBlock.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: StateAtBlock, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): StateAtBlock;
    static deserializeBinaryFromReader(message: StateAtBlock, reader: jspb.BinaryReader): StateAtBlock;
  }

  export namespace StateAtBlock {
    export type AsObject = {
      proof?: platform_v0_platform_pb.Proof.AsObject,
      metadata?: platform_v0_platform_pb.ResponseMetadata.AsObject,
    }
  }
}

