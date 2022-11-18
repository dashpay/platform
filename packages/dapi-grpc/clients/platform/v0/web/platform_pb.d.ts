// package: org.dash.platform.dapi.v0
// file: platform.proto

import * as jspb from "google-protobuf";

export class Proof extends jspb.Message {
  getMerkleProof(): Uint8Array | string;
  getMerkleProof_asU8(): Uint8Array;
  getMerkleProof_asB64(): string;
  setMerkleProof(value: Uint8Array | string): void;

  getSignatureLlmqHash(): Uint8Array | string;
  getSignatureLlmqHash_asU8(): Uint8Array;
  getSignatureLlmqHash_asB64(): string;
  setSignatureLlmqHash(value: Uint8Array | string): void;

  getSignature(): Uint8Array | string;
  getSignature_asU8(): Uint8Array;
  getSignature_asB64(): string;
  setSignature(value: Uint8Array | string): void;

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
    merkleProof: Uint8Array | string,
    signatureLlmqHash: Uint8Array | string,
    signature: Uint8Array | string,
  }
}

export class ResponseMetadata extends jspb.Message {
  getHeight(): number;
  setHeight(value: number): void;

  getCoreChainLockedHeight(): number;
  setCoreChainLockedHeight(value: number): void;

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
  getId(): Uint8Array | string;
  getId_asU8(): Uint8Array;
  getId_asB64(): string;
  setId(value: Uint8Array | string): void;

  getProve(): boolean;
  setProve(value: boolean): void;

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
    id: Uint8Array | string,
    prove: boolean,
  }
}

export class GetIdentityResponse extends jspb.Message {
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
    identity: Uint8Array | string,
    proof?: Proof.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }
}

export class GetDataContractRequest extends jspb.Message {
  getId(): Uint8Array | string;
  getId_asU8(): Uint8Array;
  getId_asB64(): string;
  setId(value: Uint8Array | string): void;

  getProve(): boolean;
  setProve(value: boolean): void;

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
    id: Uint8Array | string,
    prove: boolean,
  }
}

export class GetDataContractResponse extends jspb.Message {
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
    dataContract: Uint8Array | string,
    proof?: Proof.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }
}

export class GetDocumentsRequest extends jspb.Message {
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

  getStartCase(): GetDocumentsRequest.StartCase;
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

export class GetDocumentsResponse extends jspb.Message {
  clearDocumentsList(): void;
  getDocumentsList(): Array<Uint8Array | string>;
  getDocumentsList_asU8(): Array<Uint8Array>;
  getDocumentsList_asB64(): Array<string>;
  setDocumentsList(value: Array<Uint8Array | string>): void;
  addDocuments(value: Uint8Array | string, index?: number): Uint8Array | string;

  hasProof(): boolean;
  clearProof(): void;
  getProof(): Proof | undefined;
  setProof(value?: Proof): void;

  hasMetadata(): boolean;
  clearMetadata(): void;
  getMetadata(): ResponseMetadata | undefined;
  setMetadata(value?: ResponseMetadata): void;

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
    documentsList: Array<Uint8Array | string>,
    proof?: Proof.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }
}

export class GetIdentitiesByPublicKeyHashesRequest extends jspb.Message {
  clearPublicKeyHashesList(): void;
  getPublicKeyHashesList(): Array<Uint8Array | string>;
  getPublicKeyHashesList_asU8(): Array<Uint8Array>;
  getPublicKeyHashesList_asB64(): Array<string>;
  setPublicKeyHashesList(value: Array<Uint8Array | string>): void;
  addPublicKeyHashes(value: Uint8Array | string, index?: number): Uint8Array | string;

  getProve(): boolean;
  setProve(value: boolean): void;

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
    publicKeyHashesList: Array<Uint8Array | string>,
    prove: boolean,
  }
}

export class GetIdentitiesByPublicKeyHashesResponse extends jspb.Message {
  clearIdentitiesList(): void;
  getIdentitiesList(): Array<Uint8Array | string>;
  getIdentitiesList_asU8(): Array<Uint8Array>;
  getIdentitiesList_asB64(): Array<string>;
  setIdentitiesList(value: Array<Uint8Array | string>): void;
  addIdentities(value: Uint8Array | string, index?: number): Uint8Array | string;

  hasProof(): boolean;
  clearProof(): void;
  getProof(): Proof | undefined;
  setProof(value?: Proof): void;

  hasMetadata(): boolean;
  clearMetadata(): void;
  getMetadata(): ResponseMetadata | undefined;
  setMetadata(value?: ResponseMetadata): void;

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
    identitiesList: Array<Uint8Array | string>,
    proof?: Proof.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }
}

export class WaitForStateTransitionResultRequest extends jspb.Message {
  getStateTransitionHash(): Uint8Array | string;
  getStateTransitionHash_asU8(): Uint8Array;
  getStateTransitionHash_asB64(): string;
  setStateTransitionHash(value: Uint8Array | string): void;

  getProve(): boolean;
  setProve(value: boolean): void;

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
    stateTransitionHash: Uint8Array | string,
    prove: boolean,
  }
}

export class WaitForStateTransitionResultResponse extends jspb.Message {
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

  getResponsesCase(): WaitForStateTransitionResultResponse.ResponsesCase;
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
    error?: StateTransitionBroadcastError.AsObject,
    proof?: Proof.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }

  export enum ResponsesCase {
    RESPONSES_NOT_SET = 0,
    ERROR = 1,
    PROOF = 2,
  }
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

export class GetConsensusParamsRequest extends jspb.Message {
  getHeight(): number;
  setHeight(value: number): void;

  getProve(): boolean;
  setProve(value: boolean): void;

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
    height: number,
    prove: boolean,
  }
}

export class GetConsensusParamsResponse extends jspb.Message {
  hasBlock(): boolean;
  clearBlock(): void;
  getBlock(): ConsensusParamsBlock | undefined;
  setBlock(value?: ConsensusParamsBlock): void;

  hasEvidence(): boolean;
  clearEvidence(): void;
  getEvidence(): ConsensusParamsEvidence | undefined;
  setEvidence(value?: ConsensusParamsEvidence): void;

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
    block?: ConsensusParamsBlock.AsObject,
    evidence?: ConsensusParamsEvidence.AsObject,
  }
}

