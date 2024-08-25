// package: org.dash.platform.dapi.v0
// file: core.proto

import * as jspb from "google-protobuf";

export class GetBlockchainStatusRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetBlockchainStatusRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetBlockchainStatusRequest): GetBlockchainStatusRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetBlockchainStatusRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetBlockchainStatusRequest;
  static deserializeBinaryFromReader(message: GetBlockchainStatusRequest, reader: jspb.BinaryReader): GetBlockchainStatusRequest;
}

export namespace GetBlockchainStatusRequest {
  export type AsObject = {
  }
}

export class GetBlockchainStatusResponse extends jspb.Message {
  hasVersion(): boolean;
  clearVersion(): void;
  getVersion(): GetBlockchainStatusResponse.Version | undefined;
  setVersion(value?: GetBlockchainStatusResponse.Version): void;

  hasTime(): boolean;
  clearTime(): void;
  getTime(): GetBlockchainStatusResponse.Time | undefined;
  setTime(value?: GetBlockchainStatusResponse.Time): void;

  getStatus(): GetBlockchainStatusResponse.StatusMap[keyof GetBlockchainStatusResponse.StatusMap];
  setStatus(value: GetBlockchainStatusResponse.StatusMap[keyof GetBlockchainStatusResponse.StatusMap]): void;

  getSyncProgress(): number;
  setSyncProgress(value: number): void;

  hasChain(): boolean;
  clearChain(): void;
  getChain(): GetBlockchainStatusResponse.Chain | undefined;
  setChain(value?: GetBlockchainStatusResponse.Chain): void;

  hasNetwork(): boolean;
  clearNetwork(): void;
  getNetwork(): GetBlockchainStatusResponse.Network | undefined;
  setNetwork(value?: GetBlockchainStatusResponse.Network): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetBlockchainStatusResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetBlockchainStatusResponse): GetBlockchainStatusResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetBlockchainStatusResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetBlockchainStatusResponse;
  static deserializeBinaryFromReader(message: GetBlockchainStatusResponse, reader: jspb.BinaryReader): GetBlockchainStatusResponse;
}

export namespace GetBlockchainStatusResponse {
  export type AsObject = {
    version?: GetBlockchainStatusResponse.Version.AsObject,
    time?: GetBlockchainStatusResponse.Time.AsObject,
    status: GetBlockchainStatusResponse.StatusMap[keyof GetBlockchainStatusResponse.StatusMap],
    syncProgress: number,
    chain?: GetBlockchainStatusResponse.Chain.AsObject,
    network?: GetBlockchainStatusResponse.Network.AsObject,
  }

  export class Version extends jspb.Message {
    getProtocol(): number;
    setProtocol(value: number): void;

    getSoftware(): number;
    setSoftware(value: number): void;

    getAgent(): string;
    setAgent(value: string): void;

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
      protocol: number,
      software: number,
      agent: string,
    }
  }

  export class Time extends jspb.Message {
    getNow(): number;
    setNow(value: number): void;

    getOffset(): number;
    setOffset(value: number): void;

    getMedian(): number;
    setMedian(value: number): void;

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
      now: number,
      offset: number,
      median: number,
    }
  }

  export class Chain extends jspb.Message {
    getName(): string;
    setName(value: string): void;

    getHeadersCount(): number;
    setHeadersCount(value: number): void;

    getBlocksCount(): number;
    setBlocksCount(value: number): void;

    getBestBlockHash(): Uint8Array | string;
    getBestBlockHash_asU8(): Uint8Array;
    getBestBlockHash_asB64(): string;
    setBestBlockHash(value: Uint8Array | string): void;

    getDifficulty(): number;
    setDifficulty(value: number): void;

    getChainWork(): Uint8Array | string;
    getChainWork_asU8(): Uint8Array;
    getChainWork_asB64(): string;
    setChainWork(value: Uint8Array | string): void;

    getIsSynced(): boolean;
    setIsSynced(value: boolean): void;

    getSyncProgress(): number;
    setSyncProgress(value: number): void;

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
      name: string,
      headersCount: number,
      blocksCount: number,
      bestBlockHash: Uint8Array | string,
      difficulty: number,
      chainWork: Uint8Array | string,
      isSynced: boolean,
      syncProgress: number,
    }
  }

  export class NetworkFee extends jspb.Message {
    getRelay(): number;
    setRelay(value: number): void;

    getIncremental(): number;
    setIncremental(value: number): void;

    serializeBinary(): Uint8Array;
    toObject(includeInstance?: boolean): NetworkFee.AsObject;
    static toObject(includeInstance: boolean, msg: NetworkFee): NetworkFee.AsObject;
    static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
    static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
    static serializeBinaryToWriter(message: NetworkFee, writer: jspb.BinaryWriter): void;
    static deserializeBinary(bytes: Uint8Array): NetworkFee;
    static deserializeBinaryFromReader(message: NetworkFee, reader: jspb.BinaryReader): NetworkFee;
  }

  export namespace NetworkFee {
    export type AsObject = {
      relay: number,
      incremental: number,
    }
  }

  export class Network extends jspb.Message {
    getPeersCount(): number;
    setPeersCount(value: number): void;

    hasFee(): boolean;
    clearFee(): void;
    getFee(): GetBlockchainStatusResponse.NetworkFee | undefined;
    setFee(value?: GetBlockchainStatusResponse.NetworkFee): void;

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
      peersCount: number,
      fee?: GetBlockchainStatusResponse.NetworkFee.AsObject,
    }
  }

  export interface StatusMap {
    NOT_STARTED: 0;
    SYNCING: 1;
    READY: 2;
    ERROR: 3;
  }

  export const Status: StatusMap;
}

export class GetMasternodeStatusRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetMasternodeStatusRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetMasternodeStatusRequest): GetMasternodeStatusRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetMasternodeStatusRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetMasternodeStatusRequest;
  static deserializeBinaryFromReader(message: GetMasternodeStatusRequest, reader: jspb.BinaryReader): GetMasternodeStatusRequest;
}

export namespace GetMasternodeStatusRequest {
  export type AsObject = {
  }
}

export class GetMasternodeStatusResponse extends jspb.Message {
  getStatus(): GetMasternodeStatusResponse.StatusMap[keyof GetMasternodeStatusResponse.StatusMap];
  setStatus(value: GetMasternodeStatusResponse.StatusMap[keyof GetMasternodeStatusResponse.StatusMap]): void;

  getProTxHash(): Uint8Array | string;
  getProTxHash_asU8(): Uint8Array;
  getProTxHash_asB64(): string;
  setProTxHash(value: Uint8Array | string): void;

  getPosePenalty(): number;
  setPosePenalty(value: number): void;

  getIsSynced(): boolean;
  setIsSynced(value: boolean): void;

  getSyncProgress(): number;
  setSyncProgress(value: number): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetMasternodeStatusResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetMasternodeStatusResponse): GetMasternodeStatusResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetMasternodeStatusResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetMasternodeStatusResponse;
  static deserializeBinaryFromReader(message: GetMasternodeStatusResponse, reader: jspb.BinaryReader): GetMasternodeStatusResponse;
}

export namespace GetMasternodeStatusResponse {
  export type AsObject = {
    status: GetMasternodeStatusResponse.StatusMap[keyof GetMasternodeStatusResponse.StatusMap],
    proTxHash: Uint8Array | string,
    posePenalty: number,
    isSynced: boolean,
    syncProgress: number,
  }

  export interface StatusMap {
    UNKNOWN: 0;
    WAITING_FOR_PROTX: 1;
    POSE_BANNED: 2;
    REMOVED: 3;
    OPERATOR_KEY_CHANGED: 4;
    PROTX_IP_CHANGED: 5;
    READY: 6;
    ERROR: 7;
  }

  export const Status: StatusMap;
}

export class GetBlockRequest extends jspb.Message {
  hasHeight(): boolean;
  clearHeight(): void;
  getHeight(): number;
  setHeight(value: number): void;

  hasHash(): boolean;
  clearHash(): void;
  getHash(): string;
  setHash(value: string): void;

  getBlockCase(): GetBlockRequest.BlockCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetBlockRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetBlockRequest): GetBlockRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetBlockRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetBlockRequest;
  static deserializeBinaryFromReader(message: GetBlockRequest, reader: jspb.BinaryReader): GetBlockRequest;
}

export namespace GetBlockRequest {
  export type AsObject = {
    height: number,
    hash: string,
  }

  export enum BlockCase {
    BLOCK_NOT_SET = 0,
    HEIGHT = 1,
    HASH = 2,
  }
}

export class GetBlockResponse extends jspb.Message {
  getBlock(): Uint8Array | string;
  getBlock_asU8(): Uint8Array;
  getBlock_asB64(): string;
  setBlock(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetBlockResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetBlockResponse): GetBlockResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetBlockResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetBlockResponse;
  static deserializeBinaryFromReader(message: GetBlockResponse, reader: jspb.BinaryReader): GetBlockResponse;
}

export namespace GetBlockResponse {
  export type AsObject = {
    block: Uint8Array | string,
  }
}

export class GetBestBlockHeightRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetBestBlockHeightRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetBestBlockHeightRequest): GetBestBlockHeightRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetBestBlockHeightRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetBestBlockHeightRequest;
  static deserializeBinaryFromReader(message: GetBestBlockHeightRequest, reader: jspb.BinaryReader): GetBestBlockHeightRequest;
}

export namespace GetBestBlockHeightRequest {
  export type AsObject = {
  }
}

export class GetBestBlockHeightResponse extends jspb.Message {
  getHeight(): number;
  setHeight(value: number): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetBestBlockHeightResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetBestBlockHeightResponse): GetBestBlockHeightResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetBestBlockHeightResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetBestBlockHeightResponse;
  static deserializeBinaryFromReader(message: GetBestBlockHeightResponse, reader: jspb.BinaryReader): GetBestBlockHeightResponse;
}

export namespace GetBestBlockHeightResponse {
  export type AsObject = {
    height: number,
  }
}

export class BroadcastTransactionRequest extends jspb.Message {
  getTransaction(): Uint8Array | string;
  getTransaction_asU8(): Uint8Array;
  getTransaction_asB64(): string;
  setTransaction(value: Uint8Array | string): void;

  getAllowHighFees(): boolean;
  setAllowHighFees(value: boolean): void;

  getBypassLimits(): boolean;
  setBypassLimits(value: boolean): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BroadcastTransactionRequest.AsObject;
  static toObject(includeInstance: boolean, msg: BroadcastTransactionRequest): BroadcastTransactionRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BroadcastTransactionRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BroadcastTransactionRequest;
  static deserializeBinaryFromReader(message: BroadcastTransactionRequest, reader: jspb.BinaryReader): BroadcastTransactionRequest;
}

export namespace BroadcastTransactionRequest {
  export type AsObject = {
    transaction: Uint8Array | string,
    allowHighFees: boolean,
    bypassLimits: boolean,
  }
}

export class BroadcastTransactionResponse extends jspb.Message {
  getTransactionId(): string;
  setTransactionId(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BroadcastTransactionResponse.AsObject;
  static toObject(includeInstance: boolean, msg: BroadcastTransactionResponse): BroadcastTransactionResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BroadcastTransactionResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BroadcastTransactionResponse;
  static deserializeBinaryFromReader(message: BroadcastTransactionResponse, reader: jspb.BinaryReader): BroadcastTransactionResponse;
}

export namespace BroadcastTransactionResponse {
  export type AsObject = {
    transactionId: string,
  }
}

export class GetTransactionRequest extends jspb.Message {
  getId(): string;
  setId(value: string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetTransactionRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetTransactionRequest): GetTransactionRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetTransactionRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetTransactionRequest;
  static deserializeBinaryFromReader(message: GetTransactionRequest, reader: jspb.BinaryReader): GetTransactionRequest;
}

export namespace GetTransactionRequest {
  export type AsObject = {
    id: string,
  }
}

export class GetTransactionResponse extends jspb.Message {
  getTransaction(): Uint8Array | string;
  getTransaction_asU8(): Uint8Array;
  getTransaction_asB64(): string;
  setTransaction(value: Uint8Array | string): void;

  getBlockHash(): Uint8Array | string;
  getBlockHash_asU8(): Uint8Array;
  getBlockHash_asB64(): string;
  setBlockHash(value: Uint8Array | string): void;

  getHeight(): number;
  setHeight(value: number): void;

  getConfirmations(): number;
  setConfirmations(value: number): void;

  getIsInstantLocked(): boolean;
  setIsInstantLocked(value: boolean): void;

  getIsChainLocked(): boolean;
  setIsChainLocked(value: boolean): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetTransactionResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetTransactionResponse): GetTransactionResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetTransactionResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetTransactionResponse;
  static deserializeBinaryFromReader(message: GetTransactionResponse, reader: jspb.BinaryReader): GetTransactionResponse;
}

export namespace GetTransactionResponse {
  export type AsObject = {
    transaction: Uint8Array | string,
    blockHash: Uint8Array | string,
    height: number,
    confirmations: number,
    isInstantLocked: boolean,
    isChainLocked: boolean,
  }
}

export class BlockHeadersWithChainLocksRequest extends jspb.Message {
  hasFromBlockHash(): boolean;
  clearFromBlockHash(): void;
  getFromBlockHash(): Uint8Array | string;
  getFromBlockHash_asU8(): Uint8Array;
  getFromBlockHash_asB64(): string;
  setFromBlockHash(value: Uint8Array | string): void;

  hasFromBlockHeight(): boolean;
  clearFromBlockHeight(): void;
  getFromBlockHeight(): number;
  setFromBlockHeight(value: number): void;

  getCount(): number;
  setCount(value: number): void;

  getFromBlockCase(): BlockHeadersWithChainLocksRequest.FromBlockCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BlockHeadersWithChainLocksRequest.AsObject;
  static toObject(includeInstance: boolean, msg: BlockHeadersWithChainLocksRequest): BlockHeadersWithChainLocksRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BlockHeadersWithChainLocksRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BlockHeadersWithChainLocksRequest;
  static deserializeBinaryFromReader(message: BlockHeadersWithChainLocksRequest, reader: jspb.BinaryReader): BlockHeadersWithChainLocksRequest;
}

export namespace BlockHeadersWithChainLocksRequest {
  export type AsObject = {
    fromBlockHash: Uint8Array | string,
    fromBlockHeight: number,
    count: number,
  }

  export enum FromBlockCase {
    FROM_BLOCK_NOT_SET = 0,
    FROM_BLOCK_HASH = 1,
    FROM_BLOCK_HEIGHT = 2,
  }
}

export class BlockHeadersWithChainLocksResponse extends jspb.Message {
  hasBlockHeaders(): boolean;
  clearBlockHeaders(): void;
  getBlockHeaders(): BlockHeaders | undefined;
  setBlockHeaders(value?: BlockHeaders): void;

  hasChainLock(): boolean;
  clearChainLock(): void;
  getChainLock(): Uint8Array | string;
  getChainLock_asU8(): Uint8Array;
  getChainLock_asB64(): string;
  setChainLock(value: Uint8Array | string): void;

  getResponsesCase(): BlockHeadersWithChainLocksResponse.ResponsesCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BlockHeadersWithChainLocksResponse.AsObject;
  static toObject(includeInstance: boolean, msg: BlockHeadersWithChainLocksResponse): BlockHeadersWithChainLocksResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BlockHeadersWithChainLocksResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BlockHeadersWithChainLocksResponse;
  static deserializeBinaryFromReader(message: BlockHeadersWithChainLocksResponse, reader: jspb.BinaryReader): BlockHeadersWithChainLocksResponse;
}

export namespace BlockHeadersWithChainLocksResponse {
  export type AsObject = {
    blockHeaders?: BlockHeaders.AsObject,
    chainLock: Uint8Array | string,
  }

  export enum ResponsesCase {
    RESPONSES_NOT_SET = 0,
    BLOCK_HEADERS = 1,
    CHAIN_LOCK = 2,
  }
}

export class BlockHeaders extends jspb.Message {
  clearHeadersList(): void;
  getHeadersList(): Array<Uint8Array | string>;
  getHeadersList_asU8(): Array<Uint8Array>;
  getHeadersList_asB64(): Array<string>;
  setHeadersList(value: Array<Uint8Array | string>): void;
  addHeaders(value: Uint8Array | string, index?: number): Uint8Array | string;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BlockHeaders.AsObject;
  static toObject(includeInstance: boolean, msg: BlockHeaders): BlockHeaders.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BlockHeaders, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BlockHeaders;
  static deserializeBinaryFromReader(message: BlockHeaders, reader: jspb.BinaryReader): BlockHeaders;
}

export namespace BlockHeaders {
  export type AsObject = {
    headersList: Array<Uint8Array | string>,
  }
}

export class GetEstimatedTransactionFeeRequest extends jspb.Message {
  getBlocks(): number;
  setBlocks(value: number): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEstimatedTransactionFeeRequest.AsObject;
  static toObject(includeInstance: boolean, msg: GetEstimatedTransactionFeeRequest): GetEstimatedTransactionFeeRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEstimatedTransactionFeeRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEstimatedTransactionFeeRequest;
  static deserializeBinaryFromReader(message: GetEstimatedTransactionFeeRequest, reader: jspb.BinaryReader): GetEstimatedTransactionFeeRequest;
}

export namespace GetEstimatedTransactionFeeRequest {
  export type AsObject = {
    blocks: number,
  }
}

export class GetEstimatedTransactionFeeResponse extends jspb.Message {
  getFee(): number;
  setFee(value: number): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): GetEstimatedTransactionFeeResponse.AsObject;
  static toObject(includeInstance: boolean, msg: GetEstimatedTransactionFeeResponse): GetEstimatedTransactionFeeResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: GetEstimatedTransactionFeeResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): GetEstimatedTransactionFeeResponse;
  static deserializeBinaryFromReader(message: GetEstimatedTransactionFeeResponse, reader: jspb.BinaryReader): GetEstimatedTransactionFeeResponse;
}

export namespace GetEstimatedTransactionFeeResponse {
  export type AsObject = {
    fee: number,
  }
}

export class TransactionsWithProofsRequest extends jspb.Message {
  hasBloomFilter(): boolean;
  clearBloomFilter(): void;
  getBloomFilter(): BloomFilter | undefined;
  setBloomFilter(value?: BloomFilter): void;

  hasFromBlockHash(): boolean;
  clearFromBlockHash(): void;
  getFromBlockHash(): Uint8Array | string;
  getFromBlockHash_asU8(): Uint8Array;
  getFromBlockHash_asB64(): string;
  setFromBlockHash(value: Uint8Array | string): void;

  hasFromBlockHeight(): boolean;
  clearFromBlockHeight(): void;
  getFromBlockHeight(): number;
  setFromBlockHeight(value: number): void;

  getCount(): number;
  setCount(value: number): void;

  getSendTransactionHashes(): boolean;
  setSendTransactionHashes(value: boolean): void;

  getFromBlockCase(): TransactionsWithProofsRequest.FromBlockCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): TransactionsWithProofsRequest.AsObject;
  static toObject(includeInstance: boolean, msg: TransactionsWithProofsRequest): TransactionsWithProofsRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: TransactionsWithProofsRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): TransactionsWithProofsRequest;
  static deserializeBinaryFromReader(message: TransactionsWithProofsRequest, reader: jspb.BinaryReader): TransactionsWithProofsRequest;
}

export namespace TransactionsWithProofsRequest {
  export type AsObject = {
    bloomFilter?: BloomFilter.AsObject,
    fromBlockHash: Uint8Array | string,
    fromBlockHeight: number,
    count: number,
    sendTransactionHashes: boolean,
  }

  export enum FromBlockCase {
    FROM_BLOCK_NOT_SET = 0,
    FROM_BLOCK_HASH = 2,
    FROM_BLOCK_HEIGHT = 3,
  }
}

export class BloomFilter extends jspb.Message {
  getVData(): Uint8Array | string;
  getVData_asU8(): Uint8Array;
  getVData_asB64(): string;
  setVData(value: Uint8Array | string): void;

  getNHashFuncs(): number;
  setNHashFuncs(value: number): void;

  getNTweak(): number;
  setNTweak(value: number): void;

  getNFlags(): number;
  setNFlags(value: number): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): BloomFilter.AsObject;
  static toObject(includeInstance: boolean, msg: BloomFilter): BloomFilter.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: BloomFilter, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): BloomFilter;
  static deserializeBinaryFromReader(message: BloomFilter, reader: jspb.BinaryReader): BloomFilter;
}

export namespace BloomFilter {
  export type AsObject = {
    vData: Uint8Array | string,
    nHashFuncs: number,
    nTweak: number,
    nFlags: number,
  }
}

export class TransactionsWithProofsResponse extends jspb.Message {
  hasRawTransactions(): boolean;
  clearRawTransactions(): void;
  getRawTransactions(): RawTransactions | undefined;
  setRawTransactions(value?: RawTransactions): void;

  hasInstantSendLockMessages(): boolean;
  clearInstantSendLockMessages(): void;
  getInstantSendLockMessages(): InstantSendLockMessages | undefined;
  setInstantSendLockMessages(value?: InstantSendLockMessages): void;

  hasRawMerkleBlock(): boolean;
  clearRawMerkleBlock(): void;
  getRawMerkleBlock(): Uint8Array | string;
  getRawMerkleBlock_asU8(): Uint8Array;
  getRawMerkleBlock_asB64(): string;
  setRawMerkleBlock(value: Uint8Array | string): void;

  getResponsesCase(): TransactionsWithProofsResponse.ResponsesCase;
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): TransactionsWithProofsResponse.AsObject;
  static toObject(includeInstance: boolean, msg: TransactionsWithProofsResponse): TransactionsWithProofsResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: TransactionsWithProofsResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): TransactionsWithProofsResponse;
  static deserializeBinaryFromReader(message: TransactionsWithProofsResponse, reader: jspb.BinaryReader): TransactionsWithProofsResponse;
}

export namespace TransactionsWithProofsResponse {
  export type AsObject = {
    rawTransactions?: RawTransactions.AsObject,
    instantSendLockMessages?: InstantSendLockMessages.AsObject,
    rawMerkleBlock: Uint8Array | string,
  }

  export enum ResponsesCase {
    RESPONSES_NOT_SET = 0,
    RAW_TRANSACTIONS = 1,
    INSTANT_SEND_LOCK_MESSAGES = 2,
    RAW_MERKLE_BLOCK = 3,
  }
}

export class RawTransactions extends jspb.Message {
  clearTransactionsList(): void;
  getTransactionsList(): Array<Uint8Array | string>;
  getTransactionsList_asU8(): Array<Uint8Array>;
  getTransactionsList_asB64(): Array<string>;
  setTransactionsList(value: Array<Uint8Array | string>): void;
  addTransactions(value: Uint8Array | string, index?: number): Uint8Array | string;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): RawTransactions.AsObject;
  static toObject(includeInstance: boolean, msg: RawTransactions): RawTransactions.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: RawTransactions, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): RawTransactions;
  static deserializeBinaryFromReader(message: RawTransactions, reader: jspb.BinaryReader): RawTransactions;
}

export namespace RawTransactions {
  export type AsObject = {
    transactionsList: Array<Uint8Array | string>,
  }
}

export class InstantSendLockMessages extends jspb.Message {
  clearMessagesList(): void;
  getMessagesList(): Array<Uint8Array | string>;
  getMessagesList_asU8(): Array<Uint8Array>;
  getMessagesList_asB64(): Array<string>;
  setMessagesList(value: Array<Uint8Array | string>): void;
  addMessages(value: Uint8Array | string, index?: number): Uint8Array | string;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): InstantSendLockMessages.AsObject;
  static toObject(includeInstance: boolean, msg: InstantSendLockMessages): InstantSendLockMessages.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: InstantSendLockMessages, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): InstantSendLockMessages;
  static deserializeBinaryFromReader(message: InstantSendLockMessages, reader: jspb.BinaryReader): InstantSendLockMessages;
}

export namespace InstantSendLockMessages {
  export type AsObject = {
    messagesList: Array<Uint8Array | string>,
  }
}

export class MasternodeListRequest extends jspb.Message {
  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): MasternodeListRequest.AsObject;
  static toObject(includeInstance: boolean, msg: MasternodeListRequest): MasternodeListRequest.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: MasternodeListRequest, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): MasternodeListRequest;
  static deserializeBinaryFromReader(message: MasternodeListRequest, reader: jspb.BinaryReader): MasternodeListRequest;
}

export namespace MasternodeListRequest {
  export type AsObject = {
  }
}

export class MasternodeListResponse extends jspb.Message {
  getMasternodeListDiff(): Uint8Array | string;
  getMasternodeListDiff_asU8(): Uint8Array;
  getMasternodeListDiff_asB64(): string;
  setMasternodeListDiff(value: Uint8Array | string): void;

  serializeBinary(): Uint8Array;
  toObject(includeInstance?: boolean): MasternodeListResponse.AsObject;
  static toObject(includeInstance: boolean, msg: MasternodeListResponse): MasternodeListResponse.AsObject;
  static extensions: {[key: number]: jspb.ExtensionFieldInfo<jspb.Message>};
  static extensionsBinary: {[key: number]: jspb.ExtensionFieldBinaryInfo<jspb.Message>};
  static serializeBinaryToWriter(message: MasternodeListResponse, writer: jspb.BinaryWriter): void;
  static deserializeBinary(bytes: Uint8Array): MasternodeListResponse;
  static deserializeBinaryFromReader(message: MasternodeListResponse, reader: jspb.BinaryReader): MasternodeListResponse;
}

export namespace MasternodeListResponse {
  export type AsObject = {
    masternodeListDiff: Uint8Array | string,
  }
}

