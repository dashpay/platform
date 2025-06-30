/**
 * WASM SDK TypeScript Definitions
 * 
 * This file provides TypeScript type definitions for the Dash Platform WASM SDK.
 * It enables type-safe interaction with the Dash Platform from JavaScript/TypeScript
 * applications running in browser environments.
 */

declare module "dash-wasm-sdk" {
  /**
   * Initialize the WASM module
   * Must be called before using any other SDK functions
   */
  export function start(): Promise<void>;

  /**
   * Error categories for better error handling
   */
  export enum ErrorCategory {
    Network = "Network",
    Serialization = "Serialization",
    Validation = "Validation",
    Platform = "Platform",
    ProofVerification = "ProofVerification",
    StateTransition = "StateTransition",
    Identity = "Identity",
    Document = "Document",
    Contract = "Contract",
    Unknown = "Unknown"
  }

  /**
   * WASM-specific error type
   */
  export class WasmError extends Error {
    readonly category: ErrorCategory;
    readonly message: string;
  }

  /**
   * Main SDK interface
   */
  export class WasmSdk {
    constructor(
      network: "mainnet" | "testnet" | "devnet",
      contextProvider?: ContextProvider
    );

    /**
     * Get the network this SDK is connected to
     */
    get network(): string;

    /**
     * Check if SDK is ready
     */
    isReady(): boolean;
  }

  /**
   * Context provider for blockchain context
   */
  export class ContextProvider {
    /**
     * Get current block height
     */
    getBlockHeight(): Promise<number>;

    /**
     * Get current core chain locked height
     */
    getCoreChainLockedHeight(): Promise<number>;

    /**
     * Get current time in milliseconds
     */
    getTimeMillis(): Promise<number>;
  }

  /**
   * Options for fetch operations
   */
  export class FetchOptions {
    constructor();
    withRetries(retries: number): FetchOptions;
    withTimeout(timeout: number): FetchOptions;
  }

  /**
   * Response from fetch operations
   */
  export interface FetchResponse {
    readonly data: any;
    readonly found: boolean;
    readonly metadataHeight: bigint;
    readonly metadataCoreChainLockedHeight: number;
    readonly metadataEpoch: number;
    readonly metadataTimeMs: bigint;
    readonly metadataProtocolVersion: number;
    readonly metadataChainId: string;
  }

  /**
   * Fetch an identity from the platform
   */
  export function fetchIdentity(
    sdk: WasmSdk,
    identityId: string,
    options?: FetchOptions
  ): Promise<any>;

  /**
   * Fetch a data contract from the platform
   */
  export function fetchDataContract(
    sdk: WasmSdk,
    contractId: string,
    options?: FetchOptions
  ): Promise<any>;

  /**
   * Fetch documents from the platform
   */
  export function fetchDocuments(
    sdk: WasmSdk,
    contractId: string,
    documentType: string,
    whereClause: any,
    options?: FetchOptions
  ): Promise<any>;

  /**
   * Query types
   */
  export class IdentifierQuery {
    constructor(id: string);
    readonly id: string;
  }

  export class IdentifiersQuery {
    constructor(ids: string[]);
    readonly ids: string[];
    readonly count: number;
  }

  export class LimitQuery {
    constructor();
    limit?: number;
    offset?: number;
    setLimit(limit: number): void;
    setOffset(offset: number): void;
    setStartKey(key: Uint8Array): void;
    setStartIncluded(included: boolean): void;
  }

  export class DocumentQuery {
    constructor(contractId: string, documentType: string);
    addWhereClause(field: string, operator: string, value: any): void;
    addOrderBy(field: string, ascending: boolean): void;
    setLimit(limit: number): void;
    setOffset(offset: number): void;
    readonly contractId: string;
    readonly documentType: string;
    readonly limit?: number;
    readonly offset?: number;
    getWhereClauses(): any[];
    getOrderByClauses(): any[];
  }

  /**
   * State transition functions
   */

  /**
   * Create a new identity
   */
  export function createIdentity(
    assetLockProof: Uint8Array,
    publicKeys: any
  ): Uint8Array;

  /**
   * Top up an existing identity
   */
  export function topupIdentity(
    identityId: string,
    assetLockProof: Uint8Array
  ): Uint8Array;

  /**
   * Update an identity
   */
  export function updateIdentity(
    identityId: string,
    revision: bigint,
    addPublicKeys: any,
    disablePublicKeys: any,
    publicKeysDisabledAt?: bigint,
    signaturePublicKeyId: number
  ): Uint8Array;

  /**
   * Create a data contract
   */
  export function createDataContract(
    ownerId: string,
    contractDefinition: any,
    identityNonce: bigint,
    signaturePublicKeyId: number
  ): Uint8Array;

  /**
   * Update a data contract
   */
  export function updateDataContract(
    contractId: string,
    ownerId: string,
    contractDefinition: any,
    identityContractNonce: bigint,
    signaturePublicKeyId: number
  ): Uint8Array;

  /**
   * Document batch builder
   */
  export class DocumentBatchBuilder {
    constructor(ownerId: string);
    
    addCreateDocument(
      contractId: string,
      documentType: string,
      documentId: string,
      data: any
    ): void;
    
    addDeleteDocument(
      contractId: string,
      documentType: string,
      documentId: string
    ): void;
    
    addReplaceDocument(
      contractId: string,
      documentType: string,
      documentId: string,
      revision: number,
      data: any
    ): void;
    
    build(signaturePublicKeyId: number): Uint8Array;
  }

  /**
   * Identity transition builder
   */
  export class IdentityTransitionBuilder {
    constructor();
    
    setIdentityId(identityId: string): void;
    setRevision(revision: bigint): void;
    
    buildCreateTransition(assetLockProof: Uint8Array): Uint8Array;
    buildTopUpTransition(assetLockProof: Uint8Array): Uint8Array;
    buildUpdateTransition(
      signaturePublicKeyId: number,
      publicKeysDisabledAt?: bigint
    ): Uint8Array;
  }

  /**
   * Data contract transition builder
   */
  export class DataContractTransitionBuilder {
    constructor(ownerId: string);
    
    setContractId(contractId: string): void;
    setVersion(version: number): void;
    setUserFeeIncrease(feeIncrease: number): void;
    setIdentityNonce(nonce: bigint): void;
    setIdentityContractNonce(nonce: bigint): void;
    addDocumentSchema(documentType: string, schema: any): void;
    setContractDefinition(definition: any): void;
    
    buildCreateTransition(signaturePublicKeyId: number): Uint8Array;
    buildUpdateTransition(signaturePublicKeyId: number): Uint8Array;
  }

  /**
   * Broadcast a state transition
   */
  export function broadcastStateTransition(
    sdk: WasmSdk,
    stateTransition: Uint8Array,
    options?: BroadcastOptions
  ): Promise<BroadcastResponse>;

  export interface BroadcastOptions {
    retries?: number;
    timeout?: number;
  }

  export interface BroadcastResponse {
    success: boolean;
    metadata?: any;
    error?: string;
  }

  /**
   * Nonce management
   */
  export interface NonceResponse {
    nonce: bigint;
    previousValue: bigint;
    metadata: any;
  }

  export function getIdentityNonce(
    sdk: WasmSdk,
    identityId: string,
    cached: boolean
  ): Promise<NonceResponse>;

  export function incrementIdentityNonce(
    sdk: WasmSdk,
    identityId: string,
    count?: number
  ): Promise<NonceResponse>;

  export function getIdentityContractNonce(
    sdk: WasmSdk,
    identityId: string,
    contractId: string,
    cached: boolean
  ): Promise<NonceResponse>;

  export function incrementIdentityContractNonce(
    sdk: WasmSdk,
    identityId: string,
    contractId: string,
    count?: number
  ): Promise<NonceResponse>;

  /**
   * Transport layer
   */
  export class WasmDapiTransport {
    constructor(nodeAddresses: string[]);
    setTimeout(timeoutMs: number): void;
    setMaxRetries(maxRetries: number): void;
  }

  export class WasmPlatformClient {
    constructor(transport: WasmDapiTransport);
    
    getIdentity(identityId: string, prove: boolean): Promise<any>;
    getDataContract(contractId: string, prove: boolean): Promise<any>;
    broadcastStateTransition(stateTransition: Uint8Array): Promise<any>;
  }

  export class WasmCoreClient {
    constructor(transport: WasmDapiTransport);
    
    getBestBlockHash(): Promise<string>;
    getBlock(blockHash: string): Promise<any>;
  }

  /**
   * Proof verification functions
   */
  export function verifyIdentityProof(
    proof: Uint8Array,
    identityId: string,
    isProofSubset: boolean,
    platformVersion: number
  ): any;

  export function verifyDataContractProof(
    proof: Uint8Array,
    contractId: string,
    isProofSubset: boolean
  ): any;

  export function verifyDocumentsProof(
    proof: Uint8Array,
    contract: any,
    documentType: string,
    whereClauses: any,
    orderBy: any,
    limit?: number,
    offset?: number,
    platformVersion: number
  ): any;

  /**
   * DPP (Dash Platform Protocol) types
   */
  export class IdentityWasm {
    toJSON(): any;
    toObject(): any;
    getId(): string;
    getPublicKeys(): any[];
    getBalance(): bigint;
    getRevision(): bigint;
  }

  export class DataContractWasm {
    toJSON(): any;
    toObject(): any;
    getId(): string;
    getOwnerId(): string;
    getVersion(): number;
    getDocumentSchemas(): any;
  }

  export class DocumentWasm {
    toJSON(): any;
    toObject(): any;
    getId(): string;
    getRevision(): number;
    getCreatedAt(): bigint;
    getUpdatedAt(): bigint;
    getData(): any;
  }

  /**
   * Metadata operations
   */
  export interface Metadata {
    height: bigint;
    coreChainLockedHeight: number;
    epoch: number;
    timeMs: bigint;
    protocolVersion: number;
    chainId: string;
  }

  export function isMetadataValid(metadata: Metadata): boolean;
  export function getLatestMetadata(metadataList: Metadata[]): Metadata;

  /**
   * Signer functionality
   */
  export class WasmSigner {
    constructor();
    setIdentityId(identityId: string): void;
    addPrivateKey(
      publicKeyId: number,
      privateKey: Uint8Array,
      keyType: string,
      purpose: number
    ): void;
    removePrivateKey(publicKeyId: number): boolean;
    signData(data: Uint8Array, publicKeyId: number): Promise<Uint8Array>;
    getKeyCount(): number;
    hasKey(publicKeyId: number): boolean;
    getKeyIds(): number[];
  }

  export class BrowserSigner {
    constructor();
    generateKeyPair(
      keyType: string,
      publicKeyId: number
    ): Promise<CryptoKey>;
    signWithStoredKey(
      data: Uint8Array,
      publicKeyId: number
    ): Promise<Uint8Array>;
  }

  export class HDSigner {
    constructor(mnemonic: string, derivationPath: string);
    static generateMnemonic(wordCount: number): string;
    deriveKey(index: number): Uint8Array;
    get derivationPath(): string;
  }

  /**
   * Fetch unproved operations (without proof verification)
   */
  export function fetchIdentityUnproved(
    sdk: WasmSdk,
    identityId: string,
    options?: FetchOptions
  ): Promise<any>;

  export function fetchDataContractUnproved(
    sdk: WasmSdk,
    contractId: string,
    options?: FetchOptions
  ): Promise<any>;

  export function fetchDocumentsUnproved(
    sdk: WasmSdk,
    contractId: string,
    documentType: string,
    whereClause: any,
    orderBy: any,
    limit?: number,
    startAt?: Uint8Array,
    options?: FetchOptions
  ): Promise<any>;

  export function fetchIdentityByKeyUnproved(
    sdk: WasmSdk,
    publicKeyHash: Uint8Array,
    options?: FetchOptions
  ): Promise<any>;

  export function fetchDataContractHistoryUnproved(
    sdk: WasmSdk,
    contractId: string,
    startAtMs?: number,
    limit?: number,
    offset?: number,
    options?: FetchOptions
  ): Promise<any>;

  export function fetchBatchUnproved(
    sdk: WasmSdk,
    requests: Array<{ type: "identity" | "dataContract"; id: string }>,
    options?: FetchOptions
  ): Promise<any[]>;

  /**
   * Token functionality
   */
  export class TokenOptions {
    constructor();
    withRetries(retries: number): TokenOptions;
    withTimeout(timeoutMs: number): TokenOptions;
  }

  export function mintTokens(
    sdk: WasmSdk,
    tokenId: string,
    amount: number,
    recipientIdentityId: string,
    options?: TokenOptions
  ): Promise<any>;

  export function burnTokens(
    sdk: WasmSdk,
    tokenId: string,
    amount: number,
    ownerIdentityId: string,
    options?: TokenOptions
  ): Promise<any>;

  export function transferTokens(
    sdk: WasmSdk,
    tokenId: string,
    amount: number,
    senderIdentityId: string,
    recipientIdentityId: string,
    options?: TokenOptions
  ): Promise<any>;

  export function freezeTokens(
    sdk: WasmSdk,
    tokenId: string,
    identityId: string,
    options?: TokenOptions
  ): Promise<any>;

  export function unfreezeTokens(
    sdk: WasmSdk,
    tokenId: string,
    identityId: string,
    options?: TokenOptions
  ): Promise<any>;

  export function getTokenBalance(
    sdk: WasmSdk,
    tokenId: string,
    identityId: string,
    options?: TokenOptions
  ): Promise<{ balance: number; frozen: boolean }>;

  export function getTokenInfo(
    sdk: WasmSdk,
    tokenId: string,
    options?: TokenOptions
  ): Promise<{
    totalSupply: number;
    decimals: number;
    name: string;
    symbol: string;
  }>;

  export function createTokenIssuance(
    dataContractId: string,
    tokenPosition: number,
    amount: number,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function createTokenBurn(
    dataContractId: string,
    tokenPosition: number,
    amount: number,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function getContractTokens(
    sdk: WasmSdk,
    dataContractId: string,
    options?: TokenOptions
  ): Promise<any[]>;

  /**
   * Withdrawal functionality
   */
  export class WithdrawalOptions {
    constructor();
    withRetries(retries: number): WithdrawalOptions;
    withTimeout(timeoutMs: number): WithdrawalOptions;
    withFeeMultiplier(multiplier: number): WithdrawalOptions;
  }

  export function withdrawFromIdentity(
    sdk: WasmSdk,
    identityId: string,
    amount: number,
    toAddress: string,
    signaturePublicKeyId: number,
    options?: WithdrawalOptions
  ): Promise<any>;

  export function createWithdrawalTransition(
    identityId: string,
    amount: number,
    toAddress: string,
    outputScript: Uint8Array,
    identityNonce: number,
    signaturePublicKeyId: number,
    coreFeePerByte?: number
  ): Uint8Array;

  export function getWithdrawalStatus(
    sdk: WasmSdk,
    withdrawalId: string,
    options?: WithdrawalOptions
  ): Promise<{
    status: string;
    amount: number;
    transactionId: string | null;
  }>;

  export function getIdentityWithdrawals(
    sdk: WasmSdk,
    identityId: string,
    limit?: number,
    offset?: number,
    options?: WithdrawalOptions
  ): Promise<{
    withdrawals: any[];
    totalCount: number;
  }>;

  export function calculateWithdrawalFee(
    amount: number,
    outputScriptSize: number,
    coreFeePerByte?: number
  ): number;

  export function broadcastWithdrawal(
    sdk: WasmSdk,
    withdrawalTransition: Uint8Array,
    options?: WithdrawalOptions
  ): Promise<{
    success: boolean;
    transactionId: string | null;
    error?: string;
  }>;

  export function estimateWithdrawalTime(
    sdk: WasmSdk,
    options?: WithdrawalOptions
  ): Promise<{
    estimatedMinutes: number;
    currentQueueLength: number;
  }>;

  /**
   * Cache management
   */
  export class WasmCacheManager {
    constructor();
    setTTLs(
      contractsTtl: number,
      identitiesTtl: number,
      documentsTtl: number,
      tokensTtl: number,
      quorumKeysTtl: number,
      metadataTtl: number
    ): void;
    cacheContract(contractId: string, contractData: Uint8Array): void;
    getCachedContract(contractId: string): Uint8Array | undefined;
    cacheIdentity(identityId: string, identityData: Uint8Array): void;
    getCachedIdentity(identityId: string): Uint8Array | undefined;
    cacheDocument(documentKey: string, documentData: Uint8Array): void;
    getCachedDocument(documentKey: string): Uint8Array | undefined;
    cacheToken(tokenId: string, tokenData: Uint8Array): void;
    getCachedToken(tokenId: string): Uint8Array | undefined;
    cacheQuorumKeys(epoch: number, keysData: Uint8Array): void;
    getCachedQuorumKeys(epoch: number): Uint8Array | undefined;
    cacheMetadata(key: string, metadata: Uint8Array): void;
    getCachedMetadata(key: string): Uint8Array | undefined;
    clearAll(): void;
    clearCache(cacheType: string): void;
    cleanupExpired(): void;
    getStats(): {
      contracts: number;
      identities: number;
      documents: number;
      tokens: number;
      quorumKeys: number;
      metadata: number;
      totalEntries: number;
    };
  }

  /**
   * Epoch and evonode functionality
   */
  export class Epoch {
    get index(): number;
    get startBlockHeight(): number;
    get startBlockCoreHeight(): number;
    get startTimeMs(): number;
    get feeMultiplier(): number;
    toObject(): any;
  }

  export class Evonode {
    get proTxHash(): Uint8Array;
    get ownerAddress(): string;
    get votingAddress(): string;
    get isHPMN(): boolean;
    get platformP2PPort(): number;
    get platformHTTPPort(): number;
    get nodeIP(): string;
    toObject(): any;
  }

  export function getCurrentEpoch(sdk: WasmSdk): Promise<Epoch>;
  export function getEpochByIndex(sdk: WasmSdk, index: number): Promise<Epoch>;
  export function getCurrentEvonodes(sdk: WasmSdk): Promise<Evonode[]>;
  export function getEvonodesForEpoch(
    sdk: WasmSdk,
    epochIndex: number
  ): Promise<Evonode[]>;
  export function getEvonodeByProTxHash(
    sdk: WasmSdk,
    proTxHash: Uint8Array
  ): Promise<Evonode>;
  export function getCurrentQuorum(sdk: WasmSdk): Promise<{
    threshold: number;
    members: any[];
  }>;
  export function calculateEpochBlocks(network: string): number;
  export function estimateNextEpochTime(
    sdk: WasmSdk,
    currentBlockHeight: number
  ): Promise<{
    blocksRemaining: number;
    minutesRemaining: number;
    estimatedTimeMs: number;
  }>;
  export function getEpochForBlockHeight(
    sdk: WasmSdk,
    blockHeight: number
  ): Promise<Epoch>;

  /**
   * Identity balance and revision functionality
   */
  export interface IdentityBalance {
    readonly confirmed: number;
    readonly unconfirmed: number;
    readonly total: number;
    toObject(): any;
  }

  export interface IdentityRevision {
    readonly revision: number;
    readonly updatedAt: number;
    readonly publicKeysCount: number;
    toObject(): any;
  }

  export interface IdentityInfo {
    readonly id: string;
    readonly balance: IdentityBalance;
    readonly revision: IdentityRevision;
    toObject(): any;
  }

  export function fetchIdentityBalance(
    sdk: WasmSdk,
    identityId: string
  ): Promise<IdentityBalance>;

  export function fetchIdentityRevision(
    sdk: WasmSdk,
    identityId: string
  ): Promise<IdentityRevision>;

  export function fetchIdentityInfo(
    sdk: WasmSdk,
    identityId: string
  ): Promise<IdentityInfo>;

  export function fetchIdentityBalanceHistory(
    sdk: WasmSdk,
    identityId: string,
    fromTimestamp?: number,
    toTimestamp?: number,
    limit?: number
  ): Promise<any[]>;

  export function checkIdentityBalance(
    sdk: WasmSdk,
    identityId: string,
    requiredAmount: number,
    useUnconfirmed: boolean
  ): Promise<boolean>;

  export function estimateCreditsNeeded(
    operationType: string,
    dataSizeBytes?: number
  ): number;

  export function monitorIdentityBalance(
    sdk: WasmSdk,
    identityId: string,
    callback: (balance: IdentityBalance) => void,
    pollIntervalMs?: number
  ): Promise<{
    identityId: string;
    interval: number;
    active: boolean;
  }>;

  /**
   * Metadata verification
   */
  export class Metadata {
    constructor(
      height: number,
      coreChainLockedHeight: number,
      epoch: number,
      timeMs: number,
      protocolVersion: number,
      chainId: string
    );
    get height(): number;
    get coreChainLockedHeight(): number;
    get epoch(): number;
    get timeMs(): number;
    get protocolVersion(): number;
    get chainId(): string;
    toObject(): any;
  }

  export class MetadataVerificationConfig {
    constructor();
    setMaxHeightDifference(blocks: number): void;
    setMaxTimeDifference(ms: number): void;
    setVerifyTime(verify: boolean): void;
    setVerifyHeight(verify: boolean): void;
    setVerifyChainId(verify: boolean): void;
    setExpectedChainId(chainId: string): void;
  }

  export class MetadataVerificationResult {
    get valid(): boolean;
    get heightValid(): boolean | undefined;
    get timeValid(): boolean | undefined;
    get chainIdValid(): boolean | undefined;
    get heightDifference(): number | undefined;
    get timeDifferenceMs(): number | undefined;
    get errorMessage(): string | undefined;
    toObject(): any;
  }

  export function verifyMetadata(
    metadata: Metadata,
    currentHeight: number,
    currentTimeMs?: number,
    config: MetadataVerificationConfig
  ): MetadataVerificationResult;

  export function compareMetadata(
    metadata1: Metadata,
    metadata2: Metadata
  ): number;

  export function getMostRecentMetadata(
    metadataList: any[]
  ): Metadata;

  export function isMetadataStale(
    metadata: Metadata,
    maxAgeMs: number,
    maxHeightBehind: number,
    currentHeight?: number
  ): boolean;

  /**
   * Optimization utilities
   */
  export class FeatureFlags {
    constructor();
    static minimal(): FeatureFlags;
    setEnableIdentity(enable: boolean): void;
    setEnableContracts(enable: boolean): void;
    setEnableDocuments(enable: boolean): void;
    setEnableTokens(enable: boolean): void;
    setEnableWithdrawals(enable: boolean): void;
    setEnableVoting(enable: boolean): void;
    setEnableCache(enable: boolean): void;
    setEnableProofVerification(enable: boolean): void;
    getEstimatedSizeReduction(): string;
  }

  export class MemoryOptimizer {
    constructor();
    trackAllocation(size: number): void;
    getStats(): string;
    reset(): void;
    static forceGC(): void;
  }

  export function optimizeUint8Array(data: Uint8Array): Uint8Array;

  export class BatchOptimizer {
    constructor();
    setBatchSize(size: number): void;
    setMaxConcurrent(max: number): void;
    getOptimalBatchCount(totalItems: number): number;
    getBatchBoundaries(totalItems: number, batchIndex: number): {
      start: number;
      end: number;
      size: number;
    };
  }

  export function initStringCache(): void;
  export function internString(s: string): string;
  export function clearStringCache(): void;

  export class CompressionUtils {
    static shouldCompress(dataSize: number): boolean;
    static estimateCompressionRatio(data: Uint8Array): number;
  }

  export class PerformanceMonitor {
    constructor();
    mark(label: string): void;
    getReport(): string;
    reset(): void;
  }

  export function getOptimizationRecommendations(): string[];

  /**
   * Voting functionality
   */
  export enum VoteType {
    Yes = "Yes",
    No = "No",
    Abstain = "Abstain"
  }

  export class VoteChoice {
    static yes(reason?: string): VoteChoice;
    static no(reason?: string): VoteChoice;
    static abstain(reason?: string): VoteChoice;
    get voteType(): string;
    get reason(): string | undefined;
  }

  export class VotePoll {
    get id(): string;
    get title(): string;
    get description(): string;
    get startTime(): number;
    get endTime(): number;
    get voteOptions(): string[];
    get requiredVotes(): number;
    get currentVotes(): number;
    isActive(): boolean;
    getRemainingTime(): number;
    toObject(): any;
  }

  export class VoteResult {
    get pollId(): string;
    get yesVotes(): number;
    get noVotes(): number;
    get abstainVotes(): number;
    get totalVotes(): number;
    get passed(): boolean;
    getPercentage(voteType: string): number;
    toObject(): any;
  }

  export function createVoteTransition(
    voterId: string,
    pollId: string,
    voteChoice: VoteChoice,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function fetchActiveVotePolls(
    sdk: WasmSdk,
    limit?: number
  ): Promise<VotePoll[]>;

  export function fetchVotePoll(
    sdk: WasmSdk,
    pollId: string
  ): Promise<VotePoll>;

  export function fetchVoteResults(
    sdk: WasmSdk,
    pollId: string
  ): Promise<VoteResult>;

  export function hasVoted(
    sdk: WasmSdk,
    voterId: string,
    pollId: string
  ): Promise<boolean>;

  export function getVoterVote(
    sdk: WasmSdk,
    voterId: string,
    pollId: string
  ): Promise<string | undefined>;

  export function delegateVotingPower(
    delegatorId: string,
    delegateId: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function revokeVotingDelegation(
    delegatorId: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function createVotePoll(
    creatorId: string,
    title: string,
    description: string,
    durationDays: number,
    voteOptions: string[],
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function getVotingPower(
    sdk: WasmSdk,
    identityId: string
  ): Promise<number>;

  export function monitorVotePoll(
    sdk: WasmSdk,
    pollId: string,
    callback: (result: VoteResult) => void,
    pollIntervalMs?: number
  ): Promise<{
    pollId: string;
    interval: number;
    active: boolean;
  }>;

  /**
   * Group Actions functionality
   */
  export enum GroupType {
    Multisig = "Multisig",
    DAO = "DAO",
    Committee = "Committee",
    Custom = "Custom"
  }

  export enum MemberRole {
    Owner = "Owner",
    Admin = "Admin",
    Member = "Member",
    Observer = "Observer"
  }

  export class Group {
    get id(): string;
    get name(): string;
    get description(): string;
    get groupType(): string;
    get createdAt(): number;
    get memberCount(): number;
    get threshold(): number;
    get active(): boolean;
    toObject(): any;
  }

  export class GroupMember {
    get identityId(): string;
    get role(): string;
    get joinedAt(): number;
    get permissions(): string[];
    hasPermission(permission: string): boolean;
  }

  export class GroupProposal {
    get id(): string;
    get groupId(): string;
    get proposerId(): string;
    get title(): string;
    get description(): string;
    get actionType(): string;
    get actionData(): Uint8Array;
    get createdAt(): number;
    get expiresAt(): number;
    get approvals(): number;
    get rejections(): number;
    get executed(): boolean;
    isActive(): boolean;
    isExpired(): boolean;
    toObject(): any;
  }

  export function createGroup(
    creatorId: string,
    name: string,
    description: string,
    groupType: string,
    threshold: number,
    initialMembers: string[],
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function addGroupMember(
    groupId: string,
    adminId: string,
    newMemberId: string,
    role: string,
    permissions: string[],
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function removeGroupMember(
    groupId: string,
    adminId: string,
    memberId: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function createGroupProposal(
    groupId: string,
    proposerId: string,
    title: string,
    description: string,
    actionType: string,
    actionData: Uint8Array,
    durationHours: number,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function voteOnProposal(
    proposalId: string,
    voterId: string,
    approve: boolean,
    comment?: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function executeProposal(
    proposalId: string,
    executorId: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function fetchGroup(
    sdk: WasmSdk,
    groupId: string
  ): Promise<Group>;

  export function fetchGroupMembers(
    sdk: WasmSdk,
    groupId: string
  ): Promise<GroupMember[]>;

  export function fetchGroupProposals(
    sdk: WasmSdk,
    groupId: string,
    activeOnly: boolean
  ): Promise<GroupProposal[]>;

  export function fetchUserGroups(
    sdk: WasmSdk,
    userId: string
  ): Promise<Group[]>;

  export function checkGroupPermission(
    sdk: WasmSdk,
    groupId: string,
    userId: string,
    permission: string
  ): Promise<boolean>;

  /**
   * Contract History functionality
   */
  export class ContractVersion {
    get version(): number;
    get schemaHash(): string;
    get ownerId(): string;
    get createdAt(): number;
    get documentTypesCount(): number;
    get totalDocuments(): number;
    toObject(): any;
  }

  export class ContractHistoryEntry {
    get contractId(): string;
    get version(): number;
    get operation(): string;
    get timestamp(): number;
    get changes(): string[];
    get transactionHash(): string | undefined;
    toObject(): any;
  }

  export class SchemaChange {
    get documentType(): string;
    get changeType(): string;
    get fieldName(): string | undefined;
    get oldValue(): string | undefined;
    get newValue(): string | undefined;
  }

  export function fetchContractHistory(
    sdk: WasmSdk,
    contractId: string,
    startAtMs?: number,
    limit?: number,
    offset?: number
  ): Promise<ContractHistoryEntry[]>;

  export function fetchContractVersions(
    sdk: WasmSdk,
    contractId: string
  ): Promise<ContractVersion[]>;

  export function getSchemaChanges(
    sdk: WasmSdk,
    contractId: string,
    fromVersion: number,
    toVersion: number
  ): Promise<SchemaChange[]>;

  export function fetchContractAtVersion(
    sdk: WasmSdk,
    contractId: string,
    version: number
  ): Promise<any>;

  export function checkContractUpdates(
    sdk: WasmSdk,
    contractId: string,
    currentVersion: number
  ): Promise<boolean>;

  export function getMigrationGuide(
    sdk: WasmSdk,
    contractId: string,
    fromVersion: number,
    toVersion: number
  ): Promise<{
    fromVersion: number;
    toVersion: number;
    steps: string[];
    warnings: string[];
  }>;

  export function monitorContractUpdates(
    sdk: WasmSdk,
    contractId: string,
    currentVersion: number,
    callback: (update: {
      hasUpdate: boolean;
      latestVersion: number;
      currentVersion: number;
    }) => void,
    pollIntervalMs?: number
  ): Promise<{
    contractId: string;
    currentVersion: number;
    interval: number;
    active: boolean;
  }>;

  /**
   * Prefunded Specialized Balance functionality
   */
  export enum BalanceType {
    Voting = "Voting",
    Staking = "Staking",
    Reserved = "Reserved",
    Escrow = "Escrow",
    Reward = "Reward",
    Custom = "Custom"
  }

  export class PrefundedBalance {
    get balanceType(): string;
    get amount(): number;
    get lockedUntil(): number | undefined;
    get purpose(): string;
    get canWithdraw(): boolean;
    isLocked(): boolean;
    getRemainingLockTime(): number;
    toObject(): any;
  }

  export class BalanceAllocation {
    get identityId(): string;
    get balanceType(): string;
    get allocatedAmount(): number;
    get usedAmount(): number;
    getAvailableAmount(): number;
    get allocatedAt(): number;
    get expiresAt(): number | undefined;
    isExpired(): boolean;
    toObject(): any;
  }

  export function createPrefundedBalance(
    identityId: string,
    balanceType: string,
    amount: number,
    purpose: string,
    lockDurationMs?: number,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function transferPrefundedBalance(
    fromIdentityId: string,
    toIdentityId: string,
    balanceType: string,
    amount: number,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function usePrefundedBalance(
    identityId: string,
    balanceType: string,
    amount: number,
    purpose: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function releasePrefundedBalance(
    identityId: string,
    balanceType: string,
    identityNonce: number,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function fetchPrefundedBalances(
    sdk: WasmSdk,
    identityId: string
  ): Promise<PrefundedBalance[]>;

  export function getPrefundedBalance(
    sdk: WasmSdk,
    identityId: string,
    balanceType: string
  ): Promise<PrefundedBalance | undefined>;

  export function checkPrefundedBalance(
    sdk: WasmSdk,
    identityId: string,
    balanceType: string,
    requiredAmount: number
  ): Promise<boolean>;

  export function fetchBalanceAllocations(
    sdk: WasmSdk,
    identityId: string,
    balanceType?: string,
    activeOnly: boolean
  ): Promise<BalanceAllocation[]>;

  export function monitorPrefundedBalance(
    sdk: WasmSdk,
    identityId: string,
    balanceType: string,
    callback: (balance: PrefundedBalance) => void,
    pollIntervalMs?: number
  ): Promise<{
    identityId: string;
    balanceType: string;
    interval: number;
    active: boolean;
  }>;
}