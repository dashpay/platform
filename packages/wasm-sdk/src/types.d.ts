/**
 * Enhanced TypeScript definitions for Dash WASM SDK
 * 
 * This file provides comprehensive type definitions for the JavaScript wrapper layer,
 * enhancing the auto-generated definitions with structured types, error handling,
 * and complete API coverage for all WASM SDK operations.
 */

// ===== CONFIGURATION TYPES =====

/**
 * Network configuration options
 */
export type NetworkType = 'mainnet' | 'testnet';

/**
 * Transport configuration for SDK communication
 */
export interface TransportConfig {
  /** The URL of the DAPI server */
  url: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Number of retries for failed requests */
  retries?: number;
}

/**
 * SDK request settings configuration
 */
export interface RequestSettings {
  /** Connection timeout in milliseconds */
  connect_timeout_ms?: number;
  /** Request timeout in milliseconds */
  timeout_ms?: number;
  /** Number of retries for failed requests */
  retries?: number;
  /** Whether to ban failed DAPI addresses */
  ban_failed_address?: boolean;
}

/**
 * Complete SDK configuration options
 */
export interface WasmSDKConfig {
  /** Network to connect to */
  network?: NetworkType;
  /** Transport configuration */
  transport?: TransportConfig;
  /** Whether to verify proofs by default */
  proofs?: boolean;
  /** Specific platform version to use */
  version?: number | null;
  /** Request settings */
  settings?: RequestSettings;
}

// ===== ERROR TYPES =====

/**
 * Base error class for all WASM SDK errors
 */
export class WasmSDKError extends Error {
  /** Error code for programmatic handling */
  readonly code: string;
  /** Additional context about the error */
  readonly context: Record<string, any>;
  
  constructor(message: string, code: string, context?: Record<string, any>);
}

/**
 * Error thrown during WASM initialization
 */
export class WasmInitializationError extends WasmSDKError {
  constructor(message: string, context?: Record<string, any>);
}

/**
 * Error thrown during WASM operations
 */
export class WasmOperationError extends WasmSDKError {
  /** The operation that failed */
  readonly operation: string;
  
  constructor(message: string, operation: string, context?: Record<string, any>);
}

// ===== IDENTITY TYPES =====

/**
 * Options for identity retrieval operations
 */
export interface IdentityFetchOptions {
  /** Whether to fetch with cryptographic proof */
  prove?: boolean;
  /** Whether to allow unproved results */
  allowUnproved?: boolean;
}

/**
 * Options for identity key retrieval
 */
export interface IdentityKeysOptions {
  /** Specific key IDs to retrieve */
  specificKeyIds?: number[];
  /** Purpose map for key search */
  searchPurposeMap?: string;
  /** Maximum number of keys to return */
  limit?: number;
  /** Offset for pagination */
  offset?: number;
  /** Whether to fetch with proof */
  prove?: boolean;
}

/**
 * Identity information returned from queries
 */
export interface IdentityInfo {
  /** Base58-encoded identity ID */
  id: string;
  /** Identity balance in credits */
  balance: number;
  /** Identity revision number */
  revision: number;
  /** Public keys associated with the identity */
  publicKeys: PublicKeyInfo[];
  /** Additional identity data */
  [key: string]: any;
}

/**
 * Public key information
 */
export interface PublicKeyInfo {
  /** Key ID */
  id: number;
  /** Key type */
  type: number;
  /** Key purpose */
  purpose: number;
  /** Security level */
  securityLevel: number;
  /** Key data */
  data: string;
  /** Whether key is read-only */
  readOnly?: boolean;
  /** Key signature */
  signature?: string;
}

/**
 * Identity balance information
 */
export interface IdentityBalance {
  /** Identity ID */
  identityId: string;
  /** Balance in credits */
  balance: number;
  /** Balance revision */
  revision?: number;
}

// ===== DPNS TYPES =====

/**
 * DPNS name resolution result
 */
export interface DpnsNameResolution {
  /** The resolved identity ID */
  identityId: string;
  /** The name that was resolved */
  name: string;
  /** Additional resolution data */
  [key: string]: any;
}

/**
 * DPNS username information
 */
export interface DpnsUsernameInfo {
  /** The username */
  username: string;
  /** Owner identity ID */
  ownerId: string;
  /** Registration timestamp */
  createdAt?: number;
  /** Last updated timestamp */
  updatedAt?: number;
  /** Additional username data */
  [key: string]: any;
}

// ===== DATA CONTRACT TYPES =====

/**
 * Data contract information
 */
export interface DataContractInfo {
  /** Contract ID */
  id: string;
  /** Owner identity ID */
  ownerId: string;
  /** Contract schema */
  schema: Record<string, any>;
  /** Contract version */
  version: number;
  /** Additional contract data */
  [key: string]: any;
}

/**
 * Options for data contract history retrieval
 */
export interface DataContractHistoryOptions {
  /** Maximum number of records to return */
  limit?: number;
  /** Offset for pagination */
  offset?: number;
  /** Start time in milliseconds */
  startAtMs?: number;
  /** Whether to fetch with proof */
  prove?: boolean;
}

/**
 * Data contract history information
 */
export interface DataContractHistory {
  /** Contract revisions */
  revisions: DataContractRevision[];
  /** Additional history data */
  [key: string]: any;
}

/**
 * Data contract revision information
 */
export interface DataContractRevision {
  /** Revision number */
  revision: number;
  /** Timestamp */
  timestamp: number;
  /** Contract data at this revision */
  contract: DataContractInfo;
}

// ===== TOKEN TYPES =====

/**
 * Token price information
 */
export interface TokenPriceInfo {
  /** Token ID */
  tokenId: string;
  /** Current price */
  currentPrice: number;
  /** Base price */
  basePrice: number;
  /** Additional pricing data */
  [key: string]: any;
}

/**
 * Token balance information
 */
export interface TokenBalance {
  /** Token ID */
  tokenId: string;
  /** Identity ID */
  identityId: string;
  /** Token balance */
  balance: string;
  /** Additional balance data */
  [key: string]: any;
}

/**
 * Token information
 */
export interface TokenInfo {
  /** Token ID */
  id: string;
  /** Token name */
  name?: string;
  /** Token symbol */
  symbol?: string;
  /** Total supply */
  totalSupply?: string;
  /** Additional token data */
  [key: string]: any;
}

// ===== WALLET TYPES =====

/**
 * Derived key information
 */
export interface DerivedKeyInfo {
  /** Private key in WIF format */
  privateKey: string;
  /** Public key in hex format */
  publicKey: string;
  /** Address */
  address: string;
  /** Derivation path used */
  path: string;
  /** Additional key data */
  [key: string]: any;
}

/**
 * DashPay contact key information
 */
export interface DashPayContactKey {
  /** Contact private key */
  privateKey: string;
  /** Contact public key */
  publicKey: string;
  /** Contact address */
  address: string;
  /** Sender identity ID */
  senderIdentityId: string;
  /** Receiver identity ID */
  receiverIdentityId: string;
  /** Additional contact data */
  [key: string]: any;
}

// ===== EPOCH TYPES =====

/**
 * Options for epoch information retrieval
 */
export interface EpochInfoOptions {
  /** Starting epoch number */
  startEpoch?: number;
  /** Number of epochs to retrieve */
  count?: number;
  /** Whether to sort in ascending order */
  ascending?: boolean;
  /** Whether to fetch with proof */
  prove?: boolean;
}

/**
 * Epoch information
 */
export interface EpochInfo {
  /** Epoch number */
  index: number;
  /** Start time */
  startTime: number;
  /** End time */
  endTime?: number;
  /** Epoch status */
  status: string;
  /** Additional epoch data */
  [key: string]: any;
}

// ===== DOCUMENT TYPES =====

/**
 * Document information
 */
export interface DocumentInfo {
  /** Document ID */
  id: string;
  /** Document type */
  type: string;
  /** Owner identity ID */
  ownerId: string;
  /** Document data */
  data: Record<string, any>;
  /** Document revision */
  revision: number;
  /** Creation timestamp */
  createdAt?: number;
  /** Last updated timestamp */
  updatedAt?: number;
}

// ===== QUERY TYPES =====

/**
 * Generic query options
 */
export interface QueryOptions {
  /** Where clause conditions */
  where?: any[];
  /** Order by specifications */
  orderBy?: any[];
  /** Maximum number of results */
  limit?: number;
  /** Starting point for results */
  startAfter?: string;
  /** Starting time for results */
  startAt?: string;
  /** Whether to fetch with proof */
  prove?: boolean;
}

/**
 * Query result with pagination
 */
export interface QueryResult<T = any> {
  /** Result data */
  data: T[];
  /** Pagination metadata */
  pagination?: {
    /** Total count */
    total?: number;
    /** Whether there are more results */
    hasMore?: boolean;
    /** Next page token */
    nextToken?: string;
  };
  /** Proof information if requested */
  proof?: any;
}

// ===== RESPONSE TYPES =====

/**
 * Response with proof information
 */
export interface ProvenResponse<T = any> {
  /** Response data */
  data: T;
  /** Cryptographic proof */
  proof: any;
  /** Proof metadata */
  metadata: any;
}

/**
 * State transition result
 */
export interface StateTransitionResult {
  /** Transaction ID */
  transactionId: string;
  /** Block height */
  blockHeight?: number;
  /** Result data */
  result: any;
  /** Additional result metadata */
  [key: string]: any;
}

// ===== MAIN SDK CLASS =====

/**
 * Main WASM SDK class providing JavaScript wrapper over raw WASM bindings
 * 
 * @example
 * ```typescript
 * import { WasmSDK } from '@dash/wasm-sdk';
 * 
 * const sdk = new WasmSDK({
 *   network: 'testnet',
 *   transport: {
 *     url: 'https://52.12.176.90:1443/',
 *     timeout: 30000
 *   },
 *   proofs: true
 * });
 * 
 * await sdk.initialize();
 * const identity = await sdk.getIdentity('your-identity-id');
 * ```
 */
export class WasmSDK {
  /**
   * Create a new WasmSDK instance
   * @param config - Configuration options for the SDK
   */
  constructor(config?: WasmSDKConfig);

  /**
   * Initialize the WASM SDK
   * @returns Promise that resolves when SDK is ready
   * @throws {WasmInitializationError} If initialization fails
   */
  initialize(): Promise<void>;

  /**
   * Check if the SDK is initialized
   * @returns True if SDK is ready for operations
   */
  isInitialized(): boolean;

  /**
   * Get the current SDK configuration
   * @returns Current configuration object
   */
  getConfig(): WasmSDKConfig;

  /**
   * Get the SDK version
   * @returns SDK version number
   * @throws {WasmOperationError} If SDK is not initialized
   */
  getVersion(): number;

  /**
   * Destroy the SDK and cleanup resources
   */
  destroy(): void;

  // ===== IDENTITY OPERATIONS =====

  /**
   * Fetch an identity by ID
   * @param identityId - Base58 encoded identity ID
   * @param options - Options for the fetch operation
   * @returns Promise resolving to identity information
   * @throws {WasmOperationError} If the operation fails
   * 
   * @example
   * ```typescript
   * const identity = await sdk.getIdentity('your-identity-id', {
   *   prove: true
   * });
   * console.log(`Identity balance: ${identity.balance}`);
   * ```
   */
  getIdentity(identityId: string, options?: IdentityFetchOptions): Promise<IdentityInfo>;

  /**
   * Get identity keys
   * @param identityId - Identity ID
   * @param keyRequestType - Type of key request ('all', 'specific', etc.)
   * @param options - Optional parameters
   * @returns Promise resolving to identity keys
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityKeys(
    identityId: string, 
    keyRequestType?: string, 
    options?: IdentityKeysOptions
  ): Promise<PublicKeyInfo[]>;

  /**
   * Get identity nonce
   * @param identityId - Identity ID
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to identity nonce
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityNonce(identityId: string, prove?: boolean): Promise<any>;

  /**
   * Get identity contract nonce
   * @param identityId - Identity ID
   * @param contractId - Contract ID
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to identity contract nonce
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityContractNonce(identityId: string, contractId: string, prove?: boolean): Promise<any>;

  /**
   * Get identity balance
   * @param identityId - Identity ID
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to identity balance
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityBalance(identityId: string, prove?: boolean): Promise<IdentityBalance>;

  /**
   * Get multiple identity balances
   * @param identityIds - Array of identity IDs
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to identity balances
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityBalances(identityIds: string[], prove?: boolean): Promise<IdentityBalance[]>;

  /**
   * Get identity balance and revision
   * @param identityId - Identity ID
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to identity balance and revision
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityBalanceAndRevision(identityId: string, prove?: boolean): Promise<IdentityBalance>;

  /**
   * Get identity by public key hash
   * @param publicKeyHash - Public key hash
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to identity information
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityByPublicKeyHash(publicKeyHash: string, prove?: boolean): Promise<IdentityInfo>;

  // ===== DPNS OPERATIONS =====

  /**
   * Check if a username is valid according to DPNS rules
   * @param username - Username to check
   * @returns Whether the username is valid
   * @throws {WasmOperationError} If the validation fails
   * 
   * @example
   * ```typescript
   * const isValid = sdk.isDpnsUsernameValid('myusername');
   * if (isValid) {
   *   console.log('Username is valid!');
   * }
   * ```
   */
  isDpnsUsernameValid(username: string): boolean;

  /**
   * Check if a username is contested
   * @param username - Username to check
   * @returns Whether the username is contested
   * @throws {WasmOperationError} If the check fails
   */
  isDpnsUsernameContested(username: string): boolean;

  /**
   * Convert string to homograph-safe characters
   * @param input - Input string
   * @returns Homograph-safe string
   * @throws {WasmOperationError} If the conversion fails
   */
  dpnsConvertToHomographSafe(input: string): string;

  /**
   * Check if a DPNS name is available
   * @param name - Name to check
   * @returns Promise resolving to availability status
   * @throws {WasmOperationError} If the operation fails
   */
  isDpnsNameAvailable(name: string): Promise<boolean>;

  /**
   * Resolve a DPNS name to identity information
   * @param name - Name to resolve
   * @returns Promise resolving to identity information
   * @throws {WasmOperationError} If the operation fails
   * 
   * @example
   * ```typescript
   * const resolution = await sdk.resolveDpnsName('alice');
   * console.log(`Alice's identity ID: ${resolution.identityId}`);
   * ```
   */
  resolveDpnsName(name: string): Promise<DpnsNameResolution>;

  /**
   * Get DPNS username by name
   * @param username - Username to fetch
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to username information
   * @throws {WasmOperationError} If the operation fails
   */
  getDpnsUsername(username: string, prove?: boolean): Promise<DpnsUsernameInfo>;

  /**
   * Register a DPNS name
   * @param name - Name to register
   * @param identityId - Identity ID
   * @param publicKeyId - Public key ID
   * @param privateKeyWif - Private key in WIF format
   * @param preorderCallback - Optional preorder callback
   * @returns Promise resolving to registration result
   * @throws {WasmOperationError} If the operation fails
   */
  registerDpnsName(
    name: string, 
    identityId: string, 
    publicKeyId: number, 
    privateKeyWif: string, 
    preorderCallback?: Function | null
  ): Promise<StateTransitionResult>;

  // ===== DATA CONTRACT OPERATIONS =====

  /**
   * Fetch a data contract by ID
   * @param contractId - Base58 encoded contract ID
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to data contract information
   * @throws {WasmOperationError} If the operation fails
   */
  getDataContract(contractId: string, prove?: boolean): Promise<DataContractInfo>;

  /**
   * Get data contract history
   * @param contractId - Contract ID
   * @param options - Options for history retrieval
   * @returns Promise resolving to contract history
   * @throws {WasmOperationError} If the operation fails
   */
  getDataContractHistory(contractId: string, options?: DataContractHistoryOptions): Promise<DataContractHistory>;

  /**
   * Get multiple data contracts
   * @param contractIds - Array of contract IDs
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to data contracts
   * @throws {WasmOperationError} If the operation fails
   */
  getDataContracts(contractIds: string[], prove?: boolean): Promise<DataContractInfo[]>;

  // ===== TOKEN OPERATIONS =====

  /**
   * Calculate token ID from contract ID and position
   * @param contractId - Data contract ID
   * @param tokenPosition - Token position in contract
   * @returns Token ID
   * @throws {WasmOperationError} If the calculation fails
   * 
   * @example
   * ```typescript
   * const tokenId = sdk.calculateTokenId(
   *   'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv', 
   *   0
   * );
   * ```
   */
  calculateTokenId(contractId: string, tokenPosition: number): string;

  /**
   * Get token price by contract
   * @param contractId - Contract ID
   * @param tokenPosition - Token position
   * @returns Promise resolving to token price information
   * @throws {WasmOperationError} If the operation fails
   */
  getTokenPriceByContract(contractId: string, tokenPosition: number): Promise<TokenPriceInfo>;

  /**
   * Get identity token balances
   * @param identityId - Identity ID
   * @param tokenIds - Array of token IDs
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to token balances
   * @throws {WasmOperationError} If the operation fails
   */
  getIdentityTokenBalances(identityId: string, tokenIds: string[], prove?: boolean): Promise<TokenBalance[]>;

  // ===== WALLET OPERATIONS =====

  /**
   * Derive key from seed with extended path
   * @param mnemonic - Mnemonic phrase
   * @param passphrase - Optional passphrase
   * @param path - Derivation path
   * @param network - Network ('mainnet' or 'testnet')
   * @returns Derived key information
   * @throws {WasmOperationError} If the derivation fails
   * 
   * @example
   * ```typescript
   * const keyInfo = sdk.deriveKey(
   *   'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about',
   *   null,
   *   "m/9'/5'/15'/0/0",
   *   'testnet'
   * );
   * console.log(`Derived address: ${keyInfo.address}`);
   * ```
   */
  deriveKey(mnemonic: string, passphrase: string | null, path: string, network?: string | null): DerivedKeyInfo;

  /**
   * Derive DashPay contact key
   * @param mnemonic - Mnemonic phrase
   * @param passphrase - Optional passphrase
   * @param senderIdentityId - Sender identity ID
   * @param receiverIdentityId - Receiver identity ID
   * @param account - Account number
   * @param addressIndex - Address index
   * @param network - Network
   * @returns Contact key information
   * @throws {WasmOperationError} If the derivation fails
   */
  deriveDashPayContactKey(
    mnemonic: string, 
    passphrase: string | null, 
    senderIdentityId: string, 
    receiverIdentityId: string, 
    account: number, 
    addressIndex: number, 
    network?: string | null
  ): DashPayContactKey;

  // ===== EPOCH OPERATIONS =====

  /**
   * Get epochs info
   * @param options - Options for epoch retrieval
   * @returns Promise resolving to epochs info
   * @throws {WasmOperationError} If the operation fails
   */
  getEpochsInfo(options?: EpochInfoOptions): Promise<EpochInfo[]>;

  /**
   * Get current epoch
   * @param prove - Whether to fetch with proof
   * @returns Promise resolving to current epoch
   * @throws {WasmOperationError} If the operation fails
   */
  getCurrentEpoch(prove?: boolean): Promise<EpochInfo>;

  // ===== IDENTITY CREATION OPERATIONS =====

  /**
   * Create a new identity
   * @param assetLockProof - Asset lock proof transaction hex
   * @param assetLockPrivateKey - Private key controlling the asset lock
   * @param publicKeysJson - JSON array of public keys
   * @returns Promise resolving to new identity
   * @throws {WasmOperationError} If the operation fails
   */
  createIdentity(assetLockProof: string, assetLockPrivateKey: string, publicKeysJson: string): Promise<IdentityInfo>;

  /**
   * Top up an existing identity
   * @param identityId - Identity ID to top up
   * @param assetLockProof - Asset lock proof transaction hex
   * @param assetLockPrivateKey - Private key controlling the asset lock
   * @returns Promise resolving to top up result
   * @throws {WasmOperationError} If the operation fails
   */
  topUpIdentity(identityId: string, assetLockProof: string, assetLockPrivateKey: string): Promise<StateTransitionResult>;
}

// ===== UTILITY FUNCTION TYPES =====

/**
 * Convert a string to homograph-safe characters
 * @param input - Input string
 * @returns Homograph-safe string
 */
export function convertToHomographSafe(input: string): string;

/**
 * Check if a username is valid according to DPNS rules
 * @param username - Username to check
 * @returns Whether the username is valid
 */
export function isValidDpnsUsername(username: string): boolean;

/**
 * Check if a username is contested
 * @param username - Username to check
 * @returns Whether the username is contested
 */
export function isContestedDpnsUsername(username: string): boolean;

/**
 * Calculate token ID from contract ID and position
 * @param contractId - Data contract ID
 * @param tokenPosition - Token position
 * @returns Token ID
 */
export function calculateTokenIdFromContract(contractId: string, tokenPosition: number): string;

/**
 * Derive key from seed with extended path
 * @param mnemonic - Mnemonic phrase
 * @param passphrase - Optional passphrase
 * @param path - Derivation path
 * @param network - Network
 * @returns Derived key information
 */
export function deriveKeyFromSeedWithExtendedPath(
  mnemonic: string, 
  passphrase: string | null | undefined, 
  path: string, 
  network: string
): DerivedKeyInfo;

/**
 * Derive DashPay contact key
 * @param mnemonic - Mnemonic phrase
 * @param passphrase - Optional passphrase
 * @param senderIdentityId - Sender identity ID
 * @param receiverIdentityId - Receiver identity ID
 * @param account - Account number
 * @param addressIndex - Address index
 * @param network - Network
 * @returns Contact key information
 */
export function deriveDashPayContactKey(
  mnemonic: string, 
  passphrase: string | null | undefined, 
  senderIdentityId: string, 
  receiverIdentityId: string, 
  account: number, 
  addressIndex: number, 
  network: string
): DashPayContactKey;

// ===== DEFAULT EXPORT =====

/**
 * Default export of the WasmSDK class
 */
export default WasmSDK;

// ===== TYPE GUARDS =====

/**
 * Type guard to check if an error is a WasmSDKError
 * @param error - Error to check
 * @returns True if error is a WasmSDKError
 */
export function isWasmSDKError(error: any): error is WasmSDKError;

/**
 * Type guard to check if an error is a WasmInitializationError
 * @param error - Error to check
 * @returns True if error is a WasmInitializationError
 */
export function isWasmInitializationError(error: any): error is WasmInitializationError;

/**
 * Type guard to check if an error is a WasmOperationError
 * @param error - Error to check
 * @returns True if error is a WasmOperationError
 */
export function isWasmOperationError(error: any): error is WasmOperationError;

// ===== CONSTANTS =====

/**
 * Available network types
 */
export const NETWORK_TYPES: readonly NetworkType[];

/**
 * Default configuration values
 */
export const DEFAULT_CONFIG: Required<WasmSDKConfig>;

/**
 * SDK version information
 */
export const SDK_VERSION: {
  readonly MAJOR: number;
  readonly MINOR: number;
  readonly PATCH: number;
  readonly VERSION_STRING: string;
};