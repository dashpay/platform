/**
 * Complete WASM SDK TypeScript Definitions
 * 
 * This file provides comprehensive TypeScript type definitions for the Dash Platform WASM SDK.
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
  export abstract class ContextProvider {
    /**
     * Get current block height
     */
    abstract getBlockHeight(): Promise<number>;

    /**
     * Get current core chain locked height
     */
    abstract getCoreChainLockedHeight(): Promise<number>;

    /**
     * Get current time in milliseconds
     */
    abstract getTimeMillis(): Promise<number>;
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

  // Identity Management
  export interface Identity {
    readonly id: string;
    readonly revision: number;
    readonly balance: number;
    readonly publicKeys: PublicKey[];
  }

  export interface PublicKey {
    readonly id: number;
    readonly type: number;
    readonly purpose: number;
    readonly securityLevel: number;
    readonly data: Uint8Array;
    readonly readOnly: boolean;
    readonly disabledAt?: number;
  }

  export function fetchIdentity(
    sdk: WasmSdk,
    identityId: string,
    options?: FetchOptions
  ): Promise<Identity>;

  export function fetchIdentityUnproved(
    sdk: WasmSdk,
    identityId: string,
    options?: FetchOptions
  ): Promise<Identity>;

  export function createIdentity(
    assetLockProof: Uint8Array,
    publicKeys: PublicKey[]
  ): Uint8Array;

  export function updateIdentity(
    identityId: string,
    revision: bigint,
    addPublicKeys: PublicKey[],
    disablePublicKeys: number[],
    publicKeysDisabledAt?: bigint,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function topupIdentity(
    identityId: string,
    assetLockProof: Uint8Array
  ): Uint8Array;

  // Data Contracts
  export interface DataContract {
    readonly id: string;
    readonly ownerId: string;
    readonly schema: any;
    readonly version: number;
    readonly documentSchemas: { [key: string]: any };
  }

  export function fetchDataContract(
    sdk: WasmSdk,
    contractId: string,
    options?: FetchOptions
  ): Promise<DataContract>;

  export function createDataContract(
    ownerId: string,
    contractDefinition: any,
    identityNonce: bigint,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function updateDataContract(
    contractId: string,
    ownerId: string,
    contractDefinition: any,
    identityContractNonce: bigint,
    signaturePublicKeyId: number
  ): Uint8Array;

  // Documents
  export interface Document {
    readonly id: string;
    readonly ownerId: string;
    readonly dataContractId: string;
    readonly revision: number;
    readonly data: any;
    readonly createdAt: number;
    readonly updatedAt: number;
  }

  export function fetchDocuments(
    sdk: WasmSdk,
    contractId: string,
    documentType: string,
    whereClause: any,
    options?: FetchOptions & {
      orderBy?: any;
      limit?: number;
      startAt?: Uint8Array;
    }
  ): Promise<Document[]>;

  // State Transitions
  export interface BroadcastOptions {
    retries?: number;
    timeout?: number;
  }

  export interface BroadcastResponse {
    success: boolean;
    metadata?: any;
    error?: string;
  }

  export function broadcastStateTransition(
    sdk: WasmSdk,
    stateTransition: Uint8Array,
    options?: BroadcastOptions
  ): Promise<BroadcastResponse>;

  // Epoch Management
  export class Epoch {
    readonly index: number;
    readonly startBlockHeight: number;
    readonly startBlockCoreHeight: number;
    readonly startTimeMs: number;
    readonly feeMultiplier: number;
    
    toObject(): any;
  }

  export class Evonode {
    readonly proTxHash: Uint8Array;
    readonly ownerAddress: string;
    readonly votingAddress: string;
    readonly isHPMN: boolean;
    readonly platformP2PPort: number;
    readonly platformHTTPPort: number;
    readonly nodeIP: string;
    
    toObject(): any;
  }

  export function getCurrentEpoch(sdk: WasmSdk): Promise<Epoch>;
  export function getEpochByIndex(sdk: WasmSdk, index: number): Promise<Epoch>;
  export function getCurrentEvonodes(sdk: WasmSdk): Promise<any>;
  export function getEvonodesForEpoch(sdk: WasmSdk, epochIndex: number): Promise<any>;
  export function getEvonodeByProTxHash(sdk: WasmSdk, proTxHash: Uint8Array): Promise<Evonode>;
  export function getCurrentQuorum(sdk: WasmSdk): Promise<any>;
  export function calculateEpochBlocks(network: string): number;
  export function estimateNextEpochTime(sdk: WasmSdk, currentBlockHeight: number): Promise<any>;
  export function getEpochForBlockHeight(sdk: WasmSdk, blockHeight: number): Promise<Epoch>;
  export function getValidatorSetChanges(sdk: WasmSdk, fromEpoch: number, toEpoch: number): Promise<any>;
  export function getEpochStats(sdk: WasmSdk, epochIndex: number): Promise<any>;

  // Nonce Management
  export class NonceOptions {
    constructor();
    setCached(cached: boolean): void;
    setProve(prove: boolean): void;
  }

  export class NonceResponse {
    readonly nonce: number;
    readonly metadata: any;
  }

  export function checkIdentityNonceCache(identityId: string): number | null;
  export function updateIdentityNonceCache(identityId: string, nonce: number): void;
  export function checkIdentityContractNonceCache(identityId: string, contractId: string): number | null;
  export function updateIdentityContractNonceCache(identityId: string, contractId: string, nonce: number): void;
  export function incrementIdentityNonceCache(identityId: string, increment?: number): number;
  export function incrementIdentityContractNonceCache(identityId: string, contractId: string, increment?: number): number;
  export function clearIdentityNonceCache(): void;
  export function clearIdentityContractNonceCache(): void;

  // Cache Management
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
    setMaxSizes(
      contractsMax: number,
      identitiesMax: number,
      documentsMax: number,
      tokensMax: number,
      quorumKeysMax: number,
      metadataMax: number
    ): void;
    cacheContract(contractId: string, contractData: Uint8Array): void;
    getCachedContract(contractId: string): Uint8Array | undefined;
    cacheIdentity(identityId: string, identityData: Uint8Array): void;
    getCachedIdentity(identityId: string): Uint8Array | undefined;
    cacheDocument(documentKey: string, documentData: Uint8Array): void;
    getCachedDocument(documentKey: string): Uint8Array | undefined;
    clearAll(): void;
    clearCache(cacheType: string): void;
    cleanupExpired(): void;
    getStats(): any;
    startAutoCleanup(intervalMs: number): void;
    stopAutoCleanup(): void;
  }

  export class ContractCache {
    constructor(config?: ContractCacheConfig);
    cacheContract(contractBytes: Uint8Array): string;
    getCachedContract(contractId: string): Uint8Array | null;
    isContractCached(contractId: string): boolean;
    removeContract(contractId: string): boolean;
    clearCache(): void;
    getCacheStats(): any;
    getContractMetadata(contractId: string): any;
    getPreloadSuggestions(): string[];
  }

  export class ContractCacheConfig {
    constructor();
    setMaxContracts(max: number): void;
    setTtlMs(ttl: number): void;
    setCacheHistory(cache: boolean): void;
    setMaxVersionsPerContract(max: number): void;
    setEnablePreloading(enable: boolean): void;
  }

  // WebSocket Subscriptions
  export class SubscriptionHandle {
    readonly id: string;
    close(): void;
    readonly isActive: boolean;
  }

  export class SubscriptionHandleV2 {
    readonly id: string;
    close(): void;
    readonly isActive: boolean;
  }

  export class SubscriptionOptions {
    constructor();
    autoReconnect: boolean;
    maxReconnectAttempts: number;
    reconnectDelayMs: number;
    connectionTimeoutMs: number;
  }

  export function subscribeToIdentityBalanceUpdates(
    identityId: string,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandle;

  export function subscribeToDataContractUpdates(
    contractId: string,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandle;

  export function subscribeToDocumentUpdates(
    contractId: string,
    documentType: string,
    whereClause: any,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandle;

  export function subscribeToBlockHeaders(
    callback: Function,
    endpoint?: string
  ): SubscriptionHandle;

  export function subscribeToStateTransitionResults(
    stateTransitionHash: string,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandle;

  // V2 Subscriptions with better memory management
  export function subscribeToIdentityBalanceUpdatesV2(
    identityId: string,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandleV2;

  export function subscribeToDataContractUpdatesV2(
    contractId: string,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandleV2;

  export function subscribeToDocumentUpdatesV2(
    contractId: string,
    documentType: string,
    whereClause: any,
    callback: Function,
    endpoint?: string
  ): SubscriptionHandleV2;

  export function subscribeWithHandlersV2(
    subscriptionType: string,
    params: any,
    onMessage: Function,
    onError?: Function,
    onClose?: Function,
    endpoint?: string
  ): SubscriptionHandleV2;

  export function cleanupAllSubscriptions(): void;
  export function getActiveSubscriptionCount(): number;

  // Request Settings
  export class RequestSettings {
    constructor();
    setMaxRetries(retries: number): void;
    setInitialRetryDelay(delayMs: number): void;
    setMaxRetryDelay(delayMs: number): void;
    setBackoffMultiplier(multiplier: number): void;
    setTimeout(timeoutMs: number): void;
    setUseExponentialBackoff(use: boolean): void;
    setRetryOnTimeout(retry: boolean): void;
    setRetryOnNetworkError(retry: boolean): void;
    setCustomHeaders(headers: any): void;
    getRetryDelay(attempt: number): number;
    toObject(): any;
  }

  export class RetryHandler {
    constructor(settings: RequestSettings);
    shouldRetry(error: any): boolean;
    getNextRetryDelay(): number;
    incrementAttempt(): void;
    readonly currentAttempt: number;
    getElapsedTime(): number;
    isTimeoutExceeded(): boolean;
  }

  export class RequestSettingsBuilder {
    constructor();
    withMaxRetries(retries: number): RequestSettingsBuilder;
    withTimeout(timeoutMs: number): RequestSettingsBuilder;
    withInitialRetryDelay(delayMs: number): RequestSettingsBuilder;
    withBackoffMultiplier(multiplier: number): RequestSettingsBuilder;
    withoutRetries(): RequestSettingsBuilder;
    build(): RequestSettings;
  }

  export function executeWithRetry(
    requestFn: Function,
    settings: RequestSettings
  ): Promise<any>;

  // Optimization
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

  export class BatchOptimizer {
    constructor();
    setBatchSize(size: number): void;
    setMaxConcurrent(max: number): void;
    getOptimalBatchCount(totalItems: number): number;
    getBatchBoundaries(totalItems: number, batchIndex: number): any;
  }

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

  export function optimizeUint8Array(data: Uint8Array): Uint8Array;
  export function internString(s: string): string;
  export function initStringCache(): void;
  export function clearStringCache(): void;
  export function getOptimizationRecommendations(): string[];

  // Signing
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

  // Asset Lock
  export class AssetLockProof {
    static createInstant(
      transaction: Uint8Array,
      outputIndex: number,
      instantLock: Uint8Array
    ): AssetLockProof;
    
    static createChain(
      transaction: Uint8Array,
      outputIndex: number
    ): AssetLockProof;
    
    static fromBytes(bytes: Uint8Array): AssetLockProof;
    
    readonly proofType: string;
    readonly transaction: Uint8Array;
    readonly outputIndex: number;
    readonly instantLock?: Uint8Array;
    
    toBytes(): Uint8Array;
    toObject(): any;
  }

  export function validateAssetLockProof(
    proof: AssetLockProof,
    identityId?: string
  ): boolean;

  export function calculateCreditsFromProof(
    proof: AssetLockProof,
    duffsPerCredit?: number
  ): number;

  // Token Management
  export interface TokenOptions {
    prove?: boolean;
    retries?: number;
    timeout?: number;
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

  export function getTokenBalance(
    sdk: WasmSdk,
    tokenId: string,
    identityId: string,
    options?: TokenOptions
  ): Promise<{
    balance: number;
    frozen: boolean;
  }>;

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

  // Utility Functions
  export function createDocumentCacheKey(
    contractId: string,
    documentType: string,
    documentId: string
  ): string;

  export function createDocumentQueryCacheKey(
    contractId: string,
    documentType: string,
    whereClause: string,
    orderBy: string,
    limit: number,
    offset: number
  ): string;

  export function createIdentityByKeyCacheKey(publicKeyHash: Uint8Array): string;
  export function createTokenBalanceCacheKey(tokenId: string, identityId: string): string;

  // DPP Types
  export class IdentityWasm {
    constructor(platformVersion: number);
    readonly id: string;
    readonly revision: number;
    setPublicKeys(publicKeys: any[]): number;
    toObject(): any;
    toJSON(): any;
    toBuffer(): Uint8Array;
  }

  export class DataContractWasm {
    constructor(rawDataContract: any, platformVersion: number);
    readonly id: string;
    readonly ownerId: string;
    readonly version: number;
    readonly schema: string;
    getSchemaDefs(): any;
    getDocumentSchemas(): any;
    toObject(): any;
    toJSON(): any;
    toBuffer(): Uint8Array;
    setVersion(version: number): void;
    getBinaryProperties(documentType: string): any;
    getDocumentKeeps(): any;
  }

  // Group Actions
  export interface GroupStateTransitionInfo {
    groupContractPosition: number;
    actionId: string;
    actionIsProposer: boolean;
  }

  export function createGroupStateTransitionInfo(
    groupContractPosition: number,
    actionId?: string,
    isProposer?: boolean
  ): GroupStateTransitionInfo;

  export function createGroupProposal(
    dataContractId: string,
    documentTypePosition: number,
    actionName: string,
    dataJson: any,
    proposerId: string,
    info: GroupStateTransitionInfo,
    signaturePublicKeyId: number
  ): Uint8Array;

  export function createGroupAction(
    dataContractId: string,
    documentTypePosition: number,
    actionName: string,
    dataJson: any,
    actorId: string,
    info: GroupStateTransitionInfo,
    signaturePublicKeyId: number
  ): Uint8Array;

  // Verification
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

  // Metadata
  export class Metadata {
    constructor(
      height: number,
      coreChainLockedHeight: number,
      epoch: number,
      timeMs: number,
      protocolVersion: number,
      chainId: string
    );
    
    readonly height: number;
    readonly coreChainLockedHeight: number;
    readonly epoch: number;
    readonly timeMs: number;
    readonly protocolVersion: number;
    readonly chainId: string;
    
    toObject(): any;
  }

  export function verifyMetadata(
    metadata: Metadata,
    currentHeight: number,
    currentTimeMs?: number,
    config: any
  ): any;

  // BLS Operations
  export function verifyBLSSignature(
    signature: Uint8Array,
    message: Uint8Array,
    publicKey: Uint8Array
  ): boolean;

  export function aggregateBLSSignatures(signatures: Uint8Array[]): Uint8Array;
  export function aggregateBLSPublicKeys(publicKeys: Uint8Array[]): Uint8Array;

  // BIP39
  export function generateMnemonic(wordCount?: number): string;
  export function validateMnemonic(mnemonic: string): boolean;
  export function mnemonicToSeed(mnemonic: string, passphrase?: string): Uint8Array;
}