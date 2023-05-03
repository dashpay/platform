// package: org.dash.platform.dapi.v0
// file: platform.proto

import * as jspb from "google-protobuf";
import * as google_protobuf_timestamp_pb from "google-protobuf/google/protobuf/timestamp_pb";

export class ProvedResult extends jspb.Message {
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

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): ProvedResult.AsObject;
  static toObject(includeInstance: boolean, msg: ProvedResult): ProvedResult.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: ProvedResult, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): ProvedResult;
  static deserializeBinaryFromReader(message: ProvedResult, reader: jspb.BinaryReader): ProvedResult;
}

export namespace ProvedResult {
  export type AsObject = {
    grovedbProof: Uint8Array | string,
    quorumHash: Uint8Array | string,
    signature: Uint8Array | string,
    round: number,
  }
}

export class ResponseMetadata extends jspb.Message {
  getHeight(): number;
  setHeight(value: number): void;

  getCoreChainLockedHeight(): number;
  setCoreChainLockedHeight(value: number): void;

  getTimeMs(): number;
  setTimeMs(value: number): void;

  getProtocolVersion(): number;
  setProtocolVersion(value: number): void;

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
    timeMs: number,
    protocolVersion: number,
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

export class SingleItemResponse extends jspb.Message {
  hasNonProvedResult(): boolean;
  clearNonProvedResult(): void;
  getNonProvedResult(): Uint8Array | string;
  getNonProvedResult_asU8(): Uint8Array;
  getNonProvedResult_asB64(): string;
  setNonProvedResult(value: Uint8Array | string): void;

  hasProvedResult(): boolean;
  clearProvedResult(): void;
  getProvedResult(): ProvedResult | undefined;
  setProvedResult(value?: ProvedResult): void;

  hasMetadata(): boolean;
  clearMetadata(): void;
  getMetadata(): ResponseMetadata | undefined;
  setMetadata(value?: ResponseMetadata): void;

  getResultCase(): SingleItemResponse.ResultCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): SingleItemResponse.AsObject;
  static toObject(includeInstance: boolean, msg: SingleItemResponse): SingleItemResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: SingleItemResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): SingleItemResponse;
  static deserializeBinaryFromReader(message: SingleItemResponse, reader: jspb.BinaryReader): SingleItemResponse;
}

export namespace SingleItemResponse {
  export type AsObject = {
    nonProvedResult: Uint8Array | string,
    provedResult?: ProvedResult.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }

  export enum ResultCase {
    RESULT_NOT_SET = 0,
    NON_PROVED_RESULT = 1,
    PROVED_RESULT = 2,
  }
}

export class ResultList extends jspb.Message {
  clearItemsList(): void;
  getItemsList(): Array<Uint8Array | string>;
  getItemsList_asU8(): Array<Uint8Array>;
  getItemsList_asB64(): Array<string>;
  setItemsList(value: Array<Uint8Array | string>): void;
  addItems(value: Uint8Array | string, index?: number): Uint8Array | string;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): ResultList.AsObject;
  static toObject(includeInstance: boolean, msg: ResultList): ResultList.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: ResultList, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): ResultList;
  static deserializeBinaryFromReader(message: ResultList, reader: jspb.BinaryReader): ResultList;
}

export namespace ResultList {
  export type AsObject = {
    itemsList: Array<Uint8Array | string>,
  }
}

export class MultiItemResponse extends jspb.Message {
  hasNonProvedResults(): boolean;
  clearNonProvedResults(): void;
  getNonProvedResults(): ResultList | undefined;
  setNonProvedResults(value?: ResultList): void;

  hasProvedResult(): boolean;
  clearProvedResult(): void;
  getProvedResult(): ProvedResult | undefined;
  setProvedResult(value?: ProvedResult): void;

  hasMetadata(): boolean;
  clearMetadata(): void;
  getMetadata(): ResponseMetadata | undefined;
  setMetadata(value?: ResponseMetadata): void;

  getResultCase(): MultiItemResponse.ResultCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): MultiItemResponse.AsObject;
  static toObject(includeInstance: boolean, msg: MultiItemResponse): MultiItemResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: MultiItemResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): MultiItemResponse;
  static deserializeBinaryFromReader(message: MultiItemResponse, reader: jspb.BinaryReader): MultiItemResponse;
}

export namespace MultiItemResponse {
  export type AsObject = {
    nonProvedResults?: ResultList.AsObject,
    provedResult?: ProvedResult.AsObject,
    metadata?: ResponseMetadata.AsObject,
  }

  export enum ResultCase {
    RESULT_NOT_SET = 0,
    NON_PROVED_RESULTS = 1,
    PROVED_RESULT = 2,
  }
}

export class GetSingleItemRequest extends jspb.Message {
  getId(): Uint8Array | string;
  getId_asU8(): Uint8Array;
  getId_asB64(): string;
  setId(value: Uint8Array | string): void;

  getProve(): boolean;
  setProve(value: boolean): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetSingleItemRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetSingleItemRequest): GetSingleItemRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetSingleItemRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetSingleItemRequest;
  static deserializeBinaryFromReader(message: GetSingleItemRequest, reader: jspb.BinaryReader): GetSingleItemRequest;
}

export namespace GetSingleItemRequest {
  export type AsObject = {
    id: Uint8Array | string,
    prove: boolean,
  }
}

export class GetMultiItemRequest extends jspb.Message {
  clearIdsList(): void;
  getIdsList(): Array<Uint8Array | string>;
  getIdsList_asU8(): Array<Uint8Array>;
  getIdsList_asB64(): Array<string>;
  setIdsList(value: Array<Uint8Array | string>): void;
  addIds(value: Uint8Array | string, index?: number): Uint8Array | string;

  getProve(): boolean;
  setProve(value: boolean): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetMultiItemRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetMultiItemRequest): GetMultiItemRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetMultiItemRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetMultiItemRequest;
  static deserializeBinaryFromReader(message: GetMultiItemRequest, reader: jspb.BinaryReader): GetMultiItemRequest;
}

export namespace GetMultiItemRequest {
  export type AsObject = {
    idsList: Array<Uint8Array | string>,
    prove: boolean,
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
  getProof(): ProvedResult | undefined;
  setProof(value?: ProvedResult): void;

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
    proof?: ProvedResult.AsObject,
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

