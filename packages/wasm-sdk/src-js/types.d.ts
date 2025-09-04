/**
 * TypeScript definitions for Dash Platform WASM SDK JavaScript Wrapper
 * Provides complete type coverage for the modern wrapper API
 */

// ========== Configuration Types ==========

/**
 * Transport configuration options
 */
export interface TransportConfig {
    /** Primary endpoint URL (HTTPS required) */
    url?: string;
    /** Array of endpoint URLs for failover */
    urls?: string[];
    /** Request timeout in milliseconds (1000-300000) */
    timeout?: number;
    /** Number of retry attempts (0-10) */
    retries?: number;
    /** Delay between retries in milliseconds (100-10000) */
    retryDelay?: number;
    /** Keep connection alive */
    keepAlive?: boolean;
}

/**
 * Main WASM SDK configuration
 */
export interface WasmSDKConfig {
    /** Network to connect to */
    network?: 'testnet' | 'mainnet';
    /** Transport configuration */
    transport?: TransportConfig;
    /** Enable proof verification */
    proofs?: boolean;
    /** Enable debug logging */
    debug?: boolean;
}

/**
 * Complete resolved configuration with defaults applied
 */
export interface ResolvedConfig {
    network: 'testnet' | 'mainnet';
    transport: Required<TransportConfig & { urls: string[] }>;
    proofs: boolean;
    debug: boolean;
}

// ========== Error Types ==========

/**
 * Base WASM SDK error class
 */
export class WasmSDKError extends Error {
    readonly name: 'WasmSDKError';
    readonly code: string;
    readonly context: Record<string, any>;
    readonly timestamp: string;
    
    constructor(message: string, code?: string, context?: Record<string, any>);
    
    /** Convert error to JSON for logging/debugging */
    toJSON(): {
        name: string;
        message: string;
        code: string;
        context: Record<string, any>;
        timestamp: string;
        stack?: string;
    };
}

/**
 * Error thrown during WASM initialization
 */
export class WasmInitializationError extends WasmSDKError {
    readonly name: 'WasmInitializationError';
    constructor(message: string, context?: Record<string, any>);
}

/**
 * Error thrown during WASM operations
 */
export class WasmOperationError extends WasmSDKError {
    readonly name: 'WasmOperationError';
    readonly operation: string;
    constructor(message: string, operation: string, context?: Record<string, any>);
}

/**
 * Error thrown for invalid configuration
 */
export class WasmConfigurationError extends WasmSDKError {
    readonly name: 'WasmConfigurationError';
    readonly field: string;
    readonly value: any;
    constructor(message: string, field: string, value: any, context?: Record<string, any>);
}

/**
 * Error thrown for network/transport issues
 */
export class WasmTransportError extends WasmSDKError {
    readonly name: 'WasmTransportError';
    readonly endpoint: string;
    constructor(message: string, endpoint: string, context?: Record<string, any>);
}

// ========== Platform Data Types ==========

/**
 * Base identifier type (32-byte hash as base58 string)
 */
export type Identifier = string;

/**
 * Identity public key
 */
export interface IdentityPublicKey {
    id: number;
    type: number;
    data: Uint8Array;
    purpose: number;
    securityLevel: number;
    contractBounds?: {
        contractId: Identifier;
    };
    disabledAt?: number;
}

/**
 * Platform identity
 */
export interface Identity {
    id: Identifier;
    publicKeys: IdentityPublicKey[];
    balance: number;
    revision: number;
    protocolVersion: number;
}

/**
 * Data contract document property
 */
export interface DocumentProperty {
    type: string;
    description?: string;
    format?: string;
    pattern?: string;
    minimum?: number;
    maximum?: number;
    minLength?: number;
    maxLength?: number;
    items?: DocumentProperty;
    properties?: Record<string, DocumentProperty>;
    required?: string[];
    additionalProperties?: boolean;
    enum?: any[];
}

/**
 * Data contract document type definition
 */
export interface DocumentType {
    type: 'object';
    properties: Record<string, DocumentProperty>;
    required?: string[];
    additionalProperties: boolean;
    indices?: Array<{
        name?: string;
        properties: Array<{ [propertyName: string]: 'asc' | 'desc' }>;
        unique?: boolean;
    }>;
}

/**
 * Platform data contract
 */
export interface DataContract {
    id: Identifier;
    ownerId: Identifier;
    version: number;
    schema: string;
    documents: Record<string, DocumentType>;
    protocolVersion: number;
}

/**
 * Platform document
 */
export interface Document {
    id: Identifier;
    ownerId: Identifier;
    dataContractId: Identifier;
    type: string;
    data: Record<string, any>;
    revision: number;
    createdAt: number;
    updatedAt: number;
}

// ========== Query Types ==========

/**
 * Where clause condition
 */
export type WhereCondition = [string, string, any];

/**
 * Order by clause
 */
export type OrderByClause = [string, 'asc' | 'desc'];

/**
 * Document query options
 */
export interface DocumentQueryOptions {
    /** Where conditions for filtering */
    where?: WhereCondition[];
    /** Order by clauses for sorting */
    orderBy?: OrderByClause[];
    /** Maximum number of results to return */
    limit?: number;
    /** Number of results to skip */
    offset?: number;
}

// ========== State Transition Types ==========

/**
 * Identity creation data
 */
export interface IdentityCreationData {
    publicKeys: Omit<IdentityPublicKey, 'id'>[];
}

/**
 * State transition result
 */
export interface StateTransitionResult {
    hash: string;
    height: number;
    coreChainLockHeight: number;
    timestamp: number;
}

// ========== Platform Information Types ==========

/**
 * Platform version information
 */
export interface PlatformVersion {
    version: string;
    protocolVersion: number;
    networkId: number;
}

/**
 * Network status information
 */
export interface NetworkStatus {
    coreVersion: string;
    protocolVersion: number;
    blocks: number;
    timeOffset: number;
    connections: number;
    network: string;
    relayFee: number;
    difficulty: number;
}

// ========== Resource Management Types ==========

/**
 * Resource statistics
 */
export interface ResourceStats {
    totalResources: number;
    byType: Record<string, number>;
    oldestResource: {
        id: string;
        type: string;
        createdAt: number;
    } | null;
    newestResource: {
        id: string;
        type: string;
        createdAt: number;
    } | null;
    averageAge: number;
    totalAge: number;
}

/**
 * Resource cleanup options
 */
export interface ResourceCleanupOptions {
    /** Maximum age in milliseconds before cleanup */
    maxAge?: number;
    /** Maximum idle time in milliseconds before cleanup */
    maxIdleTime?: number;
}

// ========== Utility Types ==========

/**
 * Configuration utilities
 */
export interface ConfigUtils {
    /** Create testnet configuration with optional overrides */
    createTestnetConfig(overrides?: Partial<WasmSDKConfig>): WasmSDKConfig;
    /** Create mainnet configuration with optional overrides */
    createMainnetConfig(overrides?: Partial<WasmSDKConfig>): WasmSDKConfig;
    /** Create configuration with custom endpoint */
    createCustomEndpointConfig(url: string, overrides?: Partial<WasmSDKConfig>): WasmSDKConfig;
}

/**
 * Resource utilities
 */
export interface ResourceUtils {
    /** Check if an object looks like a WASM resource */
    isWasmResource(obj: any): boolean;
    /** Create a cleanup function for common WASM objects */
    createCleanupFunction(method?: 'free' | 'destroy' | 'dispose'): (resource: any) => void;
    /** Auto-detect and create appropriate cleanup function */
    detectCleanupFunction(resource: any): ((resource: any) => void) | null;
}

// ========== Main SDK Class ==========

/**
 * Modern Dash Platform WASM SDK
 * 
 * @example
 * ```typescript
 * import { WasmSDK } from '@dashevo/dash-wasm-sdk';
 * 
 * const sdk = new WasmSDK({
 *   network: 'testnet',
 *   transport: {
 *     timeout: 30000
 *   },
 *   proofs: true
 * });
 * 
 * await sdk.initialize();
 * 
 * const identity = await sdk.getIdentity(identityId);
 * ```
 */
export declare class WasmSDK {
    /**
     * Create a new WASM SDK instance
     * @param config - SDK configuration
     */
    constructor(config?: WasmSDKConfig);

    // ========== Initialization ==========

    /**
     * Initialize the WASM SDK
     * Must be called before using any SDK operations
     * 
     * @throws {WasmInitializationError} If initialization fails
     * @example
     * ```typescript
     * const sdk = new WasmSDK({ network: 'testnet' });
     * await sdk.initialize();
     * ```
     */
    initialize(): Promise<void>;

    /**
     * Check if SDK is initialized
     */
    isInitialized(): boolean;

    // ========== Configuration ==========

    /**
     * Get current configuration
     */
    getConfig(): ResolvedConfig;

    /**
     * Get current network
     */
    getNetwork(): 'testnet' | 'mainnet';

    /**
     * Get current primary endpoint
     */
    getCurrentEndpoint(): string;

    // ========== Query Operations ==========

    /**
     * Get identity by ID
     * 
     * @param identityId - Identity identifier
     * @returns Identity or null if not found
     * @throws {WasmOperationError} If operation fails
     * @example
     * ```typescript
     * const identity = await sdk.getIdentity('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
     * if (identity) {
     *   console.log('Identity balance:', identity.balance);
     * }
     * ```
     */
    getIdentity(identityId: Identifier): Promise<Identity | null>;

    /**
     * Get multiple identities by IDs
     * 
     * @param identityIds - Array of identity identifiers
     * @returns Array of identities (may contain nulls for not found)
     * @throws {WasmOperationError} If operation fails
     */
    getIdentities(identityIds: Identifier[]): Promise<(Identity | null)[]>;

    /**
     * Get data contract by ID
     * 
     * @param contractId - Data contract identifier
     * @returns Data contract or null if not found
     * @throws {WasmOperationError} If operation fails
     * @example
     * ```typescript
     * const contract = await sdk.getDataContract(contractId);
     * if (contract) {
     *   console.log('Available document types:', Object.keys(contract.documents));
     * }
     * ```
     */
    getDataContract(contractId: Identifier): Promise<DataContract | null>;

    /**
     * Get documents by contract and type
     * 
     * @param contractId - Data contract identifier
     * @param documentType - Document type name
     * @param options - Query options (where, orderBy, limit, offset)
     * @returns Array of matching documents
     * @throws {WasmOperationError} If operation fails
     * @example
     * ```typescript
     * const documents = await sdk.getDocuments(contractId, 'note', {
     *   where: [['ownerId', '=', identityId]],
     *   orderBy: [['createdAt', 'desc']],
     *   limit: 10
     * });
     * ```
     */
    getDocuments(
        contractId: Identifier, 
        documentType: string, 
        options?: DocumentQueryOptions
    ): Promise<Document[]>;

    /**
     * Get specific document by ID
     * 
     * @param contractId - Data contract identifier
     * @param documentType - Document type name
     * @param documentId - Document identifier
     * @returns Document or null if not found
     * @throws {WasmOperationError} If operation fails
     */
    getDocument(
        contractId: Identifier, 
        documentType: string, 
        documentId: Identifier
    ): Promise<Document | null>;

    // ========== State Transition Operations ==========

    /**
     * Create and submit an identity creation state transition
     * 
     * @param identityData - Identity creation data
     * @param privateKey - Private key for signing (hex string)
     * @returns State transition result
     * @throws {WasmOperationError} If operation fails
     * @example
     * ```typescript
     * const result = await sdk.createIdentity(
     *   { publicKeys: [/* public key data */] },
     *   privateKeyHex
     * );
     * console.log('Identity created at height:', result.height);
     * ```
     */
    createIdentity(identityData: IdentityCreationData, privateKey: string): Promise<StateTransitionResult>;

    /**
     * Create and submit a data contract state transition
     * 
     * @param contractData - Data contract definition
     * @param identityId - Owner identity identifier
     * @param privateKey - Private key for signing (hex string)
     * @returns State transition result
     * @throws {WasmOperationError} If operation fails
     */
    createDataContract(
        contractData: Omit<DataContract, 'id' | 'ownerId' | 'protocolVersion'>, 
        identityId: Identifier, 
        privateKey: string
    ): Promise<StateTransitionResult>;

    /**
     * Create and submit a document creation state transition
     * 
     * @param documentData - Document data
     * @param contractId - Data contract identifier
     * @param documentType - Document type name
     * @param identityId - Owner identity identifier
     * @param privateKey - Private key for signing (hex string)
     * @returns State transition result
     * @throws {WasmOperationError} If operation fails
     */
    createDocument(
        documentData: Record<string, any>,
        contractId: Identifier,
        documentType: string,
        identityId: Identifier,
        privateKey: string
    ): Promise<StateTransitionResult>;

    // ========== Utility Operations ==========

    /**
     * Get platform version information
     * 
     * @returns Platform version details
     * @throws {WasmOperationError} If operation fails
     */
    getPlatformVersion(): Promise<PlatformVersion>;

    /**
     * Get network status information
     * 
     * @returns Network status details
     * @throws {WasmOperationError} If operation fails
     */
    getNetworkStatus(): Promise<NetworkStatus>;

    /**
     * Validate a document against its data contract
     * 
     * @param document - Document to validate
     * @param dataContract - Data contract to validate against
     * @returns True if document is valid
     * @throws {WasmOperationError} If validation fails
     */
    validateDocument(document: Document, dataContract: DataContract): Promise<boolean>;

    // ========== Resource Management ==========

    /**
     * Get resource manager statistics
     * 
     * @returns Resource usage statistics
     */
    getResourceStats(): ResourceStats;

    /**
     * Clean up stale resources
     * 
     * @param options - Cleanup options
     * @returns Number of resources cleaned up
     */
    cleanupResources(options?: ResourceCleanupOptions): number;

    /**
     * Destroy the SDK and clean up all resources
     * Call this when you're done using the SDK to prevent memory leaks
     * 
     * @throws {WasmOperationError} If cleanup fails
     * @example
     * ```typescript
     * // Always destroy when done
     * try {
     *   // Use SDK...
     * } finally {
     *   await sdk.destroy();
     * }
     * ```
     */
    destroy(): Promise<void>;
}

// ========== Module Exports ==========

/**
 * Default export - WasmSDK class
 */
declare const WasmSDK: typeof WasmSDK;
export default WasmSDK;

/**
 * Named exports
 */
export {
    // Configuration utilities
    ConfigUtils,
    
    // Resource utilities  
    ResourceUtils,
    
    // All error classes
    WasmSDKError,
    WasmInitializationError,
    WasmOperationError,
    WasmConfigurationError,
    WasmTransportError
};

// ========== Global Types ==========

/**
 * Global module declaration for package consumers
 */
declare module '@dashevo/dash-wasm-sdk' {
    export {
        WasmSDK as default,
        WasmSDK,
        WasmSDKConfig,
        TransportConfig,
        ResolvedConfig,
        Identity,
        DataContract,
        Document,
        DocumentQueryOptions,
        WhereCondition,
        OrderByClause,
        IdentityCreationData,
        StateTransitionResult,
        PlatformVersion,
        NetworkStatus,
        ResourceStats,
        ResourceCleanupOptions,
        ConfigUtils,
        ResourceUtils,
        WasmSDKError,
        WasmInitializationError,
        WasmOperationError,
        WasmConfigurationError,
        WasmTransportError
    };
}