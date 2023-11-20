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

export class GetIdentitiesRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesRequest.GetIdentitiesRequestV0 | undefined;
  setV0(value?: GetIdentitiesRequest.GetIdentitiesRequestV0): void;

  getVersionCase(): GetIdentitiesRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesRequest): GetIdentitiesRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesRequest;
  static deserializeBinaryFromReader(message: GetIdentitiesRequest, reader: jspb.BinaryReader): GetIdentitiesRequest;
}

export namespace GetIdentitiesRequest {
  export type AsObject = {
    v0?: GetIdentitiesRequest.GetIdentitiesRequestV0.AsObject,
  }

  export class GetIdentitiesRequestV0 extends jspb.Message {
    clearIdsList(): void;
    getIdsList(): Array<Uint8Array | string>;
    getIdsList_asU8(): Array<Uint8Array>;
    getIdsList_asB64(): Array<string>;
    setIdsList(value: Array<Uint8Array | string>): void;
    addIds(value: Uint8Array | string, index?: number): Uint8Array | string;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesRequestV0): GetIdentitiesRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesRequestV0;
    static deserializeBinaryFromReader(message: GetIdentitiesRequestV0, reader: jspb.BinaryReader): GetIdentitiesRequestV0;
  }

  export namespace GetIdentitiesRequestV0 {
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

export class GetIdentitiesResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesResponse.GetIdentitiesResponseV0 | undefined;
  setV0(value?: GetIdentitiesResponse.GetIdentitiesResponseV0): void;

  getVersionCase(): GetIdentitiesResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesResponse): GetIdentitiesResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesResponse;
  static deserializeBinaryFromReader(message: GetIdentitiesResponse, reader: jspb.BinaryReader): GetIdentitiesResponse;
}

export namespace GetIdentitiesResponse {
  export type AsObject = {
    v0?: GetIdentitiesResponse.GetIdentitiesResponseV0.AsObject,
  }

  export class IdentityValue extends jspb.Message {
    getValue(): Uint8Array | string;
    getValue_asU8(): Uint8Array;
    getValue_asB64(): string;
    setValue(value: Uint8Array | string): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): IdentityValue.AsObject;
    static toObject(includeInstance: boolean, msg: IdentityValue): IdentityValue.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: IdentityValue, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): IdentityValue;
    static deserializeBinaryFromReader(message: IdentityValue, reader: jspb.BinaryReader): IdentityValue;
  }

  export namespace IdentityValue {
    export type AsObject = {
      value: Uint8Array | string,
    }
  }

  export class IdentityEntry extends jspb.Message {
    getKey(): Uint8Array | string;
    getKey_asU8(): Uint8Array;
    getKey_asB64(): string;
    setKey(value: Uint8Array | string): void;

    hasValue(): boolean;
    clearValue(): void;
    getValue(): GetIdentitiesResponse.IdentityValue | undefined;
    setValue(value?: GetIdentitiesResponse.IdentityValue): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): IdentityEntry.AsObject;
    static toObject(includeInstance: boolean, msg: IdentityEntry): IdentityEntry.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: IdentityEntry, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): IdentityEntry;
    static deserializeBinaryFromReader(message: IdentityEntry, reader: jspb.BinaryReader): IdentityEntry;
  }

  export namespace IdentityEntry {
    export type AsObject = {
      key: Uint8Array | string,
      value?: GetIdentitiesResponse.IdentityValue.AsObject,
    }
  }

  export class Identities extends jspb.Message {
    clearIdentityEntriesList(): void;
    getIdentityEntriesList(): Array<GetIdentitiesResponse.IdentityEntry>;
    setIdentityEntriesList(value: Array<GetIdentitiesResponse.IdentityEntry>): void;
    addIdentityEntries(value?: GetIdentitiesResponse.IdentityEntry, index?: number): GetIdentitiesResponse.IdentityEntry;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): Identities.AsObject;
    static toObject(includeInstance: boolean, msg: Identities): Identities.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: Identities, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): Identities;
    static deserializeBinaryFromReader(message: Identities, reader: jspb.BinaryReader): Identities;
  }

  export namespace Identities {
    export type AsObject = {
      identityEntriesList: Array<GetIdentitiesResponse.IdentityEntry.AsObject>,
    }
  }

  export class GetIdentitiesResponseV0 extends jspb.Message {
    hasIdentities(): boolean;
    clearIdentities(): void;
    getIdentities(): GetIdentitiesResponse.Identities | undefined;
    setIdentities(value?: GetIdentitiesResponse.Identities): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentitiesResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesResponseV0): GetIdentitiesResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesResponseV0;
    static deserializeBinaryFromReader(message: GetIdentitiesResponseV0, reader: jspb.BinaryReader): GetIdentitiesResponseV0;
  }

  export namespace GetIdentitiesResponseV0 {
    export type AsObject = {
      identities?: GetIdentitiesResponse.Identities.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITIES = 1,
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
      }
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

export class GetIdentitiesByPublicKeyHashesRequest extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0 | undefined;
  setV0(value?: GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0): void;

  getVersionCase(): GetIdentitiesByPublicKeyHashesRequest.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesByPublicKeyHashesRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesByPublicKeyHashesRequest): GetIdentitiesByPublicKeyHashesRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesByPublicKeyHashesRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesByPublicKeyHashesRequest;
  static deserializeBinaryFromReader(message: GetIdentitiesByPublicKeyHashesRequest, reader: jspb.BinaryReader): GetIdentitiesByPublicKeyHashesRequest;
}

export namespace GetIdentitiesByPublicKeyHashesRequest {
  export type AsObject = {
    v0?: GetIdentitiesByPublicKeyHashesRequest.GetIdentitiesByPublicKeyHashesRequestV0.AsObject,
  }

  export class GetIdentitiesByPublicKeyHashesRequestV0 extends jspb.Message {
    clearPublicKeyHashesList(): void;
    getPublicKeyHashesList(): Array<Uint8Array | string>;
    getPublicKeyHashesList_asU8(): Array<Uint8Array>;
    getPublicKeyHashesList_asB64(): Array<string>;
    setPublicKeyHashesList(value: Array<Uint8Array | string>): void;
    addPublicKeyHashes(value: Uint8Array | string, index?: number): Uint8Array | string;

    getProve(): boolean;
    setProve(value: boolean): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesByPublicKeyHashesRequestV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesByPublicKeyHashesRequestV0): GetIdentitiesByPublicKeyHashesRequestV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesByPublicKeyHashesRequestV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesByPublicKeyHashesRequestV0;
    static deserializeBinaryFromReader(message: GetIdentitiesByPublicKeyHashesRequestV0, reader: jspb.BinaryReader): GetIdentitiesByPublicKeyHashesRequestV0;
  }

  export namespace GetIdentitiesByPublicKeyHashesRequestV0 {
    export type AsObject = {
      publicKeyHashesList: Array<Uint8Array | string>,
      prove: boolean,
    }
  }

  export enum VersionCase {
    VERSION_NOT_SET = 0,
    V0 = 1,
  }
}

export class GetIdentitiesByPublicKeyHashesResponse extends jspb.Message {
  hasV0(): boolean;
  clearV0(): void;
  getV0(): GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0 | undefined;
  setV0(value?: GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0): void;

  getVersionCase(): GetIdentitiesByPublicKeyHashesResponse.VersionCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetIdentitiesByPublicKeyHashesResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetIdentitiesByPublicKeyHashesResponse): GetIdentitiesByPublicKeyHashesResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetIdentitiesByPublicKeyHashesResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetIdentitiesByPublicKeyHashesResponse;
  static deserializeBinaryFromReader(message: GetIdentitiesByPublicKeyHashesResponse, reader: jspb.BinaryReader): GetIdentitiesByPublicKeyHashesResponse;
}

export namespace GetIdentitiesByPublicKeyHashesResponse {
  export type AsObject = {
    v0?: GetIdentitiesByPublicKeyHashesResponse.GetIdentitiesByPublicKeyHashesResponseV0.AsObject,
  }

  export class PublicKeyHashIdentityEntry extends jspb.Message {
    getPublicKeyHash(): Uint8Array | string;
    getPublicKeyHash_asU8(): Uint8Array;
    getPublicKeyHash_asB64(): string;
    setPublicKeyHash(value: Uint8Array | string): void;

    hasValue(): boolean;
    clearValue(): void;
    getValue(): google_protobuf_wrappers_pb.BytesValue | undefined;
    setValue(value?: google_protobuf_wrappers_pb.BytesValue): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): PublicKeyHashIdentityEntry.AsObject;
    static toObject(includeInstance: boolean, msg: PublicKeyHashIdentityEntry): PublicKeyHashIdentityEntry.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: PublicKeyHashIdentityEntry, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): PublicKeyHashIdentityEntry;
    static deserializeBinaryFromReader(message: PublicKeyHashIdentityEntry, reader: jspb.BinaryReader): PublicKeyHashIdentityEntry;
  }

  export namespace PublicKeyHashIdentityEntry {
    export type AsObject = {
      publicKeyHash: Uint8Array | string,
      value?: google_protobuf_wrappers_pb.BytesValue.AsObject,
    }
  }

  export class IdentitiesByPublicKeyHashes extends jspb.Message {
    clearIdentityEntriesList(): void;
    getIdentityEntriesList(): Array<GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry>;
    setIdentityEntriesList(value: Array<GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry>): void;
    addIdentityEntries(value?: GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry, index?: number): GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): IdentitiesByPublicKeyHashes.AsObject;
    static toObject(includeInstance: boolean, msg: IdentitiesByPublicKeyHashes): IdentitiesByPublicKeyHashes.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: IdentitiesByPublicKeyHashes, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): IdentitiesByPublicKeyHashes;
    static deserializeBinaryFromReader(message: IdentitiesByPublicKeyHashes, reader: jspb.BinaryReader): IdentitiesByPublicKeyHashes;
  }

  export namespace IdentitiesByPublicKeyHashes {
    export type AsObject = {
      identityEntriesList: Array<GetIdentitiesByPublicKeyHashesResponse.PublicKeyHashIdentityEntry.AsObject>,
    }
  }

  export class GetIdentitiesByPublicKeyHashesResponseV0 extends jspb.Message {
    hasIdentities(): boolean;
    clearIdentities(): void;
    getIdentities(): GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes | undefined;
    setIdentities(value?: GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes): void;

    hasProof(): boolean;
    clearProof(): void;
    getProof(): Proof | undefined;
    setProof(value?: Proof): void;

    hasMetadata(): boolean;
    clearMetadata(): void;
    getMetadata(): ResponseMetadata | undefined;
    setMetadata(value?: ResponseMetadata): void;

    getResultCase(): GetIdentitiesByPublicKeyHashesResponseV0.ResultCase;
    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): GetIdentitiesByPublicKeyHashesResponseV0.AsObject;
    static toObject(includeInstance: boolean, msg: GetIdentitiesByPublicKeyHashesResponseV0): GetIdentitiesByPublicKeyHashesResponseV0.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: GetIdentitiesByPublicKeyHashesResponseV0, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): GetIdentitiesByPublicKeyHashesResponseV0;
    static deserializeBinaryFromReader(message: GetIdentitiesByPublicKeyHashesResponseV0, reader: jspb.BinaryReader): GetIdentitiesByPublicKeyHashesResponseV0;
  }

  export namespace GetIdentitiesByPublicKeyHashesResponseV0 {
    export type AsObject = {
      identities?: GetIdentitiesByPublicKeyHashesResponse.IdentitiesByPublicKeyHashes.AsObject,
      proof?: Proof.AsObject,
      metadata?: ResponseMetadata.AsObject,
    }

    export enum ResultCase {
      RESULT_NOT_SET = 0,
      IDENTITIES = 1,
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

