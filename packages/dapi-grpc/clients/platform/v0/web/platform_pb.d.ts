// package: org.dash.platform.dapi.v0
// file: platform.proto

import * as jspb from "google-protobuf";
import * as google_protobuf_wrappers_pb from "google-protobuf/google/protobuf/wrappers_pb";
import * as google_protobuf_struct_pb from "google-protobuf/google/protobuf/struct_pb";
import * as google_protobuf_timestamp_pb from "google-protobuf/google/protobuf/timestamp_pb";

export class Proof extends jspb.Message {
  getGrovedbProof(): Uint8Array | string;
  getGrovedbProof_asU8(): Uint8Array;
  getGrovedbProof_asB64(): string;
  setGrovedbProof(value: Uint8Array | string): void;

  getQuorumHash(): Uint8Array | string;
  getQuorumHash_asU8(): Uint8Array;
  getQuorumHash_asB64(): string;
  setQuorumHash(value: Uint8Array | string): void;

  getSignature(): Uint8Array | string;
  getSignature_asU8(): Uint8Array;
  getSignature_asB64(): string;
  setSignature(value: Uint8Array | string): void;

  getRound(): number;
  setRound(value: number): void;

  getBlockIdHash(): Uint8Array | string;
  getBlockIdHash_asU8(): Uint8Array;
  getBlockIdHash_asB64(): string;
  setBlockIdHash(value: Uint8Array | string): void;

  getQuorumType(): number;
  setQuorumType(value: number): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): Proof.AsObject;
  static toObject(includeInstance: boolean, msg: Proof): Proof.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: Proof, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): Proof;
  static deserializeBinaryFromReader(message: Proof, reader: jspb.BinaryReader): Proof;
}

export namespace Proof {
  export type AsObject = {
    grovedbProof: Uint8Array | string,
    quorumHash: Uint8Array | string,
    signature: Uint8Array | string,
    round: number,
    blockIdHash: Uint8Array | string,
    quorumType: number,
  }
}

export class ResponseMetadata extends jspb.Message {
  getHeight(): number;
  setHeight(value: number): void;

  getCoreChainLockedHeight(): number;
  setCoreChainLockedHeight(value: number): void;

  getEpoch(): number;
  setEpoch(value: number): void;

  getTimeMs(): number;
  setTimeMs(value: number): void;

  getProtocolVersion(): number;
  setProtocolVersion(value: number): void;

  getChainId(): string;
  setChainId(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): ResponseMetadata.AsObject;
  static toObject(includeInstance: boolean, msg: ResponseMetadata): ResponseMetadata.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: ResponseMetadata, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): ResponseMetadata;
  static deserializeBinaryFromReader(message: ResponseMetadata, reader: jspb.BinaryReader): ResponseMetadata;
}

export namespace ResponseMetadata {
  export type AsObject = {
    height: number,
    coreChainLockedHeight: number,
    epoch: number,
    timeMs: number,
    protocolVersion: number,
    chainId: string,
  }
}

export class StateTransitionBroadcastError extends jspb.Message {
  getCode(): number;
  setCode(value: number): void;

  getMessage(): string;
  setMessage(value: string): void;

  getData(): Uint8Array | string;
  getData_asU8(): Uint8Array;
  getData_asB64(): string;
  setData(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): StateTransitionBroadcastError.AsObject;
  static toObject(includeInstance: boolean, msg: StateTransitionBroadcastError): StateTransitionBroadcastError.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: StateTransitionBroadcastError, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): StateTransitionBroadcastError;
  static deserializeBinaryFromReader(message: StateTransitionBroadcastError, reader: jspb.BinaryReader): StateTransitionBroadcastError;
}

export namespace StateTransitionBroadcastError {
  export type AsObject = {
    code: number,
    message: string,
    data: Uint8Array | string,
  }
}

export class BroadcastStateTransitionRequest extends jspb.Message {
  getStateTransition(): Uint8Array | string;
  getStateTransition_asU8(): Uint8Array;
  getStateTransition_asB64(): string;
  setStateTransition(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BroadcastStateTransitionRequest.AsObject;
  static toObject(includeInstance: boolean, msg: BroadcastStateTransitionRequest): BroadcastStateTransitionRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BroadcastStateTransitionRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BroadcastStateTransitionRequest;
  static deserializeBinaryFromReader(message: BroadcastStateTransitionRequest, reader: jspb.BinaryReader): BroadcastStateTransitionRequest;
}

export namespace BroadcastStateTransitionRequest {
  export type AsObject = {
    stateTransition: Uint8Array | string,
  }
}

export class BroadcastStateTransitionResponse extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BroadcastStateTransitionResponse.AsObject;
  static toObject(includeInstance: boolean, msg: BroadcastStateTransitionResponse): BroadcastStateTransitionResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BroadcastStateTransitionResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BroadcastStateTransitionResponse;
  static deserializeBinaryFromReader(message: BroadcastStateTransitionResponse, reader: jspb.BinaryReader): BroadcastStateTransitionResponse;
}

export namespace BroadcastStateTransitionResponse {
  export type AsObject = {
  }
}

export class GetIdentityRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityRequest.GetIdentityRequestV0 | undefined;
  setV0(value?: GetIdentityRequest.GetIdentityRequestV0): void;

  getVersionCase(): GetIdentityRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityRequest): GetIdentityRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityRequest;
  static deserializeBinaryFromReader(message: GetIdentityRequest, reader: jspb.BinaryReader): GetIdentityRequest;
}

export namespace GetIdentityRequest {
  export type AsObject = {
    v0?: GetIdentityRequest.GetIdentityRequestV0.AsObject,
  }

  export class GetIdentityRequestV0 extends jspb.Message {
    getId(): Uint8Array | string;
    getId_asU8(): Uint8Array;
    getId_asB64(): string;
    setId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityRequestV0): GetIdentityRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityRequestV0, reader: jspb.BinaryReader): GetIdentityRequestV0;
  }

  export namespace GetIdentityRequestV0 {
    export type AsObject = {
      id: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityNonceRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityNonceRequest.GetIdentityNonceRequestV0 | undefined;
  setV0(value?: GetIdentityNonceRequest.GetIdentityNonceRequestV0): void;

  getVersionCase(): GetIdentityNonceRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityNonceRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityNonceRequest): GetIdentityNonceRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityNonceRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityNonceRequest;
  static deserializeBinaryFromReader(message: GetIdentityNonceRequest, reader: jspb.BinaryReader): GetIdentityNonceRequest;
}

export namespace GetIdentityNonceRequest {
  export type AsObject = {
    v0?: GetIdentityNonceRequest.GetIdentityNonceRequestV0.AsObject,
  }

  export class GetIdentityNonceRequestV0 extends jspb.Message {
    getIdentityId(): Uint8Array | string;
    getIdentityId_asU8(): Uint8Array;
    getIdentityId_asB64(): string;
    setIdentityId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityNonceRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityNonceRequestV0): GetIdentityNonceRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityNonceRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityNonceRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityNonceRequestV0, reader: jspb.BinaryReader): GetIdentityNonceRequestV0;
  }

  export namespace GetIdentityNonceRequestV0 {
    export type AsObject = {
      identityId: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityContractNonceRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityContractNonceRequest.GetIdentityContractNonceRequestV0 | undefined;
  setV0(value?: GetIdentityContractNonceRequest.GetIdentityContractNonceRequestV0): void;

  getVersionCase(): GetIdentityContractNonceRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityContractNonceRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityContractNonceRequest): GetIdentityContractNonceRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityContractNonceRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityContractNonceRequest;
  static deserializeBinaryFromReader(message: GetIdentityContractNonceRequest, reader: jspb.BinaryReader): GetIdentityContractNonceRequest;
}

export namespace GetIdentityContractNonceRequest {
  export type AsObject = {
    v0?: GetIdentityContractNonceRequest.GetIdentityContractNonceRequestV0.AsObject,
  }

  export class GetIdentityContractNonceRequestV0 extends jspb.Message {
    getIdentityId(): Uint8Array | string;
    getIdentityId_asU8(): Uint8Array;
    getIdentityId_asB64(): string;
    setIdentityId(value: Uint8Array | string): void;

    getContractId(): Uint8Array | string;
    getContractId_asU8(): Uint8Array;
    getContractId_asB64(): string;
    setContractId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityContractNonceRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityContractNonceRequestV0): GetIdentityContractNonceRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityContractNonceRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityContractNonceRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityContractNonceRequestV0, reader: jspb.BinaryReader): GetIdentityContractNonceRequestV0;
  }

  export namespace GetIdentityContractNonceRequestV0 {
    export type AsObject = {
      identityId: Uint8Array | string,
      contractId: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityBalanceRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityBalanceRequest.GetIdentityBalanceRequestV0 | undefined;
  setV0(value?: GetIdentityBalanceRequest.GetIdentityBalanceRequestV0): void;

  getVersionCase(): GetIdentityBalanceRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityBalanceRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityBalanceRequest): GetIdentityBalanceRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityBalanceRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceRequest;
  static deserializeBinaryFromReader(message: GetIdentityBalanceRequest, reader: jspb.BinaryReader): GetIdentityBalanceRequest;
}

export namespace GetIdentityBalanceRequest {
  export type AsObject = {
    v0?: GetIdentityBalanceRequest.GetIdentityBalanceRequestV0.AsObject,
  }

  export class GetIdentityBalanceRequestV0 extends jspb.Message {
    getId(): Uint8Array | string;
    getId_asU8(): Uint8Array;
    getId_asB64(): string;
    setId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityBalanceRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityBalanceRequestV0): GetIdentityBalanceRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityBalanceRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityBalanceRequestV0, reader: jspb.BinaryReader): GetIdentityBalanceRequestV0;
  }

  export namespace GetIdentityBalanceRequestV0 {
    export type AsObject = {
      id: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityBalanceAndRevisionRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0 | undefined;
  setV0(value?: GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0): void;

  getVersionCase(): GetIdentityBalanceAndRevisionRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityBalanceAndRevisionRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityBalanceAndRevisionRequest): GetIdentityBalanceAndRevisionRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityBalanceAndRevisionRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceAndRevisionRequest;
  static deserializeBinaryFromReader(message: GetIdentityBalanceAndRevisionRequest, reader: jspb.BinaryReader): GetIdentityBalanceAndRevisionRequest;
}

export namespace GetIdentityBalanceAndRevisionRequest {
  export type AsObject = {
    v0?: GetIdentityBalanceAndRevisionRequest.GetIdentityBalanceAndRevisionRequestV0.AsObject,
  }

  export class GetIdentityBalanceAndRevisionRequestV0 extends jspb.Message {
    getId(): Uint8Array | string;
    getId_asU8(): Uint8Array;
    getId_asB64(): string;
    setId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityBalanceAndRevisionRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityBalanceAndRevisionRequestV0): GetIdentityBalanceAndRevisionRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityBalanceAndRevisionRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceAndRevisionRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityBalanceAndRevisionRequestV0, reader: jspb.BinaryReader): GetIdentityBalanceAndRevisionRequestV0;
  }

  export namespace GetIdentityBalanceAndRevisionRequestV0 {
    export type AsObject = {
      id: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityResponse.GetIdentityResponseV0 | undefined;
  setV0(value?: GetIdentityResponse.GetIdentityResponseV0): void;

  getVersionCase(): GetIdentityResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityResponse): GetIdentityResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityResponse;
  static deserializeBinaryFromReader(message: GetIdentityResponse, reader: jspb.BinaryReader): GetIdentityResponse;
}

export namespace GetIdentityResponse {
  export type AsObject = {
    v0?: GetIdentityResponse.GetIdentityResponseV0.AsObject,
  }

  export class GetIdentityResponseV0 extends jspb.Message {
    hasIdentity(): boolean;
    clearIdentity(): void;
    getIdentity(): Uint8Array | string;
    getIdentity_asU8(): Uint8Array;
    getIdentity_asB64(): string;
    setIdentity(value: Uint8Array | string): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityResponseV0): GetIdentityResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityResponseV0, reader: jspb.BinaryReader): GetIdentityResponseV0;
  }

  export namespace GetIdentityResponseV0 {
    export type AsObject = {
      identity: Uint8Array | string,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITY = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityNonceResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityNonceResponse.GetIdentityNonceResponseV0 | undefined;
  setV0(value?: GetIdentityNonceResponse.GetIdentityNonceResponseV0): void;

  getVersionCase(): GetIdentityNonceResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityNonceResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityNonceResponse): GetIdentityNonceResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityNonceResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityNonceResponse;
  static deserializeBinaryFromReader(message: GetIdentityNonceResponse, reader: jspb.BinaryReader): GetIdentityNonceResponse;
}

export namespace GetIdentityNonceResponse {
  export type AsObject = {
    v0?: GetIdentityNonceResponse.GetIdentityNonceResponseV0.AsObject,
  }

  export class GetIdentityNonceResponseV0 extends jspb.Message {
    hasIdentityNonce(): boolean;
    clearIdentityNonce(): void;
    getIdentityNonce(): number;
    setIdentityNonce(value: number): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityNonceResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityNonceResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityNonceResponseV0): GetIdentityNonceResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityNonceResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityNonceResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityNonceResponseV0, reader: jspb.BinaryReader): GetIdentityNonceResponseV0;
  }

  export namespace GetIdentityNonceResponseV0 {
    export type AsObject = {
      identityNonce: number,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITY_NONCE = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityContractNonceResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityContractNonceResponse.GetIdentityContractNonceResponseV0 | undefined;
  setV0(value?: GetIdentityContractNonceResponse.GetIdentityContractNonceResponseV0): void;

  getVersionCase(): GetIdentityContractNonceResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityContractNonceResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityContractNonceResponse): GetIdentityContractNonceResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityContractNonceResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityContractNonceResponse;
  static deserializeBinaryFromReader(message: GetIdentityContractNonceResponse, reader: jspb.BinaryReader): GetIdentityContractNonceResponse;
}

export namespace GetIdentityContractNonceResponse {
  export type AsObject = {
    v0?: GetIdentityContractNonceResponse.GetIdentityContractNonceResponseV0.AsObject,
  }

  export class GetIdentityContractNonceResponseV0 extends jspb.Message {
    hasIdentityContractNonce(): boolean;
    clearIdentityContractNonce(): void;
    getIdentityContractNonce(): number;
    setIdentityContractNonce(value: number): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityContractNonceResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityContractNonceResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityContractNonceResponseV0): GetIdentityContractNonceResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityContractNonceResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityContractNonceResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityContractNonceResponseV0, reader: jspb.BinaryReader): GetIdentityContractNonceResponseV0;
  }

  export namespace GetIdentityContractNonceResponseV0 {
    export type AsObject = {
      identityContractNonce: number,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITY_CONTRACT_NONCE = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityBalanceResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityBalanceResponse.GetIdentityBalanceResponseV0 | undefined;
  setV0(value?: GetIdentityBalanceResponse.GetIdentityBalanceResponseV0): void;

  getVersionCase(): GetIdentityBalanceResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityBalanceResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityBalanceResponse): GetIdentityBalanceResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityBalanceResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceResponse;
  static deserializeBinaryFromReader(message: GetIdentityBalanceResponse, reader: jspb.BinaryReader): GetIdentityBalanceResponse;
}

export namespace GetIdentityBalanceResponse {
  export type AsObject = {
    v0?: GetIdentityBalanceResponse.GetIdentityBalanceResponseV0.AsObject,
  }

  export class GetIdentityBalanceResponseV0 extends jspb.Message {
    hasBalance(): boolean;
    clearBalance(): void;
    getBalance(): number;
    setBalance(value: number): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityBalanceResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityBalanceResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityBalanceResponseV0): GetIdentityBalanceResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityBalanceResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityBalanceResponseV0, reader: jspb.BinaryReader): GetIdentityBalanceResponseV0;
  }

  export namespace GetIdentityBalanceResponseV0 {
    export type AsObject = {
      balance: number,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      BALANCE = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityBalanceAndRevisionResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0 | undefined;
  setV0(value?: GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0): void;

  getVersionCase(): GetIdentityBalanceAndRevisionResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityBalanceAndRevisionResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityBalanceAndRevisionResponse): GetIdentityBalanceAndRevisionResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityBalanceAndRevisionResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceAndRevisionResponse;
  static deserializeBinaryFromReader(message: GetIdentityBalanceAndRevisionResponse, reader: jspb.BinaryReader): GetIdentityBalanceAndRevisionResponse;
}

export namespace GetIdentityBalanceAndRevisionResponse {
  export type AsObject = {
    v0?: GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.AsObject,
  }

  export class GetIdentityBalanceAndRevisionResponseV0 extends jspb.Message {
    hasBalanceAndRevision(): boolean;
    clearBalanceAndRevision(): void;
    getBalanceAndRevision(): GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision | undefined;
    setBalanceAndRevision(value?: GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityBalanceAndRevisionResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityBalanceAndRevisionResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityBalanceAndRevisionResponseV0): GetIdentityBalanceAndRevisionResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityBalanceAndRevisionResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityBalanceAndRevisionResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityBalanceAndRevisionResponseV0, reader: jspb.BinaryReader): GetIdentityBalanceAndRevisionResponseV0;
  }

  export namespace GetIdentityBalanceAndRevisionResponseV0 {
    export type AsObject = {
      balanceAndRevision?: GetIdentityBalanceAndRevisionResponse.GetIdentityBalanceAndRevisionResponseV0.BalanceAndRevision.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class BalanceAndRevision extends jspb.Message {
      getBalance(): number;
      setBalance(value: number): void;

      getRevision(): number;
      setRevision(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): BalanceAndRevision.AsObject;
      static toObject(includeInstance: boolean, msg: BalanceAndRevision): BalanceAndRevision.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: BalanceAndRevision, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): BalanceAndRevision;
      static deserializeBinaryFromReader(message: BalanceAndRevision, reader: jspb.BinaryReader): BalanceAndRevision;
    }

    export namespace BalanceAndRevision {
      export type AsObject = {
        balance: number,
        revision: number,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      BALANCE_AND_REVISION = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class KeyRequestType extends jspb.Message {
  hasAllKeys(): boolean;
  clearAllKeys(): void;
  getAllKeys(): AllKeys | undefined;
  setAllKeys(value?: AllKeys): void;

  hasSpecificKeys(): boolean;
  clearSpecificKeys(): void;
  getSpecificKeys(): SpecificKeys | undefined;
  setSpecificKeys(value?: SpecificKeys): void;

  hasSearchKey(): boolean;
  clearSearchKey(): void;
  getSearchKey(): SearchKey | undefined;
  setSearchKey(value?: SearchKey): void;

  getRequestCase(): KeyRequestType.RequestCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): KeyRequestType.AsObject;
  static toObject(includeInstance: boolean, msg: KeyRequestType): KeyRequestType.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: KeyRequestType, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): KeyRequestType;
  static deserializeBinaryFromReader(message: KeyRequestType, reader: jspb.BinaryReader): KeyRequestType;
}

export namespace KeyRequestType {
  export type AsObject = {
    allKeys?: AllKeys.AsObject,
    specificKeys?: SpecificKeys.AsObject,
    searchKey?: SearchKey.AsObject,
  }

  export enum RequestCase {
    REQUEST_NOT_SET = 0,
    ALL_KEYS = 1,
    SPECIFIC_KEYS = 2,
    SEARCH_KEY = 3,
  }
}

export class AllKeys extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): AllKeys.AsObject;
  static toObject(includeInstance: boolean, msg: AllKeys): AllKeys.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: AllKeys, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): AllKeys;
  static deserializeBinaryFromReader(message: AllKeys, reader: jspb.BinaryReader): AllKeys;
}

export namespace AllKeys {
  export type AsObject = {
  }
}

export class SpecificKeys extends jspb.Message {
  clearKeyIdsList(): void;
  getKeyIdsList(): Array<number>;
  setKeyIdsList(value: Array<number>): void;
  addKeyIds(value: number, index?: number): number;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): SpecificKeys.AsObject;
  static toObject(includeInstance: boolean, msg: SpecificKeys): SpecificKeys.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: SpecificKeys, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): SpecificKeys;
  static deserializeBinaryFromReader(message: SpecificKeys, reader: jspb.BinaryReader): SpecificKeys;
}

export namespace SpecificKeys {
  export type AsObject = {
    keyIdsList: Array<number>,
  }
}

export class SearchKey extends jspb.Message {
  getPurposeMapMap(): jspb.Map<number, SecurityLevelMap>;
  clearPurposeMapMap(): void;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): SearchKey.AsObject;
  static toObject(includeInstance: boolean, msg: SearchKey): SearchKey.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: SearchKey, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): SearchKey;
  static deserializeBinaryFromReader(message: SearchKey, reader: jspb.BinaryReader): SearchKey;
}

export namespace SearchKey {
  export type AsObject = {
    purposeMapMap: Array<[number, SecurityLevelMap.AsObject]>,
  }
}

export class SecurityLevelMap extends jspb.Message {
  getSecurityLevelMapMap(): jspb.Map<number, SecurityLevelMap.KeyKindRequestTypeMap[keyof SecurityLevelMap.KeyKindRequestTypeMap]>;
  clearSecurityLevelMapMap(): void;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): SecurityLevelMap.AsObject;
  static toObject(includeInstance: boolean, msg: SecurityLevelMap): SecurityLevelMap.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: SecurityLevelMap, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): SecurityLevelMap;
  static deserializeBinaryFromReader(message: SecurityLevelMap, reader: jspb.BinaryReader): SecurityLevelMap;
}

export namespace SecurityLevelMap {
  export type AsObject = {
    securityLevelMapMap: Array<[number, SecurityLevelMap.KeyKindRequestTypeMap[keyof SecurityLevelMap.KeyKindRequestTypeMap]]>,
  }

  export interface KeyKindRequestTypeMap {
    CURRENT_KEY_OF_KIND_REQUEST: 0;
    ALL_KEYS_OF_KIND_REQUEST: 1;
  }

  export const KeyKindRequestType: KeyKindRequestTypeMap;
}

export class GetIdentityKeysRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityKeysRequest.GetIdentityKeysRequestV0 | undefined;
  setV0(value?: GetIdentityKeysRequest.GetIdentityKeysRequestV0): void;

  getVersionCase(): GetIdentityKeysRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityKeysRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityKeysRequest): GetIdentityKeysRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityKeysRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityKeysRequest;
  static deserializeBinaryFromReader(message: GetIdentityKeysRequest, reader: jspb.BinaryReader): GetIdentityKeysRequest;
}

export namespace GetIdentityKeysRequest {
  export type AsObject = {
    v0?: GetIdentityKeysRequest.GetIdentityKeysRequestV0.AsObject,
  }

  export class GetIdentityKeysRequestV0 extends jspb.Message {
    getIdentityId(): Uint8Array | string;
    getIdentityId_asU8(): Uint8Array;
    getIdentityId_asB64(): string;
    setIdentityId(value: Uint8Array | string): void;

    hasRequestType(): boolean;
    clearRequestType(): void;
    getRequestType(): KeyRequestType | undefined;
    setRequestType(value?: KeyRequestType): void;

    hasLimit(): boolean;
    clearLimit(): void;
    getLimit(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setLimit(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    hasOffset(): boolean;
    clearOffset(): void;
    getOffset(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setOffset(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityKeysRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityKeysRequestV0): GetIdentityKeysRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityKeysRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityKeysRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityKeysRequestV0, reader: jspb.BinaryReader): GetIdentityKeysRequestV0;
  }

  export namespace GetIdentityKeysRequestV0 {
    export type AsObject = {
      identityId: Uint8Array | string,
      requestType?: KeyRequestType.AsObject,
      limit?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      offset?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityKeysResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityKeysResponse.GetIdentityKeysResponseV0 | undefined;
  setV0(value?: GetIdentityKeysResponse.GetIdentityKeysResponseV0): void;

  getVersionCase(): GetIdentityKeysResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityKeysResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityKeysResponse): GetIdentityKeysResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityKeysResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityKeysResponse;
  static deserializeBinaryFromReader(message: GetIdentityKeysResponse, reader: jspb.BinaryReader): GetIdentityKeysResponse;
}

export namespace GetIdentityKeysResponse {
  export type AsObject = {
    v0?: GetIdentityKeysResponse.GetIdentityKeysResponseV0.AsObject,
  }

  export class GetIdentityKeysResponseV0 extends jspb.Message {
    hasKeys(): boolean;
    clearKeys(): void;
    getKeys(): GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys | undefined;
    setKeys(value?: GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityKeysResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityKeysResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityKeysResponseV0): GetIdentityKeysResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityKeysResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityKeysResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityKeysResponseV0, reader: jspb.BinaryReader): GetIdentityKeysResponseV0;
  }

  export namespace GetIdentityKeysResponseV0 {
    export type AsObject = {
      keys?: GetIdentityKeysResponse.GetIdentityKeysResponseV0.Keys.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class Keys extends jspb.Message {
      clearKeysBytesList(): void;
      getKeysBytesList(): Array<Uint8Array | string>;
      getKeysBytesList_asU8(): Array<Uint8Array>;
      getKeysBytesList_asB64(): Array<string>;
      setKeysBytesList(value: Array<Uint8Array | string>): void;
      addKeysBytes(value: Uint8Array | string, index?: number): Uint8Array | string;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Keys.AsObject;
      static toObject(includeInstance: boolean, msg: Keys): Keys.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Keys, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Keys;
      static deserializeBinaryFromReader(message: Keys, reader: jspb.BinaryReader): Keys;
    }

    export namespace Keys {
      export type AsObject = {
        keysBytesList: Array<Uint8Array | string>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      KEYS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentitiesContractKeysRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesContractKeysRequest.GetIdentitiesContractKeysRequestV0 | undefined;
  setV0(value?: GetIdentitiesContractKeysRequest.GetIdentitiesContractKeysRequestV0): void;

  getVersionCase(): GetIdentitiesContractKeysRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesContractKeysRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesContractKeysRequest): GetIdentitiesContractKeysRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesContractKeysRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesContractKeysRequest;
  static deserializeBinaryFromReader(message: GetIdentitiesContractKeysRequest, reader: jspb.BinaryReader): GetIdentitiesContractKeysRequest;
}

export namespace GetIdentitiesContractKeysRequest {
  export type AsObject = {
    v0?: GetIdentitiesContractKeysRequest.GetIdentitiesContractKeysRequestV0.AsObject,
  }

  export class GetIdentitiesContractKeysRequestV0 extends jspb.Message {
    clearIdentitiesIdsList(): void;
    getIdentitiesIdsList(): Array<Uint8Array | string>;
    getIdentitiesIdsList_asU8(): Array<Uint8Array>;
    getIdentitiesIdsList_asB64(): Array<string>;
    setIdentitiesIdsList(value: Array<Uint8Array | string>): void;
    addIdentitiesIds(value: Uint8Array | string, index?: number): Uint8Array | string;

    getContractId(): Uint8Array | string;
    getContractId_asU8(): Uint8Array;
    getContractId_asB64(): string;
    setContractId(value: Uint8Array | string): void;

    hasDocumentTypeName(): boolean;
    clearDocumentTypeName(): void;
    getDocumentTypeName(): string;
    setDocumentTypeName(value: string): void;

    clearPurposesList(): void;
    getPurposesList(): Array<KeyPurposeMap[keyof KeyPurposeMap]>;
    setPurposesList(value: Array<KeyPurposeMap[keyof KeyPurposeMap]>): void;
    addPurposes(value: KeyPurposeMap[keyof KeyPurposeMap], index?: number): KeyPurposeMap[keyof KeyPurposeMap];

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesContractKeysRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesContractKeysRequestV0): GetIdentitiesContractKeysRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesContractKeysRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesContractKeysRequestV0;
    static deserializeBinaryFromReader(message: GetIdentitiesContractKeysRequestV0, reader: jspb.BinaryReader): GetIdentitiesContractKeysRequestV0;
  }

  export namespace GetIdentitiesContractKeysRequestV0 {
    export type AsObject = {
      identitiesIdsList: Array<Uint8Array | string>,
      contractId: Uint8Array | string,
      documentTypeName: string,
      purposesList: Array<KeyPurposeMap[keyof KeyPurposeMap]>,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentitiesContractKeysResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0 | undefined;
  setV0(value?: GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0): void;

  getVersionCase(): GetIdentitiesContractKeysResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesContractKeysResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesContractKeysResponse): GetIdentitiesContractKeysResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesContractKeysResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesContractKeysResponse;
  static deserializeBinaryFromReader(message: GetIdentitiesContractKeysResponse, reader: jspb.BinaryReader): GetIdentitiesContractKeysResponse;
}

export namespace GetIdentitiesContractKeysResponse {
  export type AsObject = {
    v0?: GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.AsObject,
  }

  export class GetIdentitiesContractKeysResponseV0 extends jspb.Message {
    hasIdentitiesKeys(): boolean;
    clearIdentitiesKeys(): void;
    getIdentitiesKeys(): GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentitiesKeys | undefined;
    setIdentitiesKeys(value?: GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentitiesKeys): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentitiesContractKeysResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesContractKeysResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesContractKeysResponseV0): GetIdentitiesContractKeysResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesContractKeysResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesContractKeysResponseV0;
    static deserializeBinaryFromReader(message: GetIdentitiesContractKeysResponseV0, reader: jspb.BinaryReader): GetIdentitiesContractKeysResponseV0;
  }

  export namespace GetIdentitiesContractKeysResponseV0 {
    export type AsObject = {
      identitiesKeys?: GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentitiesKeys.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class PurposeKeys extends jspb.Message {
      getPurpose(): KeyPurposeMap[keyof KeyPurposeMap];
      setPurpose(value: KeyPurposeMap[keyof KeyPurposeMap]): void;

      clearKeysBytesList(): void;
      getKeysBytesList(): Array<Uint8Array | string>;
      getKeysBytesList_asU8(): Array<Uint8Array>;
      getKeysBytesList_asB64(): Array<string>;
      setKeysBytesList(value: Array<Uint8Array | string>): void;
      addKeysBytes(value: Uint8Array | string, index?: number): Uint8Array | string;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): PurposeKeys.AsObject;
      static toObject(includeInstance: boolean, msg: PurposeKeys): PurposeKeys.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: PurposeKeys, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): PurposeKeys;
      static deserializeBinaryFromReader(message: PurposeKeys, reader: jspb.BinaryReader): PurposeKeys;
    }

    export namespace PurposeKeys {
      export type AsObject = {
        purpose: KeyPurposeMap[keyof KeyPurposeMap],
        keysBytesList: Array<Uint8Array | string>,
      }
    }

    export class IdentityKeys extends jspb.Message {
      getIdentityId(): Uint8Array | string;
      getIdentityId_asU8(): Uint8Array;
      getIdentityId_asB64(): string;
      setIdentityId(value: Uint8Array | string): void;

      clearKeysList(): void;
      getKeysList(): Array<GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.PurposeKeys>;
      setKeysList(value: Array<GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.PurposeKeys>): void;
      addKeys(value?: GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.PurposeKeys, index?: number): GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.PurposeKeys;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): IdentityKeys.AsObject;
      static toObject(includeInstance: boolean, msg: IdentityKeys): IdentityKeys.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: IdentityKeys, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): IdentityKeys;
      static deserializeBinaryFromReader(message: IdentityKeys, reader: jspb.BinaryReader): IdentityKeys;
    }

    export namespace IdentityKeys {
      export type AsObject = {
        identityId: Uint8Array | string,
        keysList: Array<GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.PurposeKeys.AsObject>,
      }
    }

    export class IdentitiesKeys extends jspb.Message {
      clearEntriesList(): void;
      getEntriesList(): Array<GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentityKeys>;
      setEntriesList(value: Array<GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentityKeys>): void;
      addEntries(value?: GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentityKeys, index?: number): GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentityKeys;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): IdentitiesKeys.AsObject;
      static toObject(includeInstance: boolean, msg: IdentitiesKeys): IdentitiesKeys.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: IdentitiesKeys, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): IdentitiesKeys;
      static deserializeBinaryFromReader(message: IdentitiesKeys, reader: jspb.BinaryReader): IdentitiesKeys;
    }

    export namespace IdentitiesKeys {
      export type AsObject = {
        entriesList: Array<GetIdentitiesContractKeysResponse.GetIdentitiesContractKeysResponseV0.IdentityKeys.AsObject>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITIES_KEYS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetEvonodesProposedEpochBlocksByIdsRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetEvonodesProposedEpochBlocksByIdsRequest.GetEvonodesProposedEpochBlocksByIdsRequestV0 | undefined;
  setV0(value?: GetEvonodesProposedEpochBlocksByIdsRequest.GetEvonodesProposedEpochBlocksByIdsRequestV0): void;

  getVersionCase(): GetEvonodesProposedEpochBlocksByIdsRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEvonodesProposedEpochBlocksByIdsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetEvonodesProposedEpochBlocksByIdsRequest): GetEvonodesProposedEpochBlocksByIdsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEvonodesProposedEpochBlocksByIdsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEvonodesProposedEpochBlocksByIdsRequest;
  static deserializeBinaryFromReader(message: GetEvonodesProposedEpochBlocksByIdsRequest, reader: jspb.BinaryReader): GetEvonodesProposedEpochBlocksByIdsRequest;
}

export namespace GetEvonodesProposedEpochBlocksByIdsRequest {
  export type AsObject = {
    v0?: GetEvonodesProposedEpochBlocksByIdsRequest.GetEvonodesProposedEpochBlocksByIdsRequestV0.AsObject,
  }

  export class GetEvonodesProposedEpochBlocksByIdsRequestV0 extends jspb.Message {
    hasEpoch(): boolean;
    clearEpoch(): void;
    getEpoch(): number;
    setEpoch(value: number): void;

    clearIdsList(): void;
    getIdsList(): Array<Uint8Array | string>;
    getIdsList_asU8(): Array<Uint8Array>;
    getIdsList_asB64(): Array<string>;
    setIdsList(value: Array<Uint8Array | string>): void;
    addIds(value: Uint8Array | string, index?: number): Uint8Array | string;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetEvonodesProposedEpochBlocksByIdsRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetEvonodesProposedEpochBlocksByIdsRequestV0): GetEvonodesProposedEpochBlocksByIdsRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetEvonodesProposedEpochBlocksByIdsRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetEvonodesProposedEpochBlocksByIdsRequestV0;
    static deserializeBinaryFromReader(message: GetEvonodesProposedEpochBlocksByIdsRequestV0, reader: jspb.BinaryReader): GetEvonodesProposedEpochBlocksByIdsRequestV0;
  }

  export namespace GetEvonodesProposedEpochBlocksByIdsRequestV0 {
    export type AsObject = {
      epoch: number,
      idsList: Array<Uint8Array | string>,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetEvonodesProposedEpochBlocksResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0 | undefined;
  setV0(value?: GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0): void;

  getVersionCase(): GetEvonodesProposedEpochBlocksResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEvonodesProposedEpochBlocksResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetEvonodesProposedEpochBlocksResponse): GetEvonodesProposedEpochBlocksResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEvonodesProposedEpochBlocksResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEvonodesProposedEpochBlocksResponse;
  static deserializeBinaryFromReader(message: GetEvonodesProposedEpochBlocksResponse, reader: jspb.BinaryReader): GetEvonodesProposedEpochBlocksResponse;
}

export namespace GetEvonodesProposedEpochBlocksResponse {
  export type AsObject = {
    v0?: GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.AsObject,
  }

  export class GetEvonodesProposedEpochBlocksResponseV0 extends jspb.Message {
    hasEvonodesProposedBlockCountsInfo(): boolean;
    clearEvonodesProposedBlockCountsInfo(): void;
    getEvonodesProposedBlockCountsInfo(): GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodesProposedBlocks | undefined;
    setEvonodesProposedBlockCountsInfo(value?: GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodesProposedBlocks): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetEvonodesProposedEpochBlocksResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetEvonodesProposedEpochBlocksResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetEvonodesProposedEpochBlocksResponseV0): GetEvonodesProposedEpochBlocksResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetEvonodesProposedEpochBlocksResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetEvonodesProposedEpochBlocksResponseV0;
    static deserializeBinaryFromReader(message: GetEvonodesProposedEpochBlocksResponseV0, reader: jspb.BinaryReader): GetEvonodesProposedEpochBlocksResponseV0;
  }

  export namespace GetEvonodesProposedEpochBlocksResponseV0 {
    export type AsObject = {
      evonodesProposedBlockCountsInfo?: GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodesProposedBlocks.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class EvonodeProposedBlocks extends jspb.Message {
      getProTxHash(): Uint8Array | string;
      getProTxHash_asU8(): Uint8Array;
      getProTxHash_asB64(): string;
      setProTxHash(value: Uint8Array | string): void;

      getCount(): number;
      setCount(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): EvonodeProposedBlocks.AsObject;
      static toObject(includeInstance: boolean, msg: EvonodeProposedBlocks): EvonodeProposedBlocks.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: EvonodeProposedBlocks, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): EvonodeProposedBlocks;
      static deserializeBinaryFromReader(message: EvonodeProposedBlocks, reader: jspb.BinaryReader): EvonodeProposedBlocks;
    }

    export namespace EvonodeProposedBlocks {
      export type AsObject = {
        proTxHash: Uint8Array | string,
        count: number,
      }
    }

    export class EvonodesProposedBlocks extends jspb.Message {
      clearEvonodesProposedBlockCountsList(): void;
      getEvonodesProposedBlockCountsList(): Array<GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodeProposedBlocks>;
      setEvonodesProposedBlockCountsList(value: Array<GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodeProposedBlocks>): void;
      addEvonodesProposedBlockCounts(value?: GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodeProposedBlocks, index?: number): GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodeProposedBlocks;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): EvonodesProposedBlocks.AsObject;
      static toObject(includeInstance: boolean, msg: EvonodesProposedBlocks): EvonodesProposedBlocks.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: EvonodesProposedBlocks, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): EvonodesProposedBlocks;
      static deserializeBinaryFromReader(message: EvonodesProposedBlocks, reader: jspb.BinaryReader): EvonodesProposedBlocks;
    }

    export namespace EvonodesProposedBlocks {
      export type AsObject = {
        evonodesProposedBlockCountsList: Array<GetEvonodesProposedEpochBlocksResponse.GetEvonodesProposedEpochBlocksResponseV0.EvonodeProposedBlocks.AsObject>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      EVONODES_PROPOSED_BLOCK_COUNTS_INFO = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetEvonodesProposedEpochBlocksByRangeRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetEvonodesProposedEpochBlocksByRangeRequest.GetEvonodesProposedEpochBlocksByRangeRequestV0 | undefined;
  setV0(value?: GetEvonodesProposedEpochBlocksByRangeRequest.GetEvonodesProposedEpochBlocksByRangeRequestV0): void;

  getVersionCase(): GetEvonodesProposedEpochBlocksByRangeRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEvonodesProposedEpochBlocksByRangeRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetEvonodesProposedEpochBlocksByRangeRequest): GetEvonodesProposedEpochBlocksByRangeRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEvonodesProposedEpochBlocksByRangeRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEvonodesProposedEpochBlocksByRangeRequest;
  static deserializeBinaryFromReader(message: GetEvonodesProposedEpochBlocksByRangeRequest, reader: jspb.BinaryReader): GetEvonodesProposedEpochBlocksByRangeRequest;
}

export namespace GetEvonodesProposedEpochBlocksByRangeRequest {
  export type AsObject = {
    v0?: GetEvonodesProposedEpochBlocksByRangeRequest.GetEvonodesProposedEpochBlocksByRangeRequestV0.AsObject,
  }

  export class GetEvonodesProposedEpochBlocksByRangeRequestV0 extends jspb.Message {
    hasEpoch(): boolean;
    clearEpoch(): void;
    getEpoch(): number;
    setEpoch(value: number): void;

    hasLimit(): boolean;
    clearLimit(): void;
    getLimit(): number;
    setLimit(value: number): void;

    hasStartAfter(): boolean;
    clearStartAfter(): void;
    getStartAfter(): Uint8Array | string;
    getStartAfter_asU8(): Uint8Array;
    getStartAfter_asB64(): string;
    setStartAfter(value: Uint8Array | string): void;

    hasStartAt(): boolean;
    clearStartAt(): void;
    getStartAt(): Uint8Array | string;
    getStartAt_asU8(): Uint8Array;
    getStartAt_asB64(): string;
    setStartAt(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    getStartCase(): GetEvonodesProposedEpochBlocksByRangeRequestV0.StartCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetEvonodesProposedEpochBlocksByRangeRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetEvonodesProposedEpochBlocksByRangeRequestV0): GetEvonodesProposedEpochBlocksByRangeRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetEvonodesProposedEpochBlocksByRangeRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetEvonodesProposedEpochBlocksByRangeRequestV0;
    static deserializeBinaryFromReader(message: GetEvonodesProposedEpochBlocksByRangeRequestV0, reader: jspb.BinaryReader): GetEvonodesProposedEpochBlocksByRangeRequestV0;
  }

  export namespace GetEvonodesProposedEpochBlocksByRangeRequestV0 {
    export type AsObject = {
      epoch: number,
      limit: number,
      startAfter: Uint8Array | string,
      startAt: Uint8Array | string,
      prove: boolean,
    }

    export enum StartCase {
      START_NOT_SET = 0,
      START_AFTER = 3,
      START_AT = 4,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentitiesBalancesRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesBalancesRequest.GetIdentitiesBalancesRequestV0 | undefined;
  setV0(value?: GetIdentitiesBalancesRequest.GetIdentitiesBalancesRequestV0): void;

  getVersionCase(): GetIdentitiesBalancesRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesBalancesRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesBalancesRequest): GetIdentitiesBalancesRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesBalancesRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesBalancesRequest;
  static deserializeBinaryFromReader(message: GetIdentitiesBalancesRequest, reader: jspb.BinaryReader): GetIdentitiesBalancesRequest;
}

export namespace GetIdentitiesBalancesRequest {
  export type AsObject = {
    v0?: GetIdentitiesBalancesRequest.GetIdentitiesBalancesRequestV0.AsObject,
  }

  export class GetIdentitiesBalancesRequestV0 extends jspb.Message {
    clearIdsList(): void;
    getIdsList(): Array<Uint8Array | string>;
    getIdsList_asU8(): Array<Uint8Array>;
    getIdsList_asB64(): Array<string>;
    setIdsList(value: Array<Uint8Array | string>): void;
    addIds(value: Uint8Array | string, index?: number): Uint8Array | string;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesBalancesRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesBalancesRequestV0): GetIdentitiesBalancesRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesBalancesRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesBalancesRequestV0;
    static deserializeBinaryFromReader(message: GetIdentitiesBalancesRequestV0, reader: jspb.BinaryReader): GetIdentitiesBalancesRequestV0;
  }

  export namespace GetIdentitiesBalancesRequestV0 {
    export type AsObject = {
      idsList: Array<Uint8Array | string>,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentitiesBalancesResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0 | undefined;
  setV0(value?: GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0): void;

  getVersionCase(): GetIdentitiesBalancesResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesBalancesResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesBalancesResponse): GetIdentitiesBalancesResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesBalancesResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesBalancesResponse;
  static deserializeBinaryFromReader(message: GetIdentitiesBalancesResponse, reader: jspb.BinaryReader): GetIdentitiesBalancesResponse;
}

export namespace GetIdentitiesBalancesResponse {
  export type AsObject = {
    v0?: GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.AsObject,
  }

  export class GetIdentitiesBalancesResponseV0 extends jspb.Message {
    hasIdentitiesBalances(): boolean;
    clearIdentitiesBalances(): void;
    getIdentitiesBalances(): GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentitiesBalances | undefined;
    setIdentitiesBalances(value?: GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentitiesBalances): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentitiesBalancesResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesBalancesResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesBalancesResponseV0): GetIdentitiesBalancesResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesBalancesResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesBalancesResponseV0;
    static deserializeBinaryFromReader(message: GetIdentitiesBalancesResponseV0, reader: jspb.BinaryReader): GetIdentitiesBalancesResponseV0;
  }

  export namespace GetIdentitiesBalancesResponseV0 {
    export type AsObject = {
      identitiesBalances?: GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentitiesBalances.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class IdentityBalance extends jspb.Message {
      getIdentityId(): Uint8Array | string;
      getIdentityId_asU8(): Uint8Array;
      getIdentityId_asB64(): string;
      setIdentityId(value: Uint8Array | string): void;

      hasBalance(): boolean;
      clearBalance(): void;
      getBalance(): number;
      setBalance(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): IdentityBalance.AsObject;
      static toObject(includeInstance: boolean, msg: IdentityBalance): IdentityBalance.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: IdentityBalance, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): IdentityBalance;
      static deserializeBinaryFromReader(message: IdentityBalance, reader: jspb.BinaryReader): IdentityBalance;
    }

    export namespace IdentityBalance {
      export type AsObject = {
        identityId: Uint8Array | string,
        balance: number,
      }
    }

    export class IdentitiesBalances extends jspb.Message {
      clearEntriesList(): void;
      getEntriesList(): Array<GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentityBalance>;
      setEntriesList(value: Array<GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentityBalance>): void;
      addEntries(value?: GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentityBalance, index?: number): GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentityBalance;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): IdentitiesBalances.AsObject;
      static toObject(includeInstance: boolean, msg: IdentitiesBalances): IdentitiesBalances.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: IdentitiesBalances, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): IdentitiesBalances;
      static deserializeBinaryFromReader(message: IdentitiesBalances, reader: jspb.BinaryReader): IdentitiesBalances;
    }

    export namespace IdentitiesBalances {
      export type AsObject = {
        entriesList: Array<GetIdentitiesBalancesResponse.GetIdentitiesBalancesResponseV0.IdentityBalance.AsObject>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITIES_BALANCES = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetProofsRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetProofsRequest.GetProofsRequestV0 | undefined;
  setV0(value?: GetProofsRequest.GetProofsRequestV0): void;

  getVersionCase(): GetProofsRequest.VersionCase;
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
    v0?: GetProofsRequest.GetProofsRequestV0.AsObject,
  }

  export class GetProofsRequestV0 extends jspb.Message {
    clearIdentitiesList(): void;
    getIdentitiesList(): Array<GetProofsRequest.GetProofsRequestV0.IdentityRequest>;
    setIdentitiesList(value: Array<GetProofsRequest.GetProofsRequestV0.IdentityRequest>): void;
    addIdentities(value?: GetProofsRequest.GetProofsRequestV0.IdentityRequest, index?: number): GetProofsRequest.GetProofsRequestV0.IdentityRequest;

    clearContractsList(): void;
    getContractsList(): Array<GetProofsRequest.GetProofsRequestV0.ContractRequest>;
    setContractsList(value: Array<GetProofsRequest.GetProofsRequestV0.ContractRequest>): void;
    addContracts(value?: GetProofsRequest.GetProofsRequestV0.ContractRequest, index?: number): GetProofsRequest.GetProofsRequestV0.ContractRequest;

    clearDocumentsList(): void;
    getDocumentsList(): Array<GetProofsRequest.GetProofsRequestV0.DocumentRequest>;
    setDocumentsList(value: Array<GetProofsRequest.GetProofsRequestV0.DocumentRequest>): void;
    addDocuments(value?: GetProofsRequest.GetProofsRequestV0.DocumentRequest, index?: number): GetProofsRequest.GetProofsRequestV0.DocumentRequest;

    clearVotesList(): void;
    getVotesList(): Array<GetProofsRequest.GetProofsRequestV0.VoteStatusRequest>;
    setVotesList(value: Array<GetProofsRequest.GetProofsRequestV0.VoteStatusRequest>): void;
    addVotes(value?: GetProofsRequest.GetProofsRequestV0.VoteStatusRequest, index?: number): GetProofsRequest.GetProofsRequestV0.VoteStatusRequest;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetProofsRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetProofsRequestV0): GetProofsRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetProofsRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetProofsRequestV0;
    static deserializeBinaryFromReader(message: GetProofsRequestV0, reader: jspb.BinaryReader): GetProofsRequestV0;
  }

  export namespace GetProofsRequestV0 {
    export type AsObject = {
      identitiesList: Array<GetProofsRequest.GetProofsRequestV0.IdentityRequest.AsObject>,
      contractsList: Array<GetProofsRequest.GetProofsRequestV0.ContractRequest.AsObject>,
      documentsList: Array<GetProofsRequest.GetProofsRequestV0.DocumentRequest.AsObject>,
      votesList: Array<GetProofsRequest.GetProofsRequestV0.VoteStatusRequest.AsObject>,
    }

    export class DocumentRequest extends jspb.Message {
      getContractId(): Uint8Array | string;
      getContractId_asU8(): Uint8Array;
      getContractId_asB64(): string;
      setContractId(value: Uint8Array | string): void;

      getDocumentType(): string;
      setDocumentType(value: string): void;

      getDocumentTypeKeepsHistory(): boolean;
      setDocumentTypeKeepsHistory(value: boolean): void;

      getDocumentId(): Uint8Array | string;
      getDocumentId_asU8(): Uint8Array;
      getDocumentId_asB64(): string;
      setDocumentId(value: Uint8Array | string): void;

      getDocumentContestedStatus(): GetProofsRequest.GetProofsRequestV0.DocumentRequest.DocumentContestedStatusMap[keyof GetProofsRequest.GetProofsRequestV0.DocumentRequest.DocumentContestedStatusMap];
      setDocumentContestedStatus(value: GetProofsRequest.GetProofsRequestV0.DocumentRequest.DocumentContestedStatusMap[keyof GetProofsRequest.GetProofsRequestV0.DocumentRequest.DocumentContestedStatusMap]): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): DocumentRequest.AsObject;
      static toObject(includeInstance: boolean, msg: DocumentRequest): DocumentRequest.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: DocumentRequest, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): DocumentRequest;
      static deserializeBinaryFromReader(message: DocumentRequest, reader: jspb.BinaryReader): DocumentRequest;
    }

    export namespace DocumentRequest {
      export type AsObject = {
        contractId: Uint8Array | string,
        documentType: string,
        documentTypeKeepsHistory: boolean,
        documentId: Uint8Array | string,
        documentContestedStatus: GetProofsRequest.GetProofsRequestV0.DocumentRequest.DocumentContestedStatusMap[keyof GetProofsRequest.GetProofsRequestV0.DocumentRequest.DocumentContestedStatusMap],
      }

      export interface DocumentContestedStatusMap {
        NOT_CONTESTED: 0;
        MAYBE_CONTESTED: 1;
        CONTESTED: 2;
      }

      export const DocumentContestedStatus: DocumentContestedStatusMap;
    }

    export class IdentityRequest extends jspb.Message {
      getIdentityId(): Uint8Array | string;
      getIdentityId_asU8(): Uint8Array;
      getIdentityId_asB64(): string;
      setIdentityId(value: Uint8Array | string): void;

      getRequestType(): GetProofsRequest.GetProofsRequestV0.IdentityRequest.TypeMap[keyof GetProofsRequest.GetProofsRequestV0.IdentityRequest.TypeMap];
      setRequestType(value: GetProofsRequest.GetProofsRequestV0.IdentityRequest.TypeMap[keyof GetProofsRequest.GetProofsRequestV0.IdentityRequest.TypeMap]): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): IdentityRequest.AsObject;
      static toObject(includeInstance: boolean, msg: IdentityRequest): IdentityRequest.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: IdentityRequest, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): IdentityRequest;
      static deserializeBinaryFromReader(message: IdentityRequest, reader: jspb.BinaryReader): IdentityRequest;
    }

    export namespace IdentityRequest {
      export type AsObject = {
        identityId: Uint8Array | string,
        requestType: GetProofsRequest.GetProofsRequestV0.IdentityRequest.TypeMap[keyof GetProofsRequest.GetProofsRequestV0.IdentityRequest.TypeMap],
      }

      export interface TypeMap {
        FULL_IDENTITY: 0;
        BALANCE: 1;
        KEYS: 2;
        REVISION: 3;
      }

      export const Type: TypeMap;
    }

    export class ContractRequest extends jspb.Message {
      getContractId(): Uint8Array | string;
      getContractId_asU8(): Uint8Array;
      getContractId_asB64(): string;
      setContractId(value: Uint8Array | string): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ContractRequest.AsObject;
      static toObject(includeInstance: boolean, msg: ContractRequest): ContractRequest.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ContractRequest, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ContractRequest;
      static deserializeBinaryFromReader(message: ContractRequest, reader: jspb.BinaryReader): ContractRequest;
    }

    export namespace ContractRequest {
      export type AsObject = {
        contractId: Uint8Array | string,
      }
    }

    export class VoteStatusRequest extends jspb.Message {
      hasContestedResourceVoteStatusRequest(): boolean;
      clearContestedResourceVoteStatusRequest(): void;
      getContestedResourceVoteStatusRequest(): GetProofsRequest.GetProofsRequestV0.VoteStatusRequest.ContestedResourceVoteStatusRequest | undefined;
      setContestedResourceVoteStatusRequest(value?: GetProofsRequest.GetProofsRequestV0.VoteStatusRequest.ContestedResourceVoteStatusRequest): void;

      getRequestTypeCase(): VoteStatusRequest.RequestTypeCase;
      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): VoteStatusRequest.AsObject;
      static toObject(includeInstance: boolean, msg: VoteStatusRequest): VoteStatusRequest.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: VoteStatusRequest, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): VoteStatusRequest;
      static deserializeBinaryFromReader(message: VoteStatusRequest, reader: jspb.BinaryReader): VoteStatusRequest;
    }

    export namespace VoteStatusRequest {
      export type AsObject = {
        contestedResourceVoteStatusRequest?: GetProofsRequest.GetProofsRequestV0.VoteStatusRequest.ContestedResourceVoteStatusRequest.AsObject,
      }

      export class ContestedResourceVoteStatusRequest extends jspb.Message {
        getContractId(): Uint8Array | string;
        getContractId_asU8(): Uint8Array;
        getContractId_asB64(): string;
        setContractId(value: Uint8Array | string): void;

        getDocumentTypeName(): string;
        setDocumentTypeName(value: string): void;

        getIndexName(): string;
        setIndexName(value: string): void;

        clearIndexValuesList(): void;
        getIndexValuesList(): Array<Uint8Array | string>;
        getIndexValuesList_asU8(): Array<Uint8Array>;
        getIndexValuesList_asB64(): Array<string>;
        setIndexValuesList(value: Array<Uint8Array | string>): void;
        addIndexValues(value: Uint8Array | string, index?: number): Uint8Array | string;

        getVoterIdentifier(): Uint8Array | string;
        getVoterIdentifier_asU8(): Uint8Array;
        getVoterIdentifier_asB64(): string;
        setVoterIdentifier(value: Uint8Array | string): void;

        serializeBinary(): Uint8Array;
        toObject(includeInstance?: boolean): ContestedResourceVoteStatusRequest.AsObject;
        static toObject(includeInstance: boolean, msg: ContestedResourceVoteStatusRequest): ContestedResourceVoteStatusRequest.AsObject;
        static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
        static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
        static serializeBinaryToWriter(message: ContestedResourceVoteStatusRequest, writer: jspb.BinaryWriter): void;
        static deserializeBinary(bytes: Uint8Array): ContestedResourceVoteStatusRequest;
        static deserializeBinaryFromReader(message: ContestedResourceVoteStatusRequest, reader: jspb.BinaryReader): ContestedResourceVoteStatusRequest;
      }

      export namespace ContestedResourceVoteStatusRequest {
        export type AsObject = {
          contractId: Uint8Array | string,
          documentTypeName: string,
          indexName: string,
          indexValuesList: Array<Uint8Array | string>,
          voterIdentifier: Uint8Array | string,
        }
      }

      export enum RequestTypeCase {
        REQUEST_TYPE_NOT_SET = 0,
        CONTESTED_RESOURCE_VOTE_STATUS_REQUEST = 1,
      }
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetProofsResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetProofsResponse.GetProofsResponseV0 | undefined;
  setV0(value?: GetProofsResponse.GetProofsResponseV0): void;

  getVersionCase(): GetProofsResponse.VersionCase;
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
    v0?: GetProofsResponse.GetProofsResponseV0.AsObject,
  }

  export class GetProofsResponseV0 extends jspb.Message {
    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetProofsResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetProofsResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetProofsResponseV0): GetProofsResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetProofsResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetProofsResponseV0;
    static deserializeBinaryFromReader(message: GetProofsResponseV0, reader: jspb.BinaryReader): GetProofsResponseV0;
  }

  export namespace GetProofsResponseV0 {
    export type AsObject = {
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      PROOF = 1,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDataContractRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDataContractRequest.GetDataContractRequestV0 | undefined;
  setV0(value?: GetDataContractRequest.GetDataContractRequestV0): void;

  getVersionCase(): GetDataContractRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDataContractRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetDataContractRequest): GetDataContractRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDataContractRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDataContractRequest;
  static deserializeBinaryFromReader(message: GetDataContractRequest, reader: jspb.BinaryReader): GetDataContractRequest;
}

export namespace GetDataContractRequest {
  export type AsObject = {
    v0?: GetDataContractRequest.GetDataContractRequestV0.AsObject,
  }

  export class GetDataContractRequestV0 extends jspb.Message {
    getId(): Uint8Array | string;
    getId_asU8(): Uint8Array;
    getId_asB64(): string;
    setId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDataContractRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDataContractRequestV0): GetDataContractRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDataContractRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDataContractRequestV0;
    static deserializeBinaryFromReader(message: GetDataContractRequestV0, reader: jspb.BinaryReader): GetDataContractRequestV0;
  }

  export namespace GetDataContractRequestV0 {
    export type AsObject = {
      id: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDataContractResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDataContractResponse.GetDataContractResponseV0 | undefined;
  setV0(value?: GetDataContractResponse.GetDataContractResponseV0): void;

  getVersionCase(): GetDataContractResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDataContractResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetDataContractResponse): GetDataContractResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDataContractResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDataContractResponse;
  static deserializeBinaryFromReader(message: GetDataContractResponse, reader: jspb.BinaryReader): GetDataContractResponse;
}

export namespace GetDataContractResponse {
  export type AsObject = {
    v0?: GetDataContractResponse.GetDataContractResponseV0.AsObject,
  }

  export class GetDataContractResponseV0 extends jspb.Message {
    hasDataContract(): boolean;
    clearDataContract(): void;
    getDataContract(): Uint8Array | string;
    getDataContract_asU8(): Uint8Array;
    getDataContract_asB64(): string;
    setDataContract(value: Uint8Array | string): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetDataContractResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDataContractResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDataContractResponseV0): GetDataContractResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDataContractResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDataContractResponseV0;
    static deserializeBinaryFromReader(message: GetDataContractResponseV0, reader: jspb.BinaryReader): GetDataContractResponseV0;
  }

  export namespace GetDataContractResponseV0 {
    export type AsObject = {
      dataContract: Uint8Array | string,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      DATA_CONTRACT = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDataContractsRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDataContractsRequest.GetDataContractsRequestV0 | undefined;
  setV0(value?: GetDataContractsRequest.GetDataContractsRequestV0): void;

  getVersionCase(): GetDataContractsRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDataContractsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetDataContractsRequest): GetDataContractsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDataContractsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDataContractsRequest;
  static deserializeBinaryFromReader(message: GetDataContractsRequest, reader: jspb.BinaryReader): GetDataContractsRequest;
}

export namespace GetDataContractsRequest {
  export type AsObject = {
    v0?: GetDataContractsRequest.GetDataContractsRequestV0.AsObject,
  }

  export class GetDataContractsRequestV0 extends jspb.Message {
    clearIdsList(): void;
    getIdsList(): Array<Uint8Array | string>;
    getIdsList_asU8(): Array<Uint8Array>;
    getIdsList_asB64(): Array<string>;
    setIdsList(value: Array<Uint8Array | string>): void;
    addIds(value: Uint8Array | string, index?: number): Uint8Array | string;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDataContractsRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDataContractsRequestV0): GetDataContractsRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDataContractsRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDataContractsRequestV0;
    static deserializeBinaryFromReader(message: GetDataContractsRequestV0, reader: jspb.BinaryReader): GetDataContractsRequestV0;
  }

  export namespace GetDataContractsRequestV0 {
    export type AsObject = {
      idsList: Array<Uint8Array | string>,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDataContractsResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDataContractsResponse.GetDataContractsResponseV0 | undefined;
  setV0(value?: GetDataContractsResponse.GetDataContractsResponseV0): void;

  getVersionCase(): GetDataContractsResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDataContractsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetDataContractsResponse): GetDataContractsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDataContractsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDataContractsResponse;
  static deserializeBinaryFromReader(message: GetDataContractsResponse, reader: jspb.BinaryReader): GetDataContractsResponse;
}

export namespace GetDataContractsResponse {
  export type AsObject = {
    v0?: GetDataContractsResponse.GetDataContractsResponseV0.AsObject,
  }

  export class DataContractEntry extends jspb.Message {
    getIdentifier(): Uint8Array | string;
    getIdentifier_asU8(): Uint8Array;
    getIdentifier_asB64(): string;
    setIdentifier(value: Uint8Array | string): void;

    hasDataContract(): boolean;
    clearDataContract(): void;
    getDataContract(): google_protobuf_wrappers_pb.BytesValue | undefined;
    setDataContract(value?: google_protobuf_wrappers_pb.BytesValue): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DataContractEntry.AsObject;
    static toObject(includeInstance: boolean, msg: DataContractEntry): DataContractEntry.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DataContractEntry, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DataContractEntry;
    static deserializeBinaryFromReader(message: DataContractEntry, reader: jspb.BinaryReader): DataContractEntry;
  }

  export namespace DataContractEntry {
    export type AsObject = {
      identifier: Uint8Array | string,
      dataContract?: google_protobuf_wrappers_pb.BytesValue.AsObject,
    }
  }

  export class DataContracts extends jspb.Message {
    clearDataContractEntriesList(): void;
    getDataContractEntriesList(): Array<GetDataContractsResponse.DataContractEntry>;
    setDataContractEntriesList(value: Array<GetDataContractsResponse.DataContractEntry>): void;
    addDataContractEntries(value?: GetDataContractsResponse.DataContractEntry, index?: number): GetDataContractsResponse.DataContractEntry;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): DataContracts.AsObject;
    static toObject(includeInstance: boolean, msg: DataContracts): DataContracts.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: DataContracts, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): DataContracts;
    static deserializeBinaryFromReader(message: DataContracts, reader: jspb.BinaryReader): DataContracts;
  }

  export namespace DataContracts {
    export type AsObject = {
      dataContractEntriesList: Array<GetDataContractsResponse.DataContractEntry.AsObject>,
    }
  }

  export class GetDataContractsResponseV0 extends jspb.Message {
    hasDataContracts(): boolean;
    clearDataContracts(): void;
    getDataContracts(): GetDataContractsResponse.DataContracts | undefined;
    setDataContracts(value?: GetDataContractsResponse.DataContracts): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetDataContractsResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDataContractsResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDataContractsResponseV0): GetDataContractsResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDataContractsResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDataContractsResponseV0;
    static deserializeBinaryFromReader(message: GetDataContractsResponseV0, reader: jspb.BinaryReader): GetDataContractsResponseV0;
  }

  export namespace GetDataContractsResponseV0 {
    export type AsObject = {
      dataContracts?: GetDataContractsResponse.DataContracts.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      DATA_CONTRACTS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDataContractHistoryRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDataContractHistoryRequest.GetDataContractHistoryRequestV0 | undefined;
  setV0(value?: GetDataContractHistoryRequest.GetDataContractHistoryRequestV0): void;

  getVersionCase(): GetDataContractHistoryRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDataContractHistoryRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetDataContractHistoryRequest): GetDataContractHistoryRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDataContractHistoryRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDataContractHistoryRequest;
  static deserializeBinaryFromReader(message: GetDataContractHistoryRequest, reader: jspb.BinaryReader): GetDataContractHistoryRequest;
}

export namespace GetDataContractHistoryRequest {
  export type AsObject = {
    v0?: GetDataContractHistoryRequest.GetDataContractHistoryRequestV0.AsObject,
  }

  export class GetDataContractHistoryRequestV0 extends jspb.Message {
    getId(): Uint8Array | string;
    getId_asU8(): Uint8Array;
    getId_asB64(): string;
    setId(value: Uint8Array | string): void;

    hasLimit(): boolean;
    clearLimit(): void;
    getLimit(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setLimit(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    hasOffset(): boolean;
    clearOffset(): void;
    getOffset(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setOffset(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    getStartAtMs(): number;
    setStartAtMs(value: number): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDataContractHistoryRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDataContractHistoryRequestV0): GetDataContractHistoryRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDataContractHistoryRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDataContractHistoryRequestV0;
    static deserializeBinaryFromReader(message: GetDataContractHistoryRequestV0, reader: jspb.BinaryReader): GetDataContractHistoryRequestV0;
  }

  export namespace GetDataContractHistoryRequestV0 {
    export type AsObject = {
      id: Uint8Array | string,
      limit?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      offset?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      startAtMs: number,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDataContractHistoryResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDataContractHistoryResponse.GetDataContractHistoryResponseV0 | undefined;
  setV0(value?: GetDataContractHistoryResponse.GetDataContractHistoryResponseV0): void;

  getVersionCase(): GetDataContractHistoryResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDataContractHistoryResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetDataContractHistoryResponse): GetDataContractHistoryResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDataContractHistoryResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDataContractHistoryResponse;
  static deserializeBinaryFromReader(message: GetDataContractHistoryResponse, reader: jspb.BinaryReader): GetDataContractHistoryResponse;
}

export namespace GetDataContractHistoryResponse {
  export type AsObject = {
    v0?: GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.AsObject,
  }

  export class GetDataContractHistoryResponseV0 extends jspb.Message {
    hasDataContractHistory(): boolean;
    clearDataContractHistory(): void;
    getDataContractHistory(): GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory | undefined;
    setDataContractHistory(value?: GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetDataContractHistoryResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDataContractHistoryResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDataContractHistoryResponseV0): GetDataContractHistoryResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDataContractHistoryResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDataContractHistoryResponseV0;
    static deserializeBinaryFromReader(message: GetDataContractHistoryResponseV0, reader: jspb.BinaryReader): GetDataContractHistoryResponseV0;
  }

  export namespace GetDataContractHistoryResponseV0 {
    export type AsObject = {
      dataContractHistory?: GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistory.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class DataContractHistoryEntry extends jspb.Message {
      getDate(): number;
      setDate(value: number): void;

      getValue(): Uint8Array | string;
      getValue_asU8(): Uint8Array;
      getValue_asB64(): string;
      setValue(value: Uint8Array | string): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): DataContractHistoryEntry.AsObject;
      static toObject(includeInstance: boolean, msg: DataContractHistoryEntry): DataContractHistoryEntry.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: DataContractHistoryEntry, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): DataContractHistoryEntry;
      static deserializeBinaryFromReader(message: DataContractHistoryEntry, reader: jspb.BinaryReader): DataContractHistoryEntry;
    }

    export namespace DataContractHistoryEntry {
      export type AsObject = {
        date: number,
        value: Uint8Array | string,
      }
    }

    export class DataContractHistory extends jspb.Message {
      clearDataContractEntriesList(): void;
      getDataContractEntriesList(): Array<GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry>;
      setDataContractEntriesList(value: Array<GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry>): void;
      addDataContractEntries(value?: GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry, index?: number): GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): DataContractHistory.AsObject;
      static toObject(includeInstance: boolean, msg: DataContractHistory): DataContractHistory.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: DataContractHistory, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): DataContractHistory;
      static deserializeBinaryFromReader(message: DataContractHistory, reader: jspb.BinaryReader): DataContractHistory;
    }

    export namespace DataContractHistory {
      export type AsObject = {
        dataContractEntriesList: Array<GetDataContractHistoryResponse.GetDataContractHistoryResponseV0.DataContractHistoryEntry.AsObject>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      DATA_CONTRACT_HISTORY = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDocumentsRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDocumentsRequest.GetDocumentsRequestV0 | undefined;
  setV0(value?: GetDocumentsRequest.GetDocumentsRequestV0): void;

  getVersionCase(): GetDocumentsRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDocumentsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetDocumentsRequest): GetDocumentsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDocumentsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDocumentsRequest;
  static deserializeBinaryFromReader(message: GetDocumentsRequest, reader: jspb.BinaryReader): GetDocumentsRequest;
}

export namespace GetDocumentsRequest {
  export type AsObject = {
    v0?: GetDocumentsRequest.GetDocumentsRequestV0.AsObject,
  }

  export class GetDocumentsRequestV0 extends jspb.Message {
    getDataContractId(): Uint8Array | string;
    getDataContractId_asU8(): Uint8Array;
    getDataContractId_asB64(): string;
    setDataContractId(value: Uint8Array | string): void;

    getDocumentType(): string;
    setDocumentType(value: string): void;

    getWhere(): Uint8Array | string;
    getWhere_asU8(): Uint8Array;
    getWhere_asB64(): string;
    setWhere(value: Uint8Array | string): void;

    getOrderBy(): Uint8Array | string;
    getOrderBy_asU8(): Uint8Array;
    getOrderBy_asB64(): string;
    setOrderBy(value: Uint8Array | string): void;

    getLimit(): number;
    setLimit(value: number): void;

    hasStartAfter(): boolean;
    clearStartAfter(): void;
    getStartAfter(): Uint8Array | string;
    getStartAfter_asU8(): Uint8Array;
    getStartAfter_asB64(): string;
    setStartAfter(value: Uint8Array | string): void;

    hasStartAt(): boolean;
    clearStartAt(): void;
    getStartAt(): Uint8Array | string;
    getStartAt_asU8(): Uint8Array;
    getStartAt_asB64(): string;
    setStartAt(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    getStartCase(): GetDocumentsRequestV0.StartCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDocumentsRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDocumentsRequestV0): GetDocumentsRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDocumentsRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDocumentsRequestV0;
    static deserializeBinaryFromReader(message: GetDocumentsRequestV0, reader: jspb.BinaryReader): GetDocumentsRequestV0;
  }

  export namespace GetDocumentsRequestV0 {
    export type AsObject = {
      dataContractId: Uint8Array | string,
      documentType: string,
      where: Uint8Array | string,
      orderBy: Uint8Array | string,
      limit: number,
      startAfter: Uint8Array | string,
      startAt: Uint8Array | string,
      prove: boolean,
    }

    export enum StartCase {
      START_NOT_SET = 0,
      START_AFTER = 6,
      START_AT = 7,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetDocumentsResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetDocumentsResponse.GetDocumentsResponseV0 | undefined;
  setV0(value?: GetDocumentsResponse.GetDocumentsResponseV0): void;

  getVersionCase(): GetDocumentsResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetDocumentsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetDocumentsResponse): GetDocumentsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetDocumentsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetDocumentsResponse;
  static deserializeBinaryFromReader(message: GetDocumentsResponse, reader: jspb.BinaryReader): GetDocumentsResponse;
}

export namespace GetDocumentsResponse {
  export type AsObject = {
    v0?: GetDocumentsResponse.GetDocumentsResponseV0.AsObject,
  }

  export class GetDocumentsResponseV0 extends jspb.Message {
    hasDocuments(): boolean;
    clearDocuments(): void;
    getDocuments(): GetDocumentsResponse.GetDocumentsResponseV0.Documents | undefined;
    setDocuments(value?: GetDocumentsResponse.GetDocumentsResponseV0.Documents): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetDocumentsResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetDocumentsResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetDocumentsResponseV0): GetDocumentsResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetDocumentsResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetDocumentsResponseV0;
    static deserializeBinaryFromReader(message: GetDocumentsResponseV0, reader: jspb.BinaryReader): GetDocumentsResponseV0;
  }

  export namespace GetDocumentsResponseV0 {
    export type AsObject = {
      documents?: GetDocumentsResponse.GetDocumentsResponseV0.Documents.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class Documents extends jspb.Message {
      clearDocumentsList(): void;
      getDocumentsList(): Array<Uint8Array | string>;
      getDocumentsList_asU8(): Array<Uint8Array>;
      getDocumentsList_asB64(): Array<string>;
      setDocumentsList(value: Array<Uint8Array | string>): void;
      addDocuments(value: Uint8Array | string, index?: number): Uint8Array | string;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Documents.AsObject;
      static toObject(includeInstance: boolean, msg: Documents): Documents.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Documents, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Documents;
      static deserializeBinaryFromReader(message: Documents, reader: jspb.BinaryReader): Documents;
    }

    export namespace Documents {
      export type AsObject = {
        documentsList: Array<Uint8Array | string>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      DOCUMENTS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityByPublicKeyHashRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0 | undefined;
  setV0(value?: GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0): void;

  getVersionCase(): GetIdentityByPublicKeyHashRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityByPublicKeyHashRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityByPublicKeyHashRequest): GetIdentityByPublicKeyHashRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityByPublicKeyHashRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityByPublicKeyHashRequest;
  static deserializeBinaryFromReader(message: GetIdentityByPublicKeyHashRequest, reader: jspb.BinaryReader): GetIdentityByPublicKeyHashRequest;
}

export namespace GetIdentityByPublicKeyHashRequest {
  export type AsObject = {
    v0?: GetIdentityByPublicKeyHashRequest.GetIdentityByPublicKeyHashRequestV0.AsObject,
  }

  export class GetIdentityByPublicKeyHashRequestV0 extends jspb.Message {
    getPublicKeyHash(): Uint8Array | string;
    getPublicKeyHash_asU8(): Uint8Array;
    getPublicKeyHash_asB64(): string;
    setPublicKeyHash(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityByPublicKeyHashRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityByPublicKeyHashRequestV0): GetIdentityByPublicKeyHashRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityByPublicKeyHashRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityByPublicKeyHashRequestV0;
    static deserializeBinaryFromReader(message: GetIdentityByPublicKeyHashRequestV0, reader: jspb.BinaryReader): GetIdentityByPublicKeyHashRequestV0;
  }

  export namespace GetIdentityByPublicKeyHashRequestV0 {
    export type AsObject = {
      publicKeyHash: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentityByPublicKeyHashResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0 | undefined;
  setV0(value?: GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0): void;

  getVersionCase(): GetIdentityByPublicKeyHashResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentityByPublicKeyHashResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentityByPublicKeyHashResponse): GetIdentityByPublicKeyHashResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentityByPublicKeyHashResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentityByPublicKeyHashResponse;
  static deserializeBinaryFromReader(message: GetIdentityByPublicKeyHashResponse, reader: jspb.BinaryReader): GetIdentityByPublicKeyHashResponse;
}

export namespace GetIdentityByPublicKeyHashResponse {
  export type AsObject = {
    v0?: GetIdentityByPublicKeyHashResponse.GetIdentityByPublicKeyHashResponseV0.AsObject,
  }

  export class GetIdentityByPublicKeyHashResponseV0 extends jspb.Message {
    hasIdentity(): boolean;
    clearIdentity(): void;
    getIdentity(): Uint8Array | string;
    getIdentity_asU8(): Uint8Array;
    getIdentity_asB64(): string;
    setIdentity(value: Uint8Array | string): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentityByPublicKeyHashResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentityByPublicKeyHashResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentityByPublicKeyHashResponseV0): GetIdentityByPublicKeyHashResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentityByPublicKeyHashResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentityByPublicKeyHashResponseV0;
    static deserializeBinaryFromReader(message: GetIdentityByPublicKeyHashResponseV0, reader: jspb.BinaryReader): GetIdentityByPublicKeyHashResponseV0;
  }

  export namespace GetIdentityByPublicKeyHashResponseV0 {
    export type AsObject = {
      identity: Uint8Array | string,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITY = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class WaitForStateTransitionResultRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0 | undefined;
  setV0(value?: WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0): void;

  getVersionCase(): WaitForStateTransitionResultRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): WaitForStateTransitionResultRequest.AsObject;
  static toObject(includeInstance: boolean, msg: WaitForStateTransitionResultRequest): WaitForStateTransitionResultRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: WaitForStateTransitionResultRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): WaitForStateTransitionResultRequest;
  static deserializeBinaryFromReader(message: WaitForStateTransitionResultRequest, reader: jspb.BinaryReader): WaitForStateTransitionResultRequest;
}

export namespace WaitForStateTransitionResultRequest {
  export type AsObject = {
    v0?: WaitForStateTransitionResultRequest.WaitForStateTransitionResultRequestV0.AsObject,
  }

  export class WaitForStateTransitionResultRequestV0 extends jspb.Message {
    getStateTransitionHash(): Uint8Array | string;
    getStateTransitionHash_asU8(): Uint8Array;
    getStateTransitionHash_asB64(): string;
    setStateTransitionHash(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WaitForStateTransitionResultRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: WaitForStateTransitionResultRequestV0): WaitForStateTransitionResultRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WaitForStateTransitionResultRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WaitForStateTransitionResultRequestV0;
    static deserializeBinaryFromReader(message: WaitForStateTransitionResultRequestV0, reader: jspb.BinaryReader): WaitForStateTransitionResultRequestV0;
  }

  export namespace WaitForStateTransitionResultRequestV0 {
    export type AsObject = {
      stateTransitionHash: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class WaitForStateTransitionResultResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0 | undefined;
  setV0(value?: WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0): void;

  getVersionCase(): WaitForStateTransitionResultResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): WaitForStateTransitionResultResponse.AsObject;
  static toObject(includeInstance: boolean, msg: WaitForStateTransitionResultResponse): WaitForStateTransitionResultResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: WaitForStateTransitionResultResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): WaitForStateTransitionResultResponse;
  static deserializeBinaryFromReader(message: WaitForStateTransitionResultResponse, reader: jspb.BinaryReader): WaitForStateTransitionResultResponse;
}

export namespace WaitForStateTransitionResultResponse {
  export type AsObject = {
    v0?: WaitForStateTransitionResultResponse.WaitForStateTransitionResultResponseV0.AsObject,
  }

  export class WaitForStateTransitionResultResponseV0 extends jspb.Message {
    hasError(): boolean;
    clearError(): void;
    getError(): StateTransitionBroadcastError | undefined;
    setError(value?: StateTransitionBroadcastError): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): WaitForStateTransitionResultResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): WaitForStateTransitionResultResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: WaitForStateTransitionResultResponseV0): WaitForStateTransitionResultResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: WaitForStateTransitionResultResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): WaitForStateTransitionResultResponseV0;
    static deserializeBinaryFromReader(message: WaitForStateTransitionResultResponseV0, reader: jspb.BinaryReader): WaitForStateTransitionResultResponseV0;
  }

  export namespace WaitForStateTransitionResultResponseV0 {
    export type AsObject = {
      error?: StateTransitionBroadcastError.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      ERROR = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetConsensusParamsRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetConsensusParamsRequest.GetConsensusParamsRequestV0 | undefined;
  setV0(value?: GetConsensusParamsRequest.GetConsensusParamsRequestV0): void;

  getVersionCase(): GetConsensusParamsRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetConsensusParamsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetConsensusParamsRequest): GetConsensusParamsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetConsensusParamsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetConsensusParamsRequest;
  static deserializeBinaryFromReader(message: GetConsensusParamsRequest, reader: jspb.BinaryReader): GetConsensusParamsRequest;
}

export namespace GetConsensusParamsRequest {
  export type AsObject = {
    v0?: GetConsensusParamsRequest.GetConsensusParamsRequestV0.AsObject,
  }

  export class GetConsensusParamsRequestV0 extends jspb.Message {
    getHeight(): number;
    setHeight(value: number): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetConsensusParamsRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetConsensusParamsRequestV0): GetConsensusParamsRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetConsensusParamsRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetConsensusParamsRequestV0;
    static deserializeBinaryFromReader(message: GetConsensusParamsRequestV0, reader: jspb.BinaryReader): GetConsensusParamsRequestV0;
  }

  export namespace GetConsensusParamsRequestV0 {
    export type AsObject = {
      height: number,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetConsensusParamsResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetConsensusParamsResponse.GetConsensusParamsResponseV0 | undefined;
  setV0(value?: GetConsensusParamsResponse.GetConsensusParamsResponseV0): void;

  getVersionCase(): GetConsensusParamsResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetConsensusParamsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetConsensusParamsResponse): GetConsensusParamsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetConsensusParamsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetConsensusParamsResponse;
  static deserializeBinaryFromReader(message: GetConsensusParamsResponse, reader: jspb.BinaryReader): GetConsensusParamsResponse;
}

export namespace GetConsensusParamsResponse {
  export type AsObject = {
    v0?: GetConsensusParamsResponse.GetConsensusParamsResponseV0.AsObject,
  }

  export class ConsensusParamsBlock extends jspb.Message {
    getMaxBytes(): string;
    setMaxBytes(value: string): void;

    getMaxGas(): string;
    setMaxGas(value: string): void;

    getTimeIotaMs(): string;
    setTimeIotaMs(value: string): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ConsensusParamsBlock.AsObject;
    static toObject(includeInstance: boolean, msg: ConsensusParamsBlock): ConsensusParamsBlock.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ConsensusParamsBlock, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ConsensusParamsBlock;
    static deserializeBinaryFromReader(message: ConsensusParamsBlock, reader: jspb.BinaryReader): ConsensusParamsBlock;
  }

  export namespace ConsensusParamsBlock {
    export type AsObject = {
      maxBytes: string,
      maxGas: string,
      timeIotaMs: string,
    }
  }

  export class ConsensusParamsEvidence extends jspb.Message {
    getMaxAgeNumBlocks(): string;
    setMaxAgeNumBlocks(value: string): void;

    getMaxAgeDuration(): string;
    setMaxAgeDuration(value: string): void;

    getMaxBytes(): string;
    setMaxBytes(value: string): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ConsensusParamsEvidence.AsObject;
    static toObject(includeInstance: boolean, msg: ConsensusParamsEvidence): ConsensusParamsEvidence.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ConsensusParamsEvidence, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ConsensusParamsEvidence;
    static deserializeBinaryFromReader(message: ConsensusParamsEvidence, reader: jspb.BinaryReader): ConsensusParamsEvidence;
  }

  export namespace ConsensusParamsEvidence {
    export type AsObject = {
      maxAgeNumBlocks: string,
      maxAgeDuration: string,
      maxBytes: string,
    }
  }

  export class GetConsensusParamsResponseV0 extends jspb.Message {
    hasBlock(): boolean;
    clearBlock(): void;
    getBlock(): GetConsensusParamsResponse.ConsensusParamsBlock | undefined;
    setBlock(value?: GetConsensusParamsResponse.ConsensusParamsBlock): void;

    hasEvidence(): boolean;
    clearEvidence(): void;
    getEvidence(): GetConsensusParamsResponse.ConsensusParamsEvidence | undefined;
    setEvidence(value?: GetConsensusParamsResponse.ConsensusParamsEvidence): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetConsensusParamsResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetConsensusParamsResponseV0): GetConsensusParamsResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetConsensusParamsResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetConsensusParamsResponseV0;
    static deserializeBinaryFromReader(message: GetConsensusParamsResponseV0, reader: jspb.BinaryReader): GetConsensusParamsResponseV0;
  }

  export namespace GetConsensusParamsResponseV0 {
    export type AsObject = {
      block?: GetConsensusParamsResponse.ConsensusParamsBlock.AsObject,
      evidence?: GetConsensusParamsResponse.ConsensusParamsEvidence.AsObject,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetProtocolVersionUpgradeStateRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0 | undefined;
  setV0(value?: GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0): void;

  getVersionCase(): GetProtocolVersionUpgradeStateRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetProtocolVersionUpgradeStateRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeStateRequest): GetProtocolVersionUpgradeStateRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetProtocolVersionUpgradeStateRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeStateRequest;
  static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeStateRequest, reader: jspb.BinaryReader): GetProtocolVersionUpgradeStateRequest;
}

export namespace GetProtocolVersionUpgradeStateRequest {
  export type AsObject = {
    v0?: GetProtocolVersionUpgradeStateRequest.GetProtocolVersionUpgradeStateRequestV0.AsObject,
  }

  export class GetProtocolVersionUpgradeStateRequestV0 extends jspb.Message {
    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetProtocolVersionUpgradeStateRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeStateRequestV0): GetProtocolVersionUpgradeStateRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetProtocolVersionUpgradeStateRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeStateRequestV0;
    static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeStateRequestV0, reader: jspb.BinaryReader): GetProtocolVersionUpgradeStateRequestV0;
  }

  export namespace GetProtocolVersionUpgradeStateRequestV0 {
    export type AsObject = {
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetProtocolVersionUpgradeStateResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0 | undefined;
  setV0(value?: GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0): void;

  getVersionCase(): GetProtocolVersionUpgradeStateResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetProtocolVersionUpgradeStateResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeStateResponse): GetProtocolVersionUpgradeStateResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetProtocolVersionUpgradeStateResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeStateResponse;
  static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeStateResponse, reader: jspb.BinaryReader): GetProtocolVersionUpgradeStateResponse;
}

export namespace GetProtocolVersionUpgradeStateResponse {
  export type AsObject = {
    v0?: GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.AsObject,
  }

  export class GetProtocolVersionUpgradeStateResponseV0 extends jspb.Message {
    hasVersions(): boolean;
    clearVersions(): void;
    getVersions(): GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions | undefined;
    setVersions(value?: GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetProtocolVersionUpgradeStateResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetProtocolVersionUpgradeStateResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeStateResponseV0): GetProtocolVersionUpgradeStateResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetProtocolVersionUpgradeStateResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeStateResponseV0;
    static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeStateResponseV0, reader: jspb.BinaryReader): GetProtocolVersionUpgradeStateResponseV0;
  }

  export namespace GetProtocolVersionUpgradeStateResponseV0 {
    export type AsObject = {
      versions?: GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.Versions.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class Versions extends jspb.Message {
      clearVersionsList(): void;
      getVersionsList(): Array<GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry>;
      setVersionsList(value: Array<GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry>): void;
      addVersions(value?: GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry, index?: number): GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Versions.AsObject;
      static toObject(includeInstance: boolean, msg: Versions): Versions.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Versions, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Versions;
      static deserializeBinaryFromReader(message: Versions, reader: jspb.BinaryReader): Versions;
    }

    export namespace Versions {
      export type AsObject = {
        versionsList: Array<GetProtocolVersionUpgradeStateResponse.GetProtocolVersionUpgradeStateResponseV0.VersionEntry.AsObject>,
      }
    }

    export class VersionEntry extends jspb.Message {
      getVersionNumber(): number;
      setVersionNumber(value: number): void;

      getVoteCount(): number;
      setVoteCount(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): VersionEntry.AsObject;
      static toObject(includeInstance: boolean, msg: VersionEntry): VersionEntry.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: VersionEntry, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): VersionEntry;
      static deserializeBinaryFromReader(message: VersionEntry, reader: jspb.BinaryReader): VersionEntry;
    }

    export namespace VersionEntry {
      export type AsObject = {
        versionNumber: number,
        voteCount: number,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      VERSIONS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetProtocolVersionUpgradeVoteStatusRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0 | undefined;
  setV0(value?: GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0): void;

  getVersionCase(): GetProtocolVersionUpgradeVoteStatusRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetProtocolVersionUpgradeVoteStatusRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeVoteStatusRequest): GetProtocolVersionUpgradeVoteStatusRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetProtocolVersionUpgradeVoteStatusRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeVoteStatusRequest;
  static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeVoteStatusRequest, reader: jspb.BinaryReader): GetProtocolVersionUpgradeVoteStatusRequest;
}

export namespace GetProtocolVersionUpgradeVoteStatusRequest {
  export type AsObject = {
    v0?: GetProtocolVersionUpgradeVoteStatusRequest.GetProtocolVersionUpgradeVoteStatusRequestV0.AsObject,
  }

  export class GetProtocolVersionUpgradeVoteStatusRequestV0 extends jspb.Message {
    getStartProTxHash(): Uint8Array | string;
    getStartProTxHash_asU8(): Uint8Array;
    getStartProTxHash_asB64(): string;
    setStartProTxHash(value: Uint8Array | string): void;

    getCount(): number;
    setCount(value: number): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetProtocolVersionUpgradeVoteStatusRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeVoteStatusRequestV0): GetProtocolVersionUpgradeVoteStatusRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetProtocolVersionUpgradeVoteStatusRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeVoteStatusRequestV0;
    static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeVoteStatusRequestV0, reader: jspb.BinaryReader): GetProtocolVersionUpgradeVoteStatusRequestV0;
  }

  export namespace GetProtocolVersionUpgradeVoteStatusRequestV0 {
    export type AsObject = {
      startProTxHash: Uint8Array | string,
      count: number,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetProtocolVersionUpgradeVoteStatusResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0 | undefined;
  setV0(value?: GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0): void;

  getVersionCase(): GetProtocolVersionUpgradeVoteStatusResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetProtocolVersionUpgradeVoteStatusResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeVoteStatusResponse): GetProtocolVersionUpgradeVoteStatusResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetProtocolVersionUpgradeVoteStatusResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeVoteStatusResponse;
  static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeVoteStatusResponse, reader: jspb.BinaryReader): GetProtocolVersionUpgradeVoteStatusResponse;
}

export namespace GetProtocolVersionUpgradeVoteStatusResponse {
  export type AsObject = {
    v0?: GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.AsObject,
  }

  export class GetProtocolVersionUpgradeVoteStatusResponseV0 extends jspb.Message {
    hasVersions(): boolean;
    clearVersions(): void;
    getVersions(): GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals | undefined;
    setVersions(value?: GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetProtocolVersionUpgradeVoteStatusResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetProtocolVersionUpgradeVoteStatusResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetProtocolVersionUpgradeVoteStatusResponseV0): GetProtocolVersionUpgradeVoteStatusResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetProtocolVersionUpgradeVoteStatusResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetProtocolVersionUpgradeVoteStatusResponseV0;
    static deserializeBinaryFromReader(message: GetProtocolVersionUpgradeVoteStatusResponseV0, reader: jspb.BinaryReader): GetProtocolVersionUpgradeVoteStatusResponseV0;
  }

  export namespace GetProtocolVersionUpgradeVoteStatusResponseV0 {
    export type AsObject = {
      versions?: GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignals.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class VersionSignals extends jspb.Message {
      clearVersionSignalsList(): void;
      getVersionSignalsList(): Array<GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal>;
      setVersionSignalsList(value: Array<GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal>): void;
      addVersionSignals(value?: GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal, index?: number): GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): VersionSignals.AsObject;
      static toObject(includeInstance: boolean, msg: VersionSignals): VersionSignals.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: VersionSignals, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): VersionSignals;
      static deserializeBinaryFromReader(message: VersionSignals, reader: jspb.BinaryReader): VersionSignals;
    }

    export namespace VersionSignals {
      export type AsObject = {
        versionSignalsList: Array<GetProtocolVersionUpgradeVoteStatusResponse.GetProtocolVersionUpgradeVoteStatusResponseV0.VersionSignal.AsObject>,
      }
    }

    export class VersionSignal extends jspb.Message {
      getProTxHash(): Uint8Array | string;
      getProTxHash_asU8(): Uint8Array;
      getProTxHash_asB64(): string;
      setProTxHash(value: Uint8Array | string): void;

      getVersion(): number;
      setVersion(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): VersionSignal.AsObject;
      static toObject(includeInstance: boolean, msg: VersionSignal): VersionSignal.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: VersionSignal, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): VersionSignal;
      static deserializeBinaryFromReader(message: VersionSignal, reader: jspb.BinaryReader): VersionSignal;
    }

    export namespace VersionSignal {
      export type AsObject = {
        proTxHash: Uint8Array | string,
        version: number,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      VERSIONS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetEpochsInfoRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetEpochsInfoRequest.GetEpochsInfoRequestV0 | undefined;
  setV0(value?: GetEpochsInfoRequest.GetEpochsInfoRequestV0): void;

  getVersionCase(): GetEpochsInfoRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEpochsInfoRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetEpochsInfoRequest): GetEpochsInfoRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEpochsInfoRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEpochsInfoRequest;
  static deserializeBinaryFromReader(message: GetEpochsInfoRequest, reader: jspb.BinaryReader): GetEpochsInfoRequest;
}

export namespace GetEpochsInfoRequest {
  export type AsObject = {
    v0?: GetEpochsInfoRequest.GetEpochsInfoRequestV0.AsObject,
  }

  export class GetEpochsInfoRequestV0 extends jspb.Message {
    hasStartEpoch(): boolean;
    clearStartEpoch(): void;
    getStartEpoch(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setStartEpoch(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    getCount(): number;
    setCount(value: number): void;

    getAscending(): boolean;
    setAscending(value: boolean): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetEpochsInfoRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetEpochsInfoRequestV0): GetEpochsInfoRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetEpochsInfoRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetEpochsInfoRequestV0;
    static deserializeBinaryFromReader(message: GetEpochsInfoRequestV0, reader: jspb.BinaryReader): GetEpochsInfoRequestV0;
  }

  export namespace GetEpochsInfoRequestV0 {
    export type AsObject = {
      startEpoch?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      count: number,
      ascending: boolean,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetEpochsInfoResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetEpochsInfoResponse.GetEpochsInfoResponseV0 | undefined;
  setV0(value?: GetEpochsInfoResponse.GetEpochsInfoResponseV0): void;

  getVersionCase(): GetEpochsInfoResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEpochsInfoResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetEpochsInfoResponse): GetEpochsInfoResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEpochsInfoResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEpochsInfoResponse;
  static deserializeBinaryFromReader(message: GetEpochsInfoResponse, reader: jspb.BinaryReader): GetEpochsInfoResponse;
}

export namespace GetEpochsInfoResponse {
  export type AsObject = {
    v0?: GetEpochsInfoResponse.GetEpochsInfoResponseV0.AsObject,
  }

  export class GetEpochsInfoResponseV0 extends jspb.Message {
    hasEpochs(): boolean;
    clearEpochs(): void;
    getEpochs(): GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos | undefined;
    setEpochs(value?: GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetEpochsInfoResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetEpochsInfoResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetEpochsInfoResponseV0): GetEpochsInfoResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetEpochsInfoResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetEpochsInfoResponseV0;
    static deserializeBinaryFromReader(message: GetEpochsInfoResponseV0, reader: jspb.BinaryReader): GetEpochsInfoResponseV0;
  }

  export namespace GetEpochsInfoResponseV0 {
    export type AsObject = {
      epochs?: GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfos.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class EpochInfos extends jspb.Message {
      clearEpochInfosList(): void;
      getEpochInfosList(): Array<GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo>;
      setEpochInfosList(value: Array<GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo>): void;
      addEpochInfos(value?: GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo, index?: number): GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): EpochInfos.AsObject;
      static toObject(includeInstance: boolean, msg: EpochInfos): EpochInfos.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: EpochInfos, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): EpochInfos;
      static deserializeBinaryFromReader(message: EpochInfos, reader: jspb.BinaryReader): EpochInfos;
    }

    export namespace EpochInfos {
      export type AsObject = {
        epochInfosList: Array<GetEpochsInfoResponse.GetEpochsInfoResponseV0.EpochInfo.AsObject>,
      }
    }

    export class EpochInfo extends jspb.Message {
      getNumber(): number;
      setNumber(value: number): void;

      getFirstBlockHeight(): number;
      setFirstBlockHeight(value: number): void;

      getFirstCoreBlockHeight(): number;
      setFirstCoreBlockHeight(value: number): void;

      getStartTime(): number;
      setStartTime(value: number): void;

      getFeeMultiplier(): number;
      setFeeMultiplier(value: number): void;

      getProtocolVersion(): number;
      setProtocolVersion(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): EpochInfo.AsObject;
      static toObject(includeInstance: boolean, msg: EpochInfo): EpochInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: EpochInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): EpochInfo;
      static deserializeBinaryFromReader(message: EpochInfo, reader: jspb.BinaryReader): EpochInfo;
    }

    export namespace EpochInfo {
      export type AsObject = {
        number: number,
        firstBlockHeight: number,
        firstCoreBlockHeight: number,
        startTime: number,
        feeMultiplier: number,
        protocolVersion: number,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      EPOCHS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourcesRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourcesRequest.GetContestedResourcesRequestV0 | undefined;
  setV0(value?: GetContestedResourcesRequest.GetContestedResourcesRequestV0): void;

  getVersionCase(): GetContestedResourcesRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourcesRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourcesRequest): GetContestedResourcesRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourcesRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourcesRequest;
  static deserializeBinaryFromReader(message: GetContestedResourcesRequest, reader: jspb.BinaryReader): GetContestedResourcesRequest;
}

export namespace GetContestedResourcesRequest {
  export type AsObject = {
    v0?: GetContestedResourcesRequest.GetContestedResourcesRequestV0.AsObject,
  }

  export class GetContestedResourcesRequestV0 extends jspb.Message {
    getContractId(): Uint8Array | string;
    getContractId_asU8(): Uint8Array;
    getContractId_asB64(): string;
    setContractId(value: Uint8Array | string): void;

    getDocumentTypeName(): string;
    setDocumentTypeName(value: string): void;

    getIndexName(): string;
    setIndexName(value: string): void;

    clearStartIndexValuesList(): void;
    getStartIndexValuesList(): Array<Uint8Array | string>;
    getStartIndexValuesList_asU8(): Array<Uint8Array>;
    getStartIndexValuesList_asB64(): Array<string>;
    setStartIndexValuesList(value: Array<Uint8Array | string>): void;
    addStartIndexValues(value: Uint8Array | string, index?: number): Uint8Array | string;

    clearEndIndexValuesList(): void;
    getEndIndexValuesList(): Array<Uint8Array | string>;
    getEndIndexValuesList_asU8(): Array<Uint8Array>;
    getEndIndexValuesList_asB64(): Array<string>;
    setEndIndexValuesList(value: Array<Uint8Array | string>): void;
    addEndIndexValues(value: Uint8Array | string, index?: number): Uint8Array | string;

    hasStartAtValueInfo(): boolean;
    clearStartAtValueInfo(): void;
    getStartAtValueInfo(): GetContestedResourcesRequest.GetContestedResourcesRequestV0.StartAtValueInfo | undefined;
    setStartAtValueInfo(value?: GetContestedResourcesRequest.GetContestedResourcesRequestV0.StartAtValueInfo): void;

    hasCount(): boolean;
    clearCount(): void;
    getCount(): number;
    setCount(value: number): void;

    getOrderAscending(): boolean;
    setOrderAscending(value: boolean): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourcesRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourcesRequestV0): GetContestedResourcesRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourcesRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourcesRequestV0;
    static deserializeBinaryFromReader(message: GetContestedResourcesRequestV0, reader: jspb.BinaryReader): GetContestedResourcesRequestV0;
  }

  export namespace GetContestedResourcesRequestV0 {
    export type AsObject = {
      contractId: Uint8Array | string,
      documentTypeName: string,
      indexName: string,
      startIndexValuesList: Array<Uint8Array | string>,
      endIndexValuesList: Array<Uint8Array | string>,
      startAtValueInfo?: GetContestedResourcesRequest.GetContestedResourcesRequestV0.StartAtValueInfo.AsObject,
      count: number,
      orderAscending: boolean,
      prove: boolean,
    }

    export class StartAtValueInfo extends jspb.Message {
      getStartValue(): Uint8Array | string;
      getStartValue_asU8(): Uint8Array;
      getStartValue_asB64(): string;
      setStartValue(value: Uint8Array | string): void;

      getStartValueIncluded(): boolean;
      setStartValueIncluded(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): StartAtValueInfo.AsObject;
      static toObject(includeInstance: boolean, msg: StartAtValueInfo): StartAtValueInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: StartAtValueInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): StartAtValueInfo;
      static deserializeBinaryFromReader(message: StartAtValueInfo, reader: jspb.BinaryReader): StartAtValueInfo;
    }

    export namespace StartAtValueInfo {
      export type AsObject = {
        startValue: Uint8Array | string,
        startValueIncluded: boolean,
      }
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourcesResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourcesResponse.GetContestedResourcesResponseV0 | undefined;
  setV0(value?: GetContestedResourcesResponse.GetContestedResourcesResponseV0): void;

  getVersionCase(): GetContestedResourcesResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourcesResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourcesResponse): GetContestedResourcesResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourcesResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourcesResponse;
  static deserializeBinaryFromReader(message: GetContestedResourcesResponse, reader: jspb.BinaryReader): GetContestedResourcesResponse;
}

export namespace GetContestedResourcesResponse {
  export type AsObject = {
    v0?: GetContestedResourcesResponse.GetContestedResourcesResponseV0.AsObject,
  }

  export class GetContestedResourcesResponseV0 extends jspb.Message {
    hasContestedResourceValues(): boolean;
    clearContestedResourceValues(): void;
    getContestedResourceValues(): GetContestedResourcesResponse.GetContestedResourcesResponseV0.ContestedResourceValues | undefined;
    setContestedResourceValues(value?: GetContestedResourcesResponse.GetContestedResourcesResponseV0.ContestedResourceValues): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetContestedResourcesResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourcesResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourcesResponseV0): GetContestedResourcesResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourcesResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourcesResponseV0;
    static deserializeBinaryFromReader(message: GetContestedResourcesResponseV0, reader: jspb.BinaryReader): GetContestedResourcesResponseV0;
  }

  export namespace GetContestedResourcesResponseV0 {
    export type AsObject = {
      contestedResourceValues?: GetContestedResourcesResponse.GetContestedResourcesResponseV0.ContestedResourceValues.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class ContestedResourceValues extends jspb.Message {
      clearContestedResourceValuesList(): void;
      getContestedResourceValuesList(): Array<Uint8Array | string>;
      getContestedResourceValuesList_asU8(): Array<Uint8Array>;
      getContestedResourceValuesList_asB64(): Array<string>;
      setContestedResourceValuesList(value: Array<Uint8Array | string>): void;
      addContestedResourceValues(value: Uint8Array | string, index?: number): Uint8Array | string;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ContestedResourceValues.AsObject;
      static toObject(includeInstance: boolean, msg: ContestedResourceValues): ContestedResourceValues.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ContestedResourceValues, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ContestedResourceValues;
      static deserializeBinaryFromReader(message: ContestedResourceValues, reader: jspb.BinaryReader): ContestedResourceValues;
    }

    export namespace ContestedResourceValues {
      export type AsObject = {
        contestedResourceValuesList: Array<Uint8Array | string>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      CONTESTED_RESOURCE_VALUES = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetVotePollsByEndDateRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0 | undefined;
  setV0(value?: GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0): void;

  getVersionCase(): GetVotePollsByEndDateRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetVotePollsByEndDateRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetVotePollsByEndDateRequest): GetVotePollsByEndDateRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetVotePollsByEndDateRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetVotePollsByEndDateRequest;
  static deserializeBinaryFromReader(message: GetVotePollsByEndDateRequest, reader: jspb.BinaryReader): GetVotePollsByEndDateRequest;
}

export namespace GetVotePollsByEndDateRequest {
  export type AsObject = {
    v0?: GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.AsObject,
  }

  export class GetVotePollsByEndDateRequestV0 extends jspb.Message {
    hasStartTimeInfo(): boolean;
    clearStartTimeInfo(): void;
    getStartTimeInfo(): GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.StartAtTimeInfo | undefined;
    setStartTimeInfo(value?: GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.StartAtTimeInfo): void;

    hasEndTimeInfo(): boolean;
    clearEndTimeInfo(): void;
    getEndTimeInfo(): GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.EndAtTimeInfo | undefined;
    setEndTimeInfo(value?: GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.EndAtTimeInfo): void;

    hasLimit(): boolean;
    clearLimit(): void;
    getLimit(): number;
    setLimit(value: number): void;

    hasOffset(): boolean;
    clearOffset(): void;
    getOffset(): number;
    setOffset(value: number): void;

    getAscending(): boolean;
    setAscending(value: boolean): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetVotePollsByEndDateRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetVotePollsByEndDateRequestV0): GetVotePollsByEndDateRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetVotePollsByEndDateRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetVotePollsByEndDateRequestV0;
    static deserializeBinaryFromReader(message: GetVotePollsByEndDateRequestV0, reader: jspb.BinaryReader): GetVotePollsByEndDateRequestV0;
  }

  export namespace GetVotePollsByEndDateRequestV0 {
    export type AsObject = {
      startTimeInfo?: GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.StartAtTimeInfo.AsObject,
      endTimeInfo?: GetVotePollsByEndDateRequest.GetVotePollsByEndDateRequestV0.EndAtTimeInfo.AsObject,
      limit: number,
      offset: number,
      ascending: boolean,
      prove: boolean,
    }

    export class StartAtTimeInfo extends jspb.Message {
      getStartTimeMs(): number;
      setStartTimeMs(value: number): void;

      getStartTimeIncluded(): boolean;
      setStartTimeIncluded(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): StartAtTimeInfo.AsObject;
      static toObject(includeInstance: boolean, msg: StartAtTimeInfo): StartAtTimeInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: StartAtTimeInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): StartAtTimeInfo;
      static deserializeBinaryFromReader(message: StartAtTimeInfo, reader: jspb.BinaryReader): StartAtTimeInfo;
    }

    export namespace StartAtTimeInfo {
      export type AsObject = {
        startTimeMs: number,
        startTimeIncluded: boolean,
      }
    }

    export class EndAtTimeInfo extends jspb.Message {
      getEndTimeMs(): number;
      setEndTimeMs(value: number): void;

      getEndTimeIncluded(): boolean;
      setEndTimeIncluded(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): EndAtTimeInfo.AsObject;
      static toObject(includeInstance: boolean, msg: EndAtTimeInfo): EndAtTimeInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: EndAtTimeInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): EndAtTimeInfo;
      static deserializeBinaryFromReader(message: EndAtTimeInfo, reader: jspb.BinaryReader): EndAtTimeInfo;
    }

    export namespace EndAtTimeInfo {
      export type AsObject = {
        endTimeMs: number,
        endTimeIncluded: boolean,
      }
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetVotePollsByEndDateResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0 | undefined;
  setV0(value?: GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0): void;

  getVersionCase(): GetVotePollsByEndDateResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetVotePollsByEndDateResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetVotePollsByEndDateResponse): GetVotePollsByEndDateResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetVotePollsByEndDateResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetVotePollsByEndDateResponse;
  static deserializeBinaryFromReader(message: GetVotePollsByEndDateResponse, reader: jspb.BinaryReader): GetVotePollsByEndDateResponse;
}

export namespace GetVotePollsByEndDateResponse {
  export type AsObject = {
    v0?: GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.AsObject,
  }

  export class GetVotePollsByEndDateResponseV0 extends jspb.Message {
    hasVotePollsByTimestamps(): boolean;
    clearVotePollsByTimestamps(): void;
    getVotePollsByTimestamps(): GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamps | undefined;
    setVotePollsByTimestamps(value?: GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamps): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetVotePollsByEndDateResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetVotePollsByEndDateResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetVotePollsByEndDateResponseV0): GetVotePollsByEndDateResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetVotePollsByEndDateResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetVotePollsByEndDateResponseV0;
    static deserializeBinaryFromReader(message: GetVotePollsByEndDateResponseV0, reader: jspb.BinaryReader): GetVotePollsByEndDateResponseV0;
  }

  export namespace GetVotePollsByEndDateResponseV0 {
    export type AsObject = {
      votePollsByTimestamps?: GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamps.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class SerializedVotePollsByTimestamp extends jspb.Message {
      getTimestamp(): number;
      setTimestamp(value: number): void;

      clearSerializedVotePollsList(): void;
      getSerializedVotePollsList(): Array<Uint8Array | string>;
      getSerializedVotePollsList_asU8(): Array<Uint8Array>;
      getSerializedVotePollsList_asB64(): Array<string>;
      setSerializedVotePollsList(value: Array<Uint8Array | string>): void;
      addSerializedVotePolls(value: Uint8Array | string, index?: number): Uint8Array | string;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): SerializedVotePollsByTimestamp.AsObject;
      static toObject(includeInstance: boolean, msg: SerializedVotePollsByTimestamp): SerializedVotePollsByTimestamp.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: SerializedVotePollsByTimestamp, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): SerializedVotePollsByTimestamp;
      static deserializeBinaryFromReader(message: SerializedVotePollsByTimestamp, reader: jspb.BinaryReader): SerializedVotePollsByTimestamp;
    }

    export namespace SerializedVotePollsByTimestamp {
      export type AsObject = {
        timestamp: number,
        serializedVotePollsList: Array<Uint8Array | string>,
      }
    }

    export class SerializedVotePollsByTimestamps extends jspb.Message {
      clearVotePollsByTimestampsList(): void;
      getVotePollsByTimestampsList(): Array<GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamp>;
      setVotePollsByTimestampsList(value: Array<GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamp>): void;
      addVotePollsByTimestamps(value?: GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamp, index?: number): GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamp;

      getFinishedResults(): boolean;
      setFinishedResults(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): SerializedVotePollsByTimestamps.AsObject;
      static toObject(includeInstance: boolean, msg: SerializedVotePollsByTimestamps): SerializedVotePollsByTimestamps.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: SerializedVotePollsByTimestamps, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): SerializedVotePollsByTimestamps;
      static deserializeBinaryFromReader(message: SerializedVotePollsByTimestamps, reader: jspb.BinaryReader): SerializedVotePollsByTimestamps;
    }

    export namespace SerializedVotePollsByTimestamps {
      export type AsObject = {
        votePollsByTimestampsList: Array<GetVotePollsByEndDateResponse.GetVotePollsByEndDateResponseV0.SerializedVotePollsByTimestamp.AsObject>,
        finishedResults: boolean,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      VOTE_POLLS_BY_TIMESTAMPS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourceVoteStateRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0 | undefined;
  setV0(value?: GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0): void;

  getVersionCase(): GetContestedResourceVoteStateRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourceVoteStateRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourceVoteStateRequest): GetContestedResourceVoteStateRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourceVoteStateRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourceVoteStateRequest;
  static deserializeBinaryFromReader(message: GetContestedResourceVoteStateRequest, reader: jspb.BinaryReader): GetContestedResourceVoteStateRequest;
}

export namespace GetContestedResourceVoteStateRequest {
  export type AsObject = {
    v0?: GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.AsObject,
  }

  export class GetContestedResourceVoteStateRequestV0 extends jspb.Message {
    getContractId(): Uint8Array | string;
    getContractId_asU8(): Uint8Array;
    getContractId_asB64(): string;
    setContractId(value: Uint8Array | string): void;

    getDocumentTypeName(): string;
    setDocumentTypeName(value: string): void;

    getIndexName(): string;
    setIndexName(value: string): void;

    clearIndexValuesList(): void;
    getIndexValuesList(): Array<Uint8Array | string>;
    getIndexValuesList_asU8(): Array<Uint8Array>;
    getIndexValuesList_asB64(): Array<string>;
    setIndexValuesList(value: Array<Uint8Array | string>): void;
    addIndexValues(value: Uint8Array | string, index?: number): Uint8Array | string;

    getResultType(): GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.ResultTypeMap[keyof GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.ResultTypeMap];
    setResultType(value: GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.ResultTypeMap[keyof GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.ResultTypeMap]): void;

    getAllowIncludeLockedAndAbstainingVoteTally(): boolean;
    setAllowIncludeLockedAndAbstainingVoteTally(value: boolean): void;

    hasStartAtIdentifierInfo(): boolean;
    clearStartAtIdentifierInfo(): void;
    getStartAtIdentifierInfo(): GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.StartAtIdentifierInfo | undefined;
    setStartAtIdentifierInfo(value?: GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.StartAtIdentifierInfo): void;

    hasCount(): boolean;
    clearCount(): void;
    getCount(): number;
    setCount(value: number): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourceVoteStateRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourceVoteStateRequestV0): GetContestedResourceVoteStateRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourceVoteStateRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourceVoteStateRequestV0;
    static deserializeBinaryFromReader(message: GetContestedResourceVoteStateRequestV0, reader: jspb.BinaryReader): GetContestedResourceVoteStateRequestV0;
  }

  export namespace GetContestedResourceVoteStateRequestV0 {
    export type AsObject = {
      contractId: Uint8Array | string,
      documentTypeName: string,
      indexName: string,
      indexValuesList: Array<Uint8Array | string>,
      resultType: GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.ResultTypeMap[keyof GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.ResultTypeMap],
      allowIncludeLockedAndAbstainingVoteTally: boolean,
      startAtIdentifierInfo?: GetContestedResourceVoteStateRequest.GetContestedResourceVoteStateRequestV0.StartAtIdentifierInfo.AsObject,
      count: number,
      prove: boolean,
    }

    export class StartAtIdentifierInfo extends jspb.Message {
      getStartIdentifier(): Uint8Array | string;
      getStartIdentifier_asU8(): Uint8Array;
      getStartIdentifier_asB64(): string;
      setStartIdentifier(value: Uint8Array | string): void;

      getStartIdentifierIncluded(): boolean;
      setStartIdentifierIncluded(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): StartAtIdentifierInfo.AsObject;
      static toObject(includeInstance: boolean, msg: StartAtIdentifierInfo): StartAtIdentifierInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: StartAtIdentifierInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): StartAtIdentifierInfo;
      static deserializeBinaryFromReader(message: StartAtIdentifierInfo, reader: jspb.BinaryReader): StartAtIdentifierInfo;
    }

    export namespace StartAtIdentifierInfo {
      export type AsObject = {
        startIdentifier: Uint8Array | string,
        startIdentifierIncluded: boolean,
      }
    }

    export interface ResultTypeMap {
      DOCUMENTS: 0;
      VOTE_TALLY: 1;
      DOCUMENTS_AND_VOTE_TALLY: 2;
    }

    export const ResultType: ResultTypeMap;
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourceVoteStateResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0 | undefined;
  setV0(value?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0): void;

  getVersionCase(): GetContestedResourceVoteStateResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourceVoteStateResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourceVoteStateResponse): GetContestedResourceVoteStateResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourceVoteStateResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourceVoteStateResponse;
  static deserializeBinaryFromReader(message: GetContestedResourceVoteStateResponse, reader: jspb.BinaryReader): GetContestedResourceVoteStateResponse;
}

export namespace GetContestedResourceVoteStateResponse {
  export type AsObject = {
    v0?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.AsObject,
  }

  export class GetContestedResourceVoteStateResponseV0 extends jspb.Message {
    hasContestedResourceContenders(): boolean;
    clearContestedResourceContenders(): void;
    getContestedResourceContenders(): GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.ContestedResourceContenders | undefined;
    setContestedResourceContenders(value?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.ContestedResourceContenders): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetContestedResourceVoteStateResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourceVoteStateResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourceVoteStateResponseV0): GetContestedResourceVoteStateResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourceVoteStateResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourceVoteStateResponseV0;
    static deserializeBinaryFromReader(message: GetContestedResourceVoteStateResponseV0, reader: jspb.BinaryReader): GetContestedResourceVoteStateResponseV0;
  }

  export namespace GetContestedResourceVoteStateResponseV0 {
    export type AsObject = {
      contestedResourceContenders?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.ContestedResourceContenders.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class FinishedVoteInfo extends jspb.Message {
      getFinishedVoteOutcome(): GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.FinishedVoteOutcomeMap[keyof GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.FinishedVoteOutcomeMap];
      setFinishedVoteOutcome(value: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.FinishedVoteOutcomeMap[keyof GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.FinishedVoteOutcomeMap]): void;

      hasWonByIdentityId(): boolean;
      clearWonByIdentityId(): void;
      getWonByIdentityId(): Uint8Array | string;
      getWonByIdentityId_asU8(): Uint8Array;
      getWonByIdentityId_asB64(): string;
      setWonByIdentityId(value: Uint8Array | string): void;

      getFinishedAtBlockHeight(): number;
      setFinishedAtBlockHeight(value: number): void;

      getFinishedAtCoreBlockHeight(): number;
      setFinishedAtCoreBlockHeight(value: number): void;

      getFinishedAtBlockTimeMs(): number;
      setFinishedAtBlockTimeMs(value: number): void;

      getFinishedAtEpoch(): number;
      setFinishedAtEpoch(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): FinishedVoteInfo.AsObject;
      static toObject(includeInstance: boolean, msg: FinishedVoteInfo): FinishedVoteInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: FinishedVoteInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): FinishedVoteInfo;
      static deserializeBinaryFromReader(message: FinishedVoteInfo, reader: jspb.BinaryReader): FinishedVoteInfo;
    }

    export namespace FinishedVoteInfo {
      export type AsObject = {
        finishedVoteOutcome: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.FinishedVoteOutcomeMap[keyof GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.FinishedVoteOutcomeMap],
        wonByIdentityId: Uint8Array | string,
        finishedAtBlockHeight: number,
        finishedAtCoreBlockHeight: number,
        finishedAtBlockTimeMs: number,
        finishedAtEpoch: number,
      }

      export interface FinishedVoteOutcomeMap {
        TOWARDS_IDENTITY: 0;
        LOCKED: 1;
        NO_PREVIOUS_WINNER: 2;
      }

      export const FinishedVoteOutcome: FinishedVoteOutcomeMap;
    }

    export class ContestedResourceContenders extends jspb.Message {
      clearContendersList(): void;
      getContendersList(): Array<GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.Contender>;
      setContendersList(value: Array<GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.Contender>): void;
      addContenders(value?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.Contender, index?: number): GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.Contender;

      hasAbstainVoteTally(): boolean;
      clearAbstainVoteTally(): void;
      getAbstainVoteTally(): number;
      setAbstainVoteTally(value: number): void;

      hasLockVoteTally(): boolean;
      clearLockVoteTally(): void;
      getLockVoteTally(): number;
      setLockVoteTally(value: number): void;

      hasFinishedVoteInfo(): boolean;
      clearFinishedVoteInfo(): void;
      getFinishedVoteInfo(): GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo | undefined;
      setFinishedVoteInfo(value?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ContestedResourceContenders.AsObject;
      static toObject(includeInstance: boolean, msg: ContestedResourceContenders): ContestedResourceContenders.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ContestedResourceContenders, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ContestedResourceContenders;
      static deserializeBinaryFromReader(message: ContestedResourceContenders, reader: jspb.BinaryReader): ContestedResourceContenders;
    }

    export namespace ContestedResourceContenders {
      export type AsObject = {
        contendersList: Array<GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.Contender.AsObject>,
        abstainVoteTally: number,
        lockVoteTally: number,
        finishedVoteInfo?: GetContestedResourceVoteStateResponse.GetContestedResourceVoteStateResponseV0.FinishedVoteInfo.AsObject,
      }
    }

    export class Contender extends jspb.Message {
      getIdentifier(): Uint8Array | string;
      getIdentifier_asU8(): Uint8Array;
      getIdentifier_asB64(): string;
      setIdentifier(value: Uint8Array | string): void;

      hasVoteCount(): boolean;
      clearVoteCount(): void;
      getVoteCount(): number;
      setVoteCount(value: number): void;

      hasDocument(): boolean;
      clearDocument(): void;
      getDocument(): Uint8Array | string;
      getDocument_asU8(): Uint8Array;
      getDocument_asB64(): string;
      setDocument(value: Uint8Array | string): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Contender.AsObject;
      static toObject(includeInstance: boolean, msg: Contender): Contender.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Contender, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Contender;
      static deserializeBinaryFromReader(message: Contender, reader: jspb.BinaryReader): Contender;
    }

    export namespace Contender {
      export type AsObject = {
        identifier: Uint8Array | string,
        voteCount: number,
        document: Uint8Array | string,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      CONTESTED_RESOURCE_CONTENDERS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourceVotersForIdentityRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourceVotersForIdentityRequest.GetContestedResourceVotersForIdentityRequestV0 | undefined;
  setV0(value?: GetContestedResourceVotersForIdentityRequest.GetContestedResourceVotersForIdentityRequestV0): void;

  getVersionCase(): GetContestedResourceVotersForIdentityRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourceVotersForIdentityRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourceVotersForIdentityRequest): GetContestedResourceVotersForIdentityRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourceVotersForIdentityRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourceVotersForIdentityRequest;
  static deserializeBinaryFromReader(message: GetContestedResourceVotersForIdentityRequest, reader: jspb.BinaryReader): GetContestedResourceVotersForIdentityRequest;
}

export namespace GetContestedResourceVotersForIdentityRequest {
  export type AsObject = {
    v0?: GetContestedResourceVotersForIdentityRequest.GetContestedResourceVotersForIdentityRequestV0.AsObject,
  }

  export class GetContestedResourceVotersForIdentityRequestV0 extends jspb.Message {
    getContractId(): Uint8Array | string;
    getContractId_asU8(): Uint8Array;
    getContractId_asB64(): string;
    setContractId(value: Uint8Array | string): void;

    getDocumentTypeName(): string;
    setDocumentTypeName(value: string): void;

    getIndexName(): string;
    setIndexName(value: string): void;

    clearIndexValuesList(): void;
    getIndexValuesList(): Array<Uint8Array | string>;
    getIndexValuesList_asU8(): Array<Uint8Array>;
    getIndexValuesList_asB64(): Array<string>;
    setIndexValuesList(value: Array<Uint8Array | string>): void;
    addIndexValues(value: Uint8Array | string, index?: number): Uint8Array | string;

    getContestantId(): Uint8Array | string;
    getContestantId_asU8(): Uint8Array;
    getContestantId_asB64(): string;
    setContestantId(value: Uint8Array | string): void;

    hasStartAtIdentifierInfo(): boolean;
    clearStartAtIdentifierInfo(): void;
    getStartAtIdentifierInfo(): GetContestedResourceVotersForIdentityRequest.GetContestedResourceVotersForIdentityRequestV0.StartAtIdentifierInfo | undefined;
    setStartAtIdentifierInfo(value?: GetContestedResourceVotersForIdentityRequest.GetContestedResourceVotersForIdentityRequestV0.StartAtIdentifierInfo): void;

    hasCount(): boolean;
    clearCount(): void;
    getCount(): number;
    setCount(value: number): void;

    getOrderAscending(): boolean;
    setOrderAscending(value: boolean): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourceVotersForIdentityRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourceVotersForIdentityRequestV0): GetContestedResourceVotersForIdentityRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourceVotersForIdentityRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourceVotersForIdentityRequestV0;
    static deserializeBinaryFromReader(message: GetContestedResourceVotersForIdentityRequestV0, reader: jspb.BinaryReader): GetContestedResourceVotersForIdentityRequestV0;
  }

  export namespace GetContestedResourceVotersForIdentityRequestV0 {
    export type AsObject = {
      contractId: Uint8Array | string,
      documentTypeName: string,
      indexName: string,
      indexValuesList: Array<Uint8Array | string>,
      contestantId: Uint8Array | string,
      startAtIdentifierInfo?: GetContestedResourceVotersForIdentityRequest.GetContestedResourceVotersForIdentityRequestV0.StartAtIdentifierInfo.AsObject,
      count: number,
      orderAscending: boolean,
      prove: boolean,
    }

    export class StartAtIdentifierInfo extends jspb.Message {
      getStartIdentifier(): Uint8Array | string;
      getStartIdentifier_asU8(): Uint8Array;
      getStartIdentifier_asB64(): string;
      setStartIdentifier(value: Uint8Array | string): void;

      getStartIdentifierIncluded(): boolean;
      setStartIdentifierIncluded(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): StartAtIdentifierInfo.AsObject;
      static toObject(includeInstance: boolean, msg: StartAtIdentifierInfo): StartAtIdentifierInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: StartAtIdentifierInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): StartAtIdentifierInfo;
      static deserializeBinaryFromReader(message: StartAtIdentifierInfo, reader: jspb.BinaryReader): StartAtIdentifierInfo;
    }

    export namespace StartAtIdentifierInfo {
      export type AsObject = {
        startIdentifier: Uint8Array | string,
        startIdentifierIncluded: boolean,
      }
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourceVotersForIdentityResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourceVotersForIdentityResponse.GetContestedResourceVotersForIdentityResponseV0 | undefined;
  setV0(value?: GetContestedResourceVotersForIdentityResponse.GetContestedResourceVotersForIdentityResponseV0): void;

  getVersionCase(): GetContestedResourceVotersForIdentityResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourceVotersForIdentityResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourceVotersForIdentityResponse): GetContestedResourceVotersForIdentityResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourceVotersForIdentityResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourceVotersForIdentityResponse;
  static deserializeBinaryFromReader(message: GetContestedResourceVotersForIdentityResponse, reader: jspb.BinaryReader): GetContestedResourceVotersForIdentityResponse;
}

export namespace GetContestedResourceVotersForIdentityResponse {
  export type AsObject = {
    v0?: GetContestedResourceVotersForIdentityResponse.GetContestedResourceVotersForIdentityResponseV0.AsObject,
  }

  export class GetContestedResourceVotersForIdentityResponseV0 extends jspb.Message {
    hasContestedResourceVoters(): boolean;
    clearContestedResourceVoters(): void;
    getContestedResourceVoters(): GetContestedResourceVotersForIdentityResponse.GetContestedResourceVotersForIdentityResponseV0.ContestedResourceVoters | undefined;
    setContestedResourceVoters(value?: GetContestedResourceVotersForIdentityResponse.GetContestedResourceVotersForIdentityResponseV0.ContestedResourceVoters): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetContestedResourceVotersForIdentityResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourceVotersForIdentityResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourceVotersForIdentityResponseV0): GetContestedResourceVotersForIdentityResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourceVotersForIdentityResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourceVotersForIdentityResponseV0;
    static deserializeBinaryFromReader(message: GetContestedResourceVotersForIdentityResponseV0, reader: jspb.BinaryReader): GetContestedResourceVotersForIdentityResponseV0;
  }

  export namespace GetContestedResourceVotersForIdentityResponseV0 {
    export type AsObject = {
      contestedResourceVoters?: GetContestedResourceVotersForIdentityResponse.GetContestedResourceVotersForIdentityResponseV0.ContestedResourceVoters.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class ContestedResourceVoters extends jspb.Message {
      clearVotersList(): void;
      getVotersList(): Array<Uint8Array | string>;
      getVotersList_asU8(): Array<Uint8Array>;
      getVotersList_asB64(): Array<string>;
      setVotersList(value: Array<Uint8Array | string>): void;
      addVoters(value: Uint8Array | string, index?: number): Uint8Array | string;

      getFinishedResults(): boolean;
      setFinishedResults(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ContestedResourceVoters.AsObject;
      static toObject(includeInstance: boolean, msg: ContestedResourceVoters): ContestedResourceVoters.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ContestedResourceVoters, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ContestedResourceVoters;
      static deserializeBinaryFromReader(message: ContestedResourceVoters, reader: jspb.BinaryReader): ContestedResourceVoters;
    }

    export namespace ContestedResourceVoters {
      export type AsObject = {
        votersList: Array<Uint8Array | string>,
        finishedResults: boolean,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      CONTESTED_RESOURCE_VOTERS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourceIdentityVotesRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourceIdentityVotesRequest.GetContestedResourceIdentityVotesRequestV0 | undefined;
  setV0(value?: GetContestedResourceIdentityVotesRequest.GetContestedResourceIdentityVotesRequestV0): void;

  getVersionCase(): GetContestedResourceIdentityVotesRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourceIdentityVotesRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourceIdentityVotesRequest): GetContestedResourceIdentityVotesRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourceIdentityVotesRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourceIdentityVotesRequest;
  static deserializeBinaryFromReader(message: GetContestedResourceIdentityVotesRequest, reader: jspb.BinaryReader): GetContestedResourceIdentityVotesRequest;
}

export namespace GetContestedResourceIdentityVotesRequest {
  export type AsObject = {
    v0?: GetContestedResourceIdentityVotesRequest.GetContestedResourceIdentityVotesRequestV0.AsObject,
  }

  export class GetContestedResourceIdentityVotesRequestV0 extends jspb.Message {
    getIdentityId(): Uint8Array | string;
    getIdentityId_asU8(): Uint8Array;
    getIdentityId_asB64(): string;
    setIdentityId(value: Uint8Array | string): void;

    hasLimit(): boolean;
    clearLimit(): void;
    getLimit(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setLimit(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    hasOffset(): boolean;
    clearOffset(): void;
    getOffset(): google_protobuf_wrappers_pb.UInt32Value | undefined;
    setOffset(value?: google_protobuf_wrappers_pb.UInt32Value): void;

    getOrderAscending(): boolean;
    setOrderAscending(value: boolean): void;

    hasStartAtVotePollIdInfo(): boolean;
    clearStartAtVotePollIdInfo(): void;
    getStartAtVotePollIdInfo(): GetContestedResourceIdentityVotesRequest.GetContestedResourceIdentityVotesRequestV0.StartAtVotePollIdInfo | undefined;
    setStartAtVotePollIdInfo(value?: GetContestedResourceIdentityVotesRequest.GetContestedResourceIdentityVotesRequestV0.StartAtVotePollIdInfo): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourceIdentityVotesRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourceIdentityVotesRequestV0): GetContestedResourceIdentityVotesRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourceIdentityVotesRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourceIdentityVotesRequestV0;
    static deserializeBinaryFromReader(message: GetContestedResourceIdentityVotesRequestV0, reader: jspb.BinaryReader): GetContestedResourceIdentityVotesRequestV0;
  }

  export namespace GetContestedResourceIdentityVotesRequestV0 {
    export type AsObject = {
      identityId: Uint8Array | string,
      limit?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      offset?: google_protobuf_wrappers_pb.UInt32Value.AsObject,
      orderAscending: boolean,
      startAtVotePollIdInfo?: GetContestedResourceIdentityVotesRequest.GetContestedResourceIdentityVotesRequestV0.StartAtVotePollIdInfo.AsObject,
      prove: boolean,
    }

    export class StartAtVotePollIdInfo extends jspb.Message {
      getStartAtPollIdentifier(): Uint8Array | string;
      getStartAtPollIdentifier_asU8(): Uint8Array;
      getStartAtPollIdentifier_asB64(): string;
      setStartAtPollIdentifier(value: Uint8Array | string): void;

      getStartPollIdentifierIncluded(): boolean;
      setStartPollIdentifierIncluded(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): StartAtVotePollIdInfo.AsObject;
      static toObject(includeInstance: boolean, msg: StartAtVotePollIdInfo): StartAtVotePollIdInfo.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: StartAtVotePollIdInfo, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): StartAtVotePollIdInfo;
      static deserializeBinaryFromReader(message: StartAtVotePollIdInfo, reader: jspb.BinaryReader): StartAtVotePollIdInfo;
    }

    export namespace StartAtVotePollIdInfo {
      export type AsObject = {
        startAtPollIdentifier: Uint8Array | string,
        startPollIdentifierIncluded: boolean,
      }
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetContestedResourceIdentityVotesResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0 | undefined;
  setV0(value?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0): void;

  getVersionCase(): GetContestedResourceIdentityVotesResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetContestedResourceIdentityVotesResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetContestedResourceIdentityVotesResponse): GetContestedResourceIdentityVotesResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetContestedResourceIdentityVotesResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetContestedResourceIdentityVotesResponse;
  static deserializeBinaryFromReader(message: GetContestedResourceIdentityVotesResponse, reader: jspb.BinaryReader): GetContestedResourceIdentityVotesResponse;
}

export namespace GetContestedResourceIdentityVotesResponse {
  export type AsObject = {
    v0?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.AsObject,
  }

  export class GetContestedResourceIdentityVotesResponseV0 extends jspb.Message {
    hasVotes(): boolean;
    clearVotes(): void;
    getVotes(): GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVotes | undefined;
    setVotes(value?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVotes): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetContestedResourceIdentityVotesResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetContestedResourceIdentityVotesResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetContestedResourceIdentityVotesResponseV0): GetContestedResourceIdentityVotesResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetContestedResourceIdentityVotesResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetContestedResourceIdentityVotesResponseV0;
    static deserializeBinaryFromReader(message: GetContestedResourceIdentityVotesResponseV0, reader: jspb.BinaryReader): GetContestedResourceIdentityVotesResponseV0;
  }

  export namespace GetContestedResourceIdentityVotesResponseV0 {
    export type AsObject = {
      votes?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVotes.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class ContestedResourceIdentityVotes extends jspb.Message {
      clearContestedResourceIdentityVotesList(): void;
      getContestedResourceIdentityVotesList(): Array<GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVote>;
      setContestedResourceIdentityVotesList(value: Array<GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVote>): void;
      addContestedResourceIdentityVotes(value?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVote, index?: number): GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVote;

      getFinishedResults(): boolean;
      setFinishedResults(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ContestedResourceIdentityVotes.AsObject;
      static toObject(includeInstance: boolean, msg: ContestedResourceIdentityVotes): ContestedResourceIdentityVotes.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ContestedResourceIdentityVotes, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ContestedResourceIdentityVotes;
      static deserializeBinaryFromReader(message: ContestedResourceIdentityVotes, reader: jspb.BinaryReader): ContestedResourceIdentityVotes;
    }

    export namespace ContestedResourceIdentityVotes {
      export type AsObject = {
        contestedResourceIdentityVotesList: Array<GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ContestedResourceIdentityVote.AsObject>,
        finishedResults: boolean,
      }
    }

    export class ResourceVoteChoice extends jspb.Message {
      getVoteChoiceType(): GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.VoteChoiceTypeMap[keyof GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.VoteChoiceTypeMap];
      setVoteChoiceType(value: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.VoteChoiceTypeMap[keyof GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.VoteChoiceTypeMap]): void;

      hasIdentityId(): boolean;
      clearIdentityId(): void;
      getIdentityId(): Uint8Array | string;
      getIdentityId_asU8(): Uint8Array;
      getIdentityId_asB64(): string;
      setIdentityId(value: Uint8Array | string): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ResourceVoteChoice.AsObject;
      static toObject(includeInstance: boolean, msg: ResourceVoteChoice): ResourceVoteChoice.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ResourceVoteChoice, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ResourceVoteChoice;
      static deserializeBinaryFromReader(message: ResourceVoteChoice, reader: jspb.BinaryReader): ResourceVoteChoice;
    }

    export namespace ResourceVoteChoice {
      export type AsObject = {
        voteChoiceType: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.VoteChoiceTypeMap[keyof GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.VoteChoiceTypeMap],
        identityId: Uint8Array | string,
      }

      export interface VoteChoiceTypeMap {
        TOWARDS_IDENTITY: 0;
        ABSTAIN: 1;
        LOCK: 2;
      }

      export const VoteChoiceType: VoteChoiceTypeMap;
    }

    export class ContestedResourceIdentityVote extends jspb.Message {
      getContractId(): Uint8Array | string;
      getContractId_asU8(): Uint8Array;
      getContractId_asB64(): string;
      setContractId(value: Uint8Array | string): void;

      getDocumentTypeName(): string;
      setDocumentTypeName(value: string): void;

      clearSerializedIndexStorageValuesList(): void;
      getSerializedIndexStorageValuesList(): Array<Uint8Array | string>;
      getSerializedIndexStorageValuesList_asU8(): Array<Uint8Array>;
      getSerializedIndexStorageValuesList_asB64(): Array<string>;
      setSerializedIndexStorageValuesList(value: Array<Uint8Array | string>): void;
      addSerializedIndexStorageValues(value: Uint8Array | string, index?: number): Uint8Array | string;

      hasVoteChoice(): boolean;
      clearVoteChoice(): void;
      getVoteChoice(): GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice | undefined;
      setVoteChoice(value?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): ContestedResourceIdentityVote.AsObject;
      static toObject(includeInstance: boolean, msg: ContestedResourceIdentityVote): ContestedResourceIdentityVote.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: ContestedResourceIdentityVote, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): ContestedResourceIdentityVote;
      static deserializeBinaryFromReader(message: ContestedResourceIdentityVote, reader: jspb.BinaryReader): ContestedResourceIdentityVote;
    }

    export namespace ContestedResourceIdentityVote {
      export type AsObject = {
        contractId: Uint8Array | string,
        documentTypeName: string,
        serializedIndexStorageValuesList: Array<Uint8Array | string>,
        voteChoice?: GetContestedResourceIdentityVotesResponse.GetContestedResourceIdentityVotesResponseV0.ResourceVoteChoice.AsObject,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      VOTES = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetPrefundedSpecializedBalanceRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetPrefundedSpecializedBalanceRequest.GetPrefundedSpecializedBalanceRequestV0 | undefined;
  setV0(value?: GetPrefundedSpecializedBalanceRequest.GetPrefundedSpecializedBalanceRequestV0): void;

  getVersionCase(): GetPrefundedSpecializedBalanceRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetPrefundedSpecializedBalanceRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetPrefundedSpecializedBalanceRequest): GetPrefundedSpecializedBalanceRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetPrefundedSpecializedBalanceRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetPrefundedSpecializedBalanceRequest;
  static deserializeBinaryFromReader(message: GetPrefundedSpecializedBalanceRequest, reader: jspb.BinaryReader): GetPrefundedSpecializedBalanceRequest;
}

export namespace GetPrefundedSpecializedBalanceRequest {
  export type AsObject = {
    v0?: GetPrefundedSpecializedBalanceRequest.GetPrefundedSpecializedBalanceRequestV0.AsObject,
  }

  export class GetPrefundedSpecializedBalanceRequestV0 extends jspb.Message {
    getId(): Uint8Array | string;
    getId_asU8(): Uint8Array;
    getId_asB64(): string;
    setId(value: Uint8Array | string): void;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetPrefundedSpecializedBalanceRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetPrefundedSpecializedBalanceRequestV0): GetPrefundedSpecializedBalanceRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetPrefundedSpecializedBalanceRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetPrefundedSpecializedBalanceRequestV0;
    static deserializeBinaryFromReader(message: GetPrefundedSpecializedBalanceRequestV0, reader: jspb.BinaryReader): GetPrefundedSpecializedBalanceRequestV0;
  }

  export namespace GetPrefundedSpecializedBalanceRequestV0 {
    export type AsObject = {
      id: Uint8Array | string,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetPrefundedSpecializedBalanceResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetPrefundedSpecializedBalanceResponse.GetPrefundedSpecializedBalanceResponseV0 | undefined;
  setV0(value?: GetPrefundedSpecializedBalanceResponse.GetPrefundedSpecializedBalanceResponseV0): void;

  getVersionCase(): GetPrefundedSpecializedBalanceResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetPrefundedSpecializedBalanceResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetPrefundedSpecializedBalanceResponse): GetPrefundedSpecializedBalanceResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetPrefundedSpecializedBalanceResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetPrefundedSpecializedBalanceResponse;
  static deserializeBinaryFromReader(message: GetPrefundedSpecializedBalanceResponse, reader: jspb.BinaryReader): GetPrefundedSpecializedBalanceResponse;
}

export namespace GetPrefundedSpecializedBalanceResponse {
  export type AsObject = {
    v0?: GetPrefundedSpecializedBalanceResponse.GetPrefundedSpecializedBalanceResponseV0.AsObject,
  }

  export class GetPrefundedSpecializedBalanceResponseV0 extends jspb.Message {
    hasBalance(): boolean;
    clearBalance(): void;
    getBalance(): number;
    setBalance(value: number): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetPrefundedSpecializedBalanceResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetPrefundedSpecializedBalanceResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetPrefundedSpecializedBalanceResponseV0): GetPrefundedSpecializedBalanceResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetPrefundedSpecializedBalanceResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetPrefundedSpecializedBalanceResponseV0;
    static deserializeBinaryFromReader(message: GetPrefundedSpecializedBalanceResponseV0, reader: jspb.BinaryReader): GetPrefundedSpecializedBalanceResponseV0;
  }

  export namespace GetPrefundedSpecializedBalanceResponseV0 {
    export type AsObject = {
      balance: number,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      BALANCE = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetTotalCreditsInPlatformRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetTotalCreditsInPlatformRequest.GetTotalCreditsInPlatformRequestV0 | undefined;
  setV0(value?: GetTotalCreditsInPlatformRequest.GetTotalCreditsInPlatformRequestV0): void;

  getVersionCase(): GetTotalCreditsInPlatformRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetTotalCreditsInPlatformRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetTotalCreditsInPlatformRequest): GetTotalCreditsInPlatformRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetTotalCreditsInPlatformRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetTotalCreditsInPlatformRequest;
  static deserializeBinaryFromReader(message: GetTotalCreditsInPlatformRequest, reader: jspb.BinaryReader): GetTotalCreditsInPlatformRequest;
}

export namespace GetTotalCreditsInPlatformRequest {
  export type AsObject = {
    v0?: GetTotalCreditsInPlatformRequest.GetTotalCreditsInPlatformRequestV0.AsObject,
  }

  export class GetTotalCreditsInPlatformRequestV0 extends jspb.Message {
    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetTotalCreditsInPlatformRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetTotalCreditsInPlatformRequestV0): GetTotalCreditsInPlatformRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetTotalCreditsInPlatformRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetTotalCreditsInPlatformRequestV0;
    static deserializeBinaryFromReader(message: GetTotalCreditsInPlatformRequestV0, reader: jspb.BinaryReader): GetTotalCreditsInPlatformRequestV0;
  }

  export namespace GetTotalCreditsInPlatformRequestV0 {
    export type AsObject = {
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetTotalCreditsInPlatformResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetTotalCreditsInPlatformResponse.GetTotalCreditsInPlatformResponseV0 | undefined;
  setV0(value?: GetTotalCreditsInPlatformResponse.GetTotalCreditsInPlatformResponseV0): void;

  getVersionCase(): GetTotalCreditsInPlatformResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetTotalCreditsInPlatformResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetTotalCreditsInPlatformResponse): GetTotalCreditsInPlatformResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetTotalCreditsInPlatformResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetTotalCreditsInPlatformResponse;
  static deserializeBinaryFromReader(message: GetTotalCreditsInPlatformResponse, reader: jspb.BinaryReader): GetTotalCreditsInPlatformResponse;
}

export namespace GetTotalCreditsInPlatformResponse {
  export type AsObject = {
    v0?: GetTotalCreditsInPlatformResponse.GetTotalCreditsInPlatformResponseV0.AsObject,
  }

  export class GetTotalCreditsInPlatformResponseV0 extends jspb.Message {
    hasCredits(): boolean;
    clearCredits(): void;
    getCredits(): number;
    setCredits(value: number): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetTotalCreditsInPlatformResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetTotalCreditsInPlatformResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetTotalCreditsInPlatformResponseV0): GetTotalCreditsInPlatformResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetTotalCreditsInPlatformResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetTotalCreditsInPlatformResponseV0;
    static deserializeBinaryFromReader(message: GetTotalCreditsInPlatformResponseV0, reader: jspb.BinaryReader): GetTotalCreditsInPlatformResponseV0;
  }

  export namespace GetTotalCreditsInPlatformResponseV0 {
    export type AsObject = {
      credits: number,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      CREDITS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetPathElementsRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetPathElementsRequest.GetPathElementsRequestV0 | undefined;
  setV0(value?: GetPathElementsRequest.GetPathElementsRequestV0): void;

  getVersionCase(): GetPathElementsRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetPathElementsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetPathElementsRequest): GetPathElementsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetPathElementsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetPathElementsRequest;
  static deserializeBinaryFromReader(message: GetPathElementsRequest, reader: jspb.BinaryReader): GetPathElementsRequest;
}

export namespace GetPathElementsRequest {
  export type AsObject = {
    v0?: GetPathElementsRequest.GetPathElementsRequestV0.AsObject,
  }

  export class GetPathElementsRequestV0 extends jspb.Message {
    clearPathList(): void;
    getPathList(): Array<Uint8Array | string>;
    getPathList_asU8(): Array<Uint8Array>;
    getPathList_asB64(): Array<string>;
    setPathList(value: Array<Uint8Array | string>): void;
    addPath(value: Uint8Array | string, index?: number): Uint8Array | string;

    clearKeysList(): void;
    getKeysList(): Array<Uint8Array | string>;
    getKeysList_asU8(): Array<Uint8Array>;
    getKeysList_asB64(): Array<string>;
    setKeysList(value: Array<Uint8Array | string>): void;
    addKeys(value: Uint8Array | string, index?: number): Uint8Array | string;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetPathElementsRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetPathElementsRequestV0): GetPathElementsRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetPathElementsRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetPathElementsRequestV0;
    static deserializeBinaryFromReader(message: GetPathElementsRequestV0, reader: jspb.BinaryReader): GetPathElementsRequestV0;
  }

  export namespace GetPathElementsRequestV0 {
    export type AsObject = {
      pathList: Array<Uint8Array | string>,
      keysList: Array<Uint8Array | string>,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetPathElementsResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetPathElementsResponse.GetPathElementsResponseV0 | undefined;
  setV0(value?: GetPathElementsResponse.GetPathElementsResponseV0): void;

  getVersionCase(): GetPathElementsResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetPathElementsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetPathElementsResponse): GetPathElementsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetPathElementsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetPathElementsResponse;
  static deserializeBinaryFromReader(message: GetPathElementsResponse, reader: jspb.BinaryReader): GetPathElementsResponse;
}

export namespace GetPathElementsResponse {
  export type AsObject = {
    v0?: GetPathElementsResponse.GetPathElementsResponseV0.AsObject,
  }

  export class GetPathElementsResponseV0 extends jspb.Message {
    hasElements(): boolean;
    clearElements(): void;
    getElements(): GetPathElementsResponse.GetPathElementsResponseV0.Elements | undefined;
    setElements(value?: GetPathElementsResponse.GetPathElementsResponseV0.Elements): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetPathElementsResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetPathElementsResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetPathElementsResponseV0): GetPathElementsResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetPathElementsResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetPathElementsResponseV0;
    static deserializeBinaryFromReader(message: GetPathElementsResponseV0, reader: jspb.BinaryReader): GetPathElementsResponseV0;
  }

  export namespace GetPathElementsResponseV0 {
    export type AsObject = {
      elements?: GetPathElementsResponse.GetPathElementsResponseV0.Elements.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export class Elements extends jspb.Message {
      clearElementsList(): void;
      getElementsList(): Array<Uint8Array | string>;
      getElementsList_asU8(): Array<Uint8Array>;
      getElementsList_asB64(): Array<string>;
      setElementsList(value: Array<Uint8Array | string>): void;
      addElements(value: Uint8Array | string, index?: number): Uint8Array | string;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Elements.AsObject;
      static toObject(includeInstance: boolean, msg: Elements): Elements.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Elements, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Elements;
      static deserializeBinaryFromReader(message: Elements, reader: jspb.BinaryReader): Elements;
    }

    export namespace Elements {
      export type AsObject = {
        elementsList: Array<Uint8Array | string>,
      }
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      ELEMENTS = 1,
      PROOF = 2,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetStatusRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetStatusRequest.GetStatusRequestV0 | undefined;
  setV0(value?: GetStatusRequest.GetStatusRequestV0): void;

  getVersionCase(): GetStatusRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetStatusRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetStatusRequest): GetStatusRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetStatusRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetStatusRequest;
  static deserializeBinaryFromReader(message: GetStatusRequest, reader: jspb.BinaryReader): GetStatusRequest;
}

export namespace GetStatusRequest {
  export type AsObject = {
    v0?: GetStatusRequest.GetStatusRequestV0.AsObject,
  }

  export class GetStatusRequestV0 extends jspb.Message {
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetStatusRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetStatusRequestV0): GetStatusRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetStatusRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetStatusRequestV0;
    static deserializeBinaryFromReader(message: GetStatusRequestV0, reader: jspb.BinaryReader): GetStatusRequestV0;
  }

  export namespace GetStatusRequestV0 {
    export type AsObject = {
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetStatusResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetStatusResponse.GetStatusResponseV0 | undefined;
  setV0(value?: GetStatusResponse.GetStatusResponseV0): void;

  getVersionCase(): GetStatusResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetStatusResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetStatusResponse): GetStatusResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetStatusResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetStatusResponse;
  static deserializeBinaryFromReader(message: GetStatusResponse, reader: jspb.BinaryReader): GetStatusResponse;
}

export namespace GetStatusResponse {
  export type AsObject = {
    v0?: GetStatusResponse.GetStatusResponseV0.AsObject,
  }

  export class GetStatusResponseV0 extends jspb.Message {
    hasVersion(): boolean;
    clearVersion(): void;
    getVersion(): GetStatusResponse.GetStatusResponseV0.Version | undefined;
    setVersion(value?: GetStatusResponse.GetStatusResponseV0.Version): void;

    hasNode(): boolean;
    clearNode(): void;
    getNode(): GetStatusResponse.GetStatusResponseV0.Node | undefined;
    setNode(value?: GetStatusResponse.GetStatusResponseV0.Node): void;

    hasChain(): boolean;
    clearChain(): void;
    getChain(): GetStatusResponse.GetStatusResponseV0.Chain | undefined;
    setChain(value?: GetStatusResponse.GetStatusResponseV0.Chain): void;

    hasNetwork(): boolean;
    clearNetwork(): void;
    getNetwork(): GetStatusResponse.GetStatusResponseV0.Network | undefined;
    setNetwork(value?: GetStatusResponse.GetStatusResponseV0.Network): void;

    hasStateSync(): boolean;
    clearStateSync(): void;
    getStateSync(): GetStatusResponse.GetStatusResponseV0.StateSync | undefined;
    setStateSync(value?: GetStatusResponse.GetStatusResponseV0.StateSync): void;

    hasTime(): boolean;
    clearTime(): void;
    getTime(): GetStatusResponse.GetStatusResponseV0.Time | undefined;
    setTime(value?: GetStatusResponse.GetStatusResponseV0.Time): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetStatusResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetStatusResponseV0): GetStatusResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetStatusResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetStatusResponseV0;
    static deserializeBinaryFromReader(message: GetStatusResponseV0, reader: jspb.BinaryReader): GetStatusResponseV0;
  }

  export namespace GetStatusResponseV0 {
    export type AsObject = {
      version?: GetStatusResponse.GetStatusResponseV0.Version.AsObject,
      node?: GetStatusResponse.GetStatusResponseV0.Node.AsObject,
      chain?: GetStatusResponse.GetStatusResponseV0.Chain.AsObject,
      network?: GetStatusResponse.GetStatusResponseV0.Network.AsObject,
      stateSync?: GetStatusResponse.GetStatusResponseV0.StateSync.AsObject,
      time?: GetStatusResponse.GetStatusResponseV0.Time.AsObject,
    }

    export class Version extends jspb.Message {
      hasSoftware(): boolean;
      clearSoftware(): void;
      getSoftware(): GetStatusResponse.GetStatusResponseV0.Version.Software | undefined;
      setSoftware(value?: GetStatusResponse.GetStatusResponseV0.Version.Software): void;

      hasProtocol(): boolean;
      clearProtocol(): void;
      getProtocol(): GetStatusResponse.GetStatusResponseV0.Version.Protocol | undefined;
      setProtocol(value?: GetStatusResponse.GetStatusResponseV0.Version.Protocol): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Version.AsObject;
      static toObject(includeInstance: boolean, msg: Version): Version.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Version, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Version;
      static deserializeBinaryFromReader(message: Version, reader: jspb.BinaryReader): Version;
    }

    export namespace Version {
      export type AsObject = {
        software?: GetStatusResponse.GetStatusResponseV0.Version.Software.AsObject,
        protocol?: GetStatusResponse.GetStatusResponseV0.Version.Protocol.AsObject,
      }

      export class Software extends jspb.Message {
        getDapi(): string;
        setDapi(value: string): void;

        hasDrive(): boolean;
        clearDrive(): void;
        getDrive(): string;
        setDrive(value: string): void;

        hasTenderdash(): boolean;
        clearTenderdash(): void;
        getTenderdash(): string;
        setTenderdash(value: string): void;

        serializeBinary(): Uint8Array;
        toObject(includeInstance?: boolean): Software.AsObject;
        static toObject(includeInstance: boolean, msg: Software): Software.AsObject;
        static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
        static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
        static serializeBinaryToWriter(message: Software, writer: jspb.BinaryWriter): void;
        static deserializeBinary(bytes: Uint8Array): Software;
        static deserializeBinaryFromReader(message: Software, reader: jspb.BinaryReader): Software;
      }

      export namespace Software {
        export type AsObject = {
          dapi: string,
          drive: string,
          tenderdash: string,
        }
      }

      export class Protocol extends jspb.Message {
        hasTenderdash(): boolean;
        clearTenderdash(): void;
        getTenderdash(): GetStatusResponse.GetStatusResponseV0.Version.Protocol.Tenderdash | undefined;
        setTenderdash(value?: GetStatusResponse.GetStatusResponseV0.Version.Protocol.Tenderdash): void;

        hasDrive(): boolean;
        clearDrive(): void;
        getDrive(): GetStatusResponse.GetStatusResponseV0.Version.Protocol.Drive | undefined;
        setDrive(value?: GetStatusResponse.GetStatusResponseV0.Version.Protocol.Drive): void;

        serializeBinary(): Uint8Array;
        toObject(includeInstance?: boolean): Protocol.AsObject;
        static toObject(includeInstance: boolean, msg: Protocol): Protocol.AsObject;
        static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
        static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
        static serializeBinaryToWriter(message: Protocol, writer: jspb.BinaryWriter): void;
        static deserializeBinary(bytes: Uint8Array): Protocol;
        static deserializeBinaryFromReader(message: Protocol, reader: jspb.BinaryReader): Protocol;
      }

      export namespace Protocol {
        export type AsObject = {
          tenderdash?: GetStatusResponse.GetStatusResponseV0.Version.Protocol.Tenderdash.AsObject,
          drive?: GetStatusResponse.GetStatusResponseV0.Version.Protocol.Drive.AsObject,
        }

        export class Tenderdash extends jspb.Message {
          getP2p(): number;
          setP2p(value: number): void;

          getBlock(): number;
          setBlock(value: number): void;

          serializeBinary(): Uint8Array;
          toObject(includeInstance?: boolean): Tenderdash.AsObject;
          static toObject(includeInstance: boolean, msg: Tenderdash): Tenderdash.AsObject;
          static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
          static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
          static serializeBinaryToWriter(message: Tenderdash, writer: jspb.BinaryWriter): void;
          static deserializeBinary(bytes: Uint8Array): Tenderdash;
          static deserializeBinaryFromReader(message: Tenderdash, reader: jspb.BinaryReader): Tenderdash;
        }

        export namespace Tenderdash {
          export type AsObject = {
            p2p: number,
            block: number,
          }
        }

        export class Drive extends jspb.Message {
          getLatest(): number;
          setLatest(value: number): void;

          getCurrent(): number;
          setCurrent(value: number): void;

          serializeBinary(): Uint8Array;
          toObject(includeInstance?: boolean): Drive.AsObject;
          static toObject(includeInstance: boolean, msg: Drive): Drive.AsObject;
          static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
          static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
          static serializeBinaryToWriter(message: Drive, writer: jspb.BinaryWriter): void;
          static deserializeBinary(bytes: Uint8Array): Drive;
          static deserializeBinaryFromReader(message: Drive, reader: jspb.BinaryReader): Drive;
        }

        export namespace Drive {
          export type AsObject = {
            latest: number,
            current: number,
          }
        }
      }
    }

    export class Time extends jspb.Message {
      getLocal(): number;
      setLocal(value: number): void;

      hasBlock(): boolean;
      clearBlock(): void;
      getBlock(): number;
      setBlock(value: number): void;

      hasGenesis(): boolean;
      clearGenesis(): void;
      getGenesis(): number;
      setGenesis(value: number): void;

      hasEpoch(): boolean;
      clearEpoch(): void;
      getEpoch(): number;
      setEpoch(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Time.AsObject;
      static toObject(includeInstance: boolean, msg: Time): Time.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Time, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Time;
      static deserializeBinaryFromReader(message: Time, reader: jspb.BinaryReader): Time;
    }

    export namespace Time {
      export type AsObject = {
        local: number,
        block: number,
        genesis: number,
        epoch: number,
      }
    }

    export class Node extends jspb.Message {
      getId(): Uint8Array | string;
      getId_asU8(): Uint8Array;
      getId_asB64(): string;
      setId(value: Uint8Array | string): void;

      hasProTxHash(): boolean;
      clearProTxHash(): void;
      getProTxHash(): Uint8Array | string;
      getProTxHash_asU8(): Uint8Array;
      getProTxHash_asB64(): string;
      setProTxHash(value: Uint8Array | string): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Node.AsObject;
      static toObject(includeInstance: boolean, msg: Node): Node.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Node, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Node;
      static deserializeBinaryFromReader(message: Node, reader: jspb.BinaryReader): Node;
    }

    export namespace Node {
      export type AsObject = {
        id: Uint8Array | string,
        proTxHash: Uint8Array | string,
      }
    }

    export class Chain extends jspb.Message {
      getCatchingUp(): boolean;
      setCatchingUp(value: boolean): void;

      getLatestBlockHash(): Uint8Array | string;
      getLatestBlockHash_asU8(): Uint8Array;
      getLatestBlockHash_asB64(): string;
      setLatestBlockHash(value: Uint8Array | string): void;

      getLatestAppHash(): Uint8Array | string;
      getLatestAppHash_asU8(): Uint8Array;
      getLatestAppHash_asB64(): string;
      setLatestAppHash(value: Uint8Array | string): void;

      getLatestBlockHeight(): number;
      setLatestBlockHeight(value: number): void;

      getEarliestBlockHash(): Uint8Array | string;
      getEarliestBlockHash_asU8(): Uint8Array;
      getEarliestBlockHash_asB64(): string;
      setEarliestBlockHash(value: Uint8Array | string): void;

      getEarliestAppHash(): Uint8Array | string;
      getEarliestAppHash_asU8(): Uint8Array;
      getEarliestAppHash_asB64(): string;
      setEarliestAppHash(value: Uint8Array | string): void;

      getEarliestBlockHeight(): number;
      setEarliestBlockHeight(value: number): void;

      getMaxPeerBlockHeight(): number;
      setMaxPeerBlockHeight(value: number): void;

      hasCoreChainLockedHeight(): boolean;
      clearCoreChainLockedHeight(): void;
      getCoreChainLockedHeight(): number;
      setCoreChainLockedHeight(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Chain.AsObject;
      static toObject(includeInstance: boolean, msg: Chain): Chain.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Chain, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Chain;
      static deserializeBinaryFromReader(message: Chain, reader: jspb.BinaryReader): Chain;
    }

    export namespace Chain {
      export type AsObject = {
        catchingUp: boolean,
        latestBlockHash: Uint8Array | string,
        latestAppHash: Uint8Array | string,
        latestBlockHeight: number,
        earliestBlockHash: Uint8Array | string,
        earliestAppHash: Uint8Array | string,
        earliestBlockHeight: number,
        maxPeerBlockHeight: number,
        coreChainLockedHeight: number,
      }
    }

    export class Network extends jspb.Message {
      getChainId(): string;
      setChainId(value: string): void;

      getPeersCount(): number;
      setPeersCount(value: number): void;

      getListening(): boolean;
      setListening(value: boolean): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): Network.AsObject;
      static toObject(includeInstance: boolean, msg: Network): Network.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: Network, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): Network;
      static deserializeBinaryFromReader(message: Network, reader: jspb.BinaryReader): Network;
    }

    export namespace Network {
      export type AsObject = {
        chainId: string,
        peersCount: number,
        listening: boolean,
      }
    }

    export class StateSync extends jspb.Message {
      getTotalSyncedTime(): number;
      setTotalSyncedTime(value: number): void;

      getRemainingTime(): number;
      setRemainingTime(value: number): void;

      getTotalSnapshots(): number;
      setTotalSnapshots(value: number): void;

      getChunkProcessAvgTime(): number;
      setChunkProcessAvgTime(value: number): void;

      getSnapshotHeight(): number;
      setSnapshotHeight(value: number): void;

      getSnapshotChunksCount(): number;
      setSnapshotChunksCount(value: number): void;

      getBackfilledBlocks(): number;
      setBackfilledBlocks(value: number): void;

      getBackfillBlocksTotal(): number;
      setBackfillBlocksTotal(value: number): void;

      serializeBinary(): Uint8Array;
      toObject(includeInstance?: boolean): StateSync.AsObject;
      static toObject(includeInstance: boolean, msg: StateSync): StateSync.AsObject;
      static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
      static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
      static serializeBinaryToWriter(message: StateSync, writer: jspb.BinaryWriter): void;
      static deserializeBinary(bytes: Uint8Array): StateSync;
      static deserializeBinaryFromReader(message: StateSync, reader: jspb.BinaryReader): StateSync;
    }

    export namespace StateSync {
      export type AsObject = {
        totalSyncedTime: number,
        remainingTime: number,
        totalSnapshots: number,
        chunkProcessAvgTime: number,
        snapshotHeight: number,
        snapshotChunksCount: number,
        backfilledBlocks: number,
        backfillBlocksTotal: number,
      }
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetCurrentQuorumsInfoRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetCurrentQuorumsInfoRequest.GetCurrentQuorumsInfoRequestV0 | undefined;
  setV0(value?: GetCurrentQuorumsInfoRequest.GetCurrentQuorumsInfoRequestV0): void;

  getVersionCase(): GetCurrentQuorumsInfoRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetCurrentQuorumsInfoRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetCurrentQuorumsInfoRequest): GetCurrentQuorumsInfoRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetCurrentQuorumsInfoRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetCurrentQuorumsInfoRequest;
  static deserializeBinaryFromReader(message: GetCurrentQuorumsInfoRequest, reader: jspb.BinaryReader): GetCurrentQuorumsInfoRequest;
}

export namespace GetCurrentQuorumsInfoRequest {
  export type AsObject = {
    v0?: GetCurrentQuorumsInfoRequest.GetCurrentQuorumsInfoRequestV0.AsObject,
  }

  export class GetCurrentQuorumsInfoRequestV0 extends jspb.Message {
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetCurrentQuorumsInfoRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetCurrentQuorumsInfoRequestV0): GetCurrentQuorumsInfoRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetCurrentQuorumsInfoRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetCurrentQuorumsInfoRequestV0;
    static deserializeBinaryFromReader(message: GetCurrentQuorumsInfoRequestV0, reader: jspb.BinaryReader): GetCurrentQuorumsInfoRequestV0;
  }

  export namespace GetCurrentQuorumsInfoRequestV0 {
    export type AsObject = {
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetCurrentQuorumsInfoResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetCurrentQuorumsInfoResponse.GetCurrentQuorumsInfoResponseV0 | undefined;
  setV0(value?: GetCurrentQuorumsInfoResponse.GetCurrentQuorumsInfoResponseV0): void;

  getVersionCase(): GetCurrentQuorumsInfoResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetCurrentQuorumsInfoResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetCurrentQuorumsInfoResponse): GetCurrentQuorumsInfoResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetCurrentQuorumsInfoResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetCurrentQuorumsInfoResponse;
  static deserializeBinaryFromReader(message: GetCurrentQuorumsInfoResponse, reader: jspb.BinaryReader): GetCurrentQuorumsInfoResponse;
}

export namespace GetCurrentQuorumsInfoResponse {
  export type AsObject = {
    v0?: GetCurrentQuorumsInfoResponse.GetCurrentQuorumsInfoResponseV0.AsObject,
  }

  export class ValidatorV0 extends jspb.Message {
    getProTxHash(): Uint8Array | string;
    getProTxHash_asU8(): Uint8Array;
    getProTxHash_asB64(): string;
    setProTxHash(value: Uint8Array | string): void;

    getNodeIp(): string;
    setNodeIp(value: string): void;

    getIsBanned(): boolean;
    setIsBanned(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ValidatorV0.AsObject;
    static toObject(includeInstance: boolean, msg: ValidatorV0): ValidatorV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ValidatorV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ValidatorV0;
    static deserializeBinaryFromReader(message: ValidatorV0, reader: jspb.BinaryReader): ValidatorV0;
  }

  export namespace ValidatorV0 {
    export type AsObject = {
      proTxHash: Uint8Array | string,
      nodeIp: string,
      isBanned: boolean,
    }
  }

  export class ValidatorSetV0 extends jspb.Message {
    getQuorumHash(): Uint8Array | string;
    getQuorumHash_asU8(): Uint8Array;
    getQuorumHash_asB64(): string;
    setQuorumHash(value: Uint8Array | string): void;

    getCoreHeight(): number;
    setCoreHeight(value: number): void;

    clearMembersList(): void;
    getMembersList(): Array<GetCurrentQuorumsInfoResponse.ValidatorV0>;
    setMembersList(value: Array<GetCurrentQuorumsInfoResponse.ValidatorV0>): void;
    addMembers(value?: GetCurrentQuorumsInfoResponse.ValidatorV0, index?: number): GetCurrentQuorumsInfoResponse.ValidatorV0;

    getThresholdPublicKey(): Uint8Array | string;
    getThresholdPublicKey_asU8(): Uint8Array;
    getThresholdPublicKey_asB64(): string;
    setThresholdPublicKey(value: Uint8Array | string): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): ValidatorSetV0.AsObject;
    static toObject(includeInstance: boolean, msg: ValidatorSetV0): ValidatorSetV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: ValidatorSetV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): ValidatorSetV0;
    static deserializeBinaryFromReader(message: ValidatorSetV0, reader: jspb.BinaryReader): ValidatorSetV0;
  }

  export namespace ValidatorSetV0 {
    export type AsObject = {
      quorumHash: Uint8Array | string,
      coreHeight: number,
      membersList: Array<GetCurrentQuorumsInfoResponse.ValidatorV0.AsObject>,
      thresholdPublicKey: Uint8Array | string,
    }
  }

  export class GetCurrentQuorumsInfoResponseV0 extends jspb.Message {
    clearQuorumHashesList(): void;
    getQuorumHashesList(): Array<Uint8Array | string>;
    getQuorumHashesList_asU8(): Array<Uint8Array>;
    getQuorumHashesList_asB64(): Array<string>;
    setQuorumHashesList(value: Array<Uint8Array | string>): void;
    addQuorumHashes(value: Uint8Array | string, index?: number): Uint8Array | string;

    getCurrentQuorumHash(): Uint8Array | string;
    getCurrentQuorumHash_asU8(): Uint8Array;
    getCurrentQuorumHash_asB64(): string;
    setCurrentQuorumHash(value: Uint8Array | string): void;

    clearValidatorSetsList(): void;
    getValidatorSetsList(): Array<GetCurrentQuorumsInfoResponse.ValidatorSetV0>;
    setValidatorSetsList(value: Array<GetCurrentQuorumsInfoResponse.ValidatorSetV0>): void;
    addValidatorSets(value?: GetCurrentQuorumsInfoResponse.ValidatorSetV0, index?: number): GetCurrentQuorumsInfoResponse.ValidatorSetV0;

    getLastBlockProposer(): Uint8Array | string;
    getLastBlockProposer_asU8(): Uint8Array;
    getLastBlockProposer_asB64(): string;
    setLastBlockProposer(value: Uint8Array | string): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetCurrentQuorumsInfoResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetCurrentQuorumsInfoResponseV0): GetCurrentQuorumsInfoResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetCurrentQuorumsInfoResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetCurrentQuorumsInfoResponseV0;
    static deserializeBinaryFromReader(message: GetCurrentQuorumsInfoResponseV0, reader: jspb.BinaryReader): GetCurrentQuorumsInfoResponseV0;
  }

  export namespace GetCurrentQuorumsInfoResponseV0 {
    export type AsObject = {
      quorumHashesList: Array<Uint8Array | string>,
      currentQuorumHash: Uint8Array | string,
      validatorSetsList: Array<GetCurrentQuorumsInfoResponse.ValidatorSetV0.AsObject>,
      lastBlockProposer: Uint8Array | string,
      metadata?: ResponseMetadata.AsObject,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export interface KeyPurposeMap {
  AUTHENTICATION: 0;
  ENCRYPTION: 1;
  DECRYPTION: 2;
  TRANSFER: 3;
  VOTING: 5;
}

export const KeyPurpose: KeyPurposeMap;

