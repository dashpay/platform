/**
 * Dash WASM SDK - JavaScript Wrapper Layer
 * 
 * Provides a clean, modern JavaScript API over the raw WASM bindings
 * with proper error handling, resource management, and Promise-based operations.
 */

import init, { 
  WasmSdkBuilder, 
  WasmContext,
  // Identity operations
  identity_fetch,
  identity_fetch_with_proof_info,
  identity_fetch_unproved,
  get_identity_keys,
  get_identity_nonce,
  get_identity_nonce_with_proof_info,
  get_identity_contract_nonce,
  get_identity_contract_nonce_with_proof_info,
  get_identity_balance,
  get_identities_balances,
  get_identity_balance_and_revision,
  get_identity_by_public_key_hash,
  get_identities_contract_keys,
  get_identity_by_non_unique_public_key_hash,
  get_identity_token_balances,
  // DPNS operations
  dpns_convert_to_homograph_safe,
  dpns_is_valid_username,
  dpns_is_contested_username,
  dpns_register_name,
  dpns_is_name_available,
  dpns_resolve_name,
  get_dpns_username_by_name,
  // Data contract operations
  data_contract_fetch,
  data_contract_fetch_with_proof_info,
  get_data_contract_history,
  get_data_contracts,
  // Token operations
  calculate_token_id_from_contract,
  get_token_price_by_contract,
  get_identities_token_balances,
  get_identity_token_infos,
  get_token_statuses,
  get_token_direct_purchase_prices,
  get_token_contract_info,
  get_token_total_supply,
  // Wallet operations
  derive_key_from_seed_with_extended_path,
  derive_dashpay_contact_key,
  // Epoch operations
  get_epochs_info,
  get_current_epoch,
  get_finalized_epoch_infos
} from '../pkg/dash_wasm_sdk.js';

/**
 * Custom error types for structured error handling
 */
export class WasmSDKError extends Error {
  constructor(message, code, context = {}) {
    super(message);
    this.name = 'WasmSDKError';
    this.code = code;
    this.context = context;
  }
}

export class WasmInitializationError extends WasmSDKError {
  constructor(message, context = {}) {
    super(message, 'WASM_INIT_ERROR', context);
    this.name = 'WasmInitializationError';
  }
}

export class WasmOperationError extends WasmSDKError {
  constructor(message, operation, context = {}) {
    super(message, 'WASM_OPERATION_ERROR', { operation, ...context });
    this.name = 'WasmOperationError';
  }
}

/**
 * Configuration validator and defaults
 */
class ConfigManager {
  constructor(config = {}) {
    this.config = this._validateConfig(config);
  }

  _validateConfig(config) {
    const defaults = {
      network: 'testnet',
      transport: {
        url: 'https://52.12.176.90:1443/',
        timeout: 30000,
        retries: 3
      },
      proofs: true,
      version: null, // Use latest
      settings: {
        connect_timeout_ms: 10000,
        timeout_ms: 30000,
        retries: 3,
        ban_failed_address: true
      }
    };

    // Merge with defaults
    const merged = {
      ...defaults,
      ...config,
      transport: { ...defaults.transport, ...(config.transport || {}) },
      settings: { ...defaults.settings, ...(config.settings || {}) }
    };

    // Validate network
    if (!['mainnet', 'testnet'].includes(merged.network)) {
      throw new WasmInitializationError(`Invalid network: ${merged.network}. Must be 'mainnet' or 'testnet'`);
    }

    // Validate transport URL
    if (!merged.transport.url || typeof merged.transport.url !== 'string') {
      throw new WasmInitializationError('Transport URL must be a valid string');
    }

    return merged;
  }

  get(key) {
    return key.split('.').reduce((obj, k) => obj?.[k], this.config);
  }
}

/**
 * Resource cleanup manager for WASM objects
 */
class ResourceManager {
  constructor() {
    this._resources = new Set();
    this._cleanupHandlers = new Map();
  }

  register(resource, cleanupHandler) {
    this._resources.add(resource);
    if (cleanupHandler) {
      this._cleanupHandlers.set(resource, cleanupHandler);
    }
    return resource;
  }

  cleanup(resource) {
    if (this._resources.has(resource)) {
      const handler = this._cleanupHandlers.get(resource);
      if (handler) {
        try {
          handler();
        } catch (error) {
          console.warn('Error during resource cleanup:', error);
        }
        this._cleanupHandlers.delete(resource);
      }
      this._resources.delete(resource);
      
      // Call WASM free() method if available and resource is not null
      if (resource && typeof resource.free === 'function') {
        try {
          // Check if the resource is still valid (not already freed)
          if (resource.ptr !== 0) {
            resource.free();
          }
        } catch (error) {
          // Silently ignore null pointer errors during cleanup
          if (!error.message.includes('null pointer')) {
            console.warn('Error calling free() on WASM resource:', error);
          }
        }
      }
    }
  }

  cleanupAll() {
    for (const resource of this._resources) {
      this.cleanup(resource);
    }
  }
}

/**
 * Main WasmSDK class - JavaScript wrapper over raw WASM bindings
 */
export class WasmSDK {
  constructor(config = {}) {
    this._configManager = new ConfigManager(config);
    this._resourceManager = new ResourceManager();
    this._wasmSdk = null;
    this._initialized = false;
    this._wasmModule = null;

    // Bind methods to preserve 'this' context
    this.initialize = this.initialize.bind(this);
    this.destroy = this.destroy.bind(this);
  }

  /**
   * Initialize the WASM SDK
   * @returns {Promise<void>}
   */
  async initialize() {
    if (this._initialized) {
      return;
    }

    try {
      // Initialize WASM module - handle different environments
      if (typeof window === 'undefined') {
        // Node.js environment - read WASM file directly
        const fs = await import('fs');
        const path = await import('path');
        const { fileURLToPath } = await import('url');
        
        const __filename = fileURLToPath(import.meta.url);
        const __dirname = path.dirname(__filename);
        const wasmPath = path.resolve(__dirname, '../pkg/dash_wasm_sdk_bg.wasm');
        const wasmBuffer = fs.readFileSync(wasmPath);
        this._wasmModule = await init(wasmBuffer);
      } else {
        // Browser environment - use default fetch-based initialization
        this._wasmModule = await init();
      }
      
      // Create SDK builder based on network configuration
      const network = this._configManager.get('network');
      let builder;
      
      if (network === 'mainnet') {
        builder = WasmSdkBuilder.new_mainnet_trusted();
      } else {
        builder = WasmSdkBuilder.new_testnet_trusted();
      }

      // Register builder for cleanup
      this._resourceManager.register(builder);

      // Configure version if specified
      const version = this._configManager.get('version');
      if (version !== null) {
        builder = builder.with_version(version);
      }

      // Configure request settings
      const settings = this._configManager.get('settings');
      builder = builder.with_settings(
        settings.connect_timeout_ms,
        settings.timeout_ms,
        settings.retries,
        settings.ban_failed_address
      );

      // Build the SDK
      this._wasmSdk = builder.build();
      this._resourceManager.register(this._wasmSdk);

      this._initialized = true;
    } catch (error) {
      throw new WasmInitializationError(`Failed to initialize WASM SDK: ${error.message}`, { 
        originalError: error,
        config: this._configManager.config 
      });
    }
  }

  /**
   * Ensure SDK is initialized
   * @private
   */
  _ensureInitialized() {
    if (!this._initialized || !this._wasmSdk) {
      throw new WasmOperationError('SDK not initialized. Call initialize() first', 'check_initialization');
    }
  }

  /**
   * Wrap WASM operations with error handling
   * @private
   */
  async _wrapOperation(operation, operationName, ...args) {
    this._ensureInitialized();
    
    try {
      return await operation(this._wasmSdk, ...args);
    } catch (error) {
      throw new WasmOperationError(
        `Operation ${operationName} failed: ${error.message}`,
        operationName,
        { args, originalError: error }
      );
    }
  }

  // ===== IDENTITY OPERATIONS =====

  /**
   * Fetch an identity by ID
   * @param {string} identityId - Base58 encoded identity ID
   * @param {Object} options - Options object
   * @param {boolean} options.prove - Whether to fetch with proof (default: true)
   * @param {boolean} options.allowUnproved - Whether to allow unproved results (default: false)
   * @returns {Promise<Object>} Identity data
   */
  async getIdentity(identityId, options = {}) {
    const { prove = true, allowUnproved = false } = options;
    
    if (prove) {
      return this._wrapOperation(identity_fetch_with_proof_info, 'getIdentity', identityId);
    } else if (allowUnproved) {
      return this._wrapOperation(identity_fetch_unproved, 'getIdentity', identityId);
    } else {
      return this._wrapOperation(identity_fetch, 'getIdentity', identityId);
    }
  }

  /**
   * Get identity keys
   * @param {string} identityId - Identity ID
   * @param {string} keyRequestType - Type of key request
   * @param {Object} options - Optional parameters
   * @returns {Promise<Object>} Identity keys
   */
  async getIdentityKeys(identityId, keyRequestType = 'all', options = {}) {
    const { specificKeyIds, searchPurposeMap, limit, offset, prove = false } = options;
    
    const operation = prove ? get_identity_keys_with_proof_info : get_identity_keys;
    return this._wrapOperation(
      operation, 
      'getIdentityKeys', 
      identityId, 
      keyRequestType, 
      specificKeyIds, 
      searchPurposeMap, 
      limit, 
      offset
    );
  }

  /**
   * Get identity nonce
   * @param {string} identityId - Identity ID
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Identity nonce
   */
  async getIdentityNonce(identityId, prove = false) {
    const operation = prove ? get_identity_nonce_with_proof_info : get_identity_nonce;
    return this._wrapOperation(operation, 'getIdentityNonce', identityId);
  }

  /**
   * Get identity contract nonce
   * @param {string} identityId - Identity ID
   * @param {string} contractId - Contract ID
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Identity contract nonce
   */
  async getIdentityContractNonce(identityId, contractId, prove = false) {
    const operation = prove ? get_identity_contract_nonce_with_proof_info : get_identity_contract_nonce;
    return this._wrapOperation(operation, 'getIdentityContractNonce', identityId, contractId);
  }

  /**
   * Get identity balance
   * @param {string} identityId - Identity ID
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Identity balance
   */
  async getIdentityBalance(identityId, prove = false) {
    const operation = prove ? get_identity_balance_with_proof_info : get_identity_balance;
    return this._wrapOperation(operation, 'getIdentityBalance', identityId);
  }

  /**
   * Get multiple identity balances
   * @param {string[]} identityIds - Array of identity IDs
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Identity balances
   */
  async getIdentityBalances(identityIds, prove = false) {
    const operation = prove ? get_identities_balances_with_proof_info : get_identities_balances;
    return this._wrapOperation(operation, 'getIdentityBalances', identityIds);
  }

  /**
   * Get identity balance and revision
   * @param {string} identityId - Identity ID
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Identity balance and revision
   */
  async getIdentityBalanceAndRevision(identityId, prove = false) {
    const operation = prove ? get_identity_balance_and_revision_with_proof_info : get_identity_balance_and_revision;
    return this._wrapOperation(operation, 'getIdentityBalanceAndRevision', identityId);
  }

  /**
   * Get identity by public key hash
   * @param {string} publicKeyHash - Public key hash
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Identity
   */
  async getIdentityByPublicKeyHash(publicKeyHash, prove = false) {
    const operation = prove ? get_identity_by_public_key_hash_with_proof_info : get_identity_by_public_key_hash;
    return this._wrapOperation(operation, 'getIdentityByPublicKeyHash', publicKeyHash);
  }

  // ===== DPNS OPERATIONS =====

  /**
   * Check if a username is valid according to DPNS rules
   * @param {string} username - Username to check
   * @returns {boolean} Whether the username is valid
   */
  isDpnsUsernameValid(username) {
    try {
      return dpns_is_valid_username(username);
    } catch (error) {
      throw new WasmOperationError(`DPNS username validation failed: ${error.message}`, 'isDpnsUsernameValid', {
        username,
        originalError: error
      });
    }
  }

  /**
   * Check if a username is contested
   * @param {string} username - Username to check
   * @returns {boolean} Whether the username is contested
   */
  isDpnsUsernameContested(username) {
    try {
      return dpns_is_contested_username(username);
    } catch (error) {
      throw new WasmOperationError(`DPNS username contested check failed: ${error.message}`, 'isDpnsUsernameContested', {
        username,
        originalError: error
      });
    }
  }

  /**
   * Convert string to homograph-safe characters
   * @param {string} input - Input string
   * @returns {string} Homograph-safe string
   */
  dpnsConvertToHomographSafe(input) {
    try {
      return dpns_convert_to_homograph_safe(input);
    } catch (error) {
      throw new WasmOperationError(`DPNS homograph conversion failed: ${error.message}`, 'dpnsConvertToHomographSafe', {
        input,
        originalError: error
      });
    }
  }

  /**
   * Check if a DPNS name is available
   * @param {string} name - Name to check
   * @returns {Promise<boolean>} Whether the name is available
   */
  async isDpnsNameAvailable(name) {
    return this._wrapOperation(dpns_is_name_available, 'isDpnsNameAvailable', name);
  }

  /**
   * Resolve a DPNS name to identity information
   * @param {string} name - Name to resolve
   * @returns {Promise<Object>} Identity information
   */
  async resolveDpnsName(name) {
    return this._wrapOperation(dpns_resolve_name, 'resolveDpnsName', name);
  }

  /**
   * Get DPNS username by name
   * @param {string} username - Username to fetch
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Username information
   */
  async getDpnsUsername(username, prove = false) {
    const operation = prove ? get_dpns_username_by_name_with_proof_info : get_dpns_username_by_name;
    return this._wrapOperation(operation, 'getDpnsUsername', username);
  }

  /**
   * Register a DPNS name
   * @param {string} name - Name to register
   * @param {string} identityId - Identity ID
   * @param {number} publicKeyId - Public key ID
   * @param {string} privateKeyWif - Private key in WIF format
   * @param {Function} preorderCallback - Optional preorder callback
   * @returns {Promise<Object>} Registration result
   */
  async registerDpnsName(name, identityId, publicKeyId, privateKeyWif, preorderCallback = null) {
    return this._wrapOperation(
      dpns_register_name,
      'registerDpnsName',
      name,
      identityId,
      publicKeyId,
      privateKeyWif,
      preorderCallback
    );
  }

  // ===== DATA CONTRACT OPERATIONS =====

  /**
   * Fetch a data contract by ID
   * @param {string} contractId - Base58 encoded contract ID
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Data contract
   */
  async getDataContract(contractId, prove = false) {
    const operation = prove ? data_contract_fetch_with_proof_info : data_contract_fetch;
    return this._wrapOperation(operation, 'getDataContract', contractId);
  }

  /**
   * Get data contract history
   * @param {string} contractId - Contract ID
   * @param {Object} options - Options
   * @returns {Promise<Object>} Contract history
   */
  async getDataContractHistory(contractId, options = {}) {
    const { limit, offset, startAtMs, prove = false } = options;
    const operation = prove ? get_data_contract_history_with_proof_info : get_data_contract_history;
    return this._wrapOperation(operation, 'getDataContractHistory', contractId, limit, offset, startAtMs);
  }

  /**
   * Get multiple data contracts
   * @param {string[]} contractIds - Array of contract IDs
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Data contracts
   */
  async getDataContracts(contractIds, prove = false) {
    const operation = prove ? get_data_contracts_with_proof_info : get_data_contracts;
    return this._wrapOperation(operation, 'getDataContracts', contractIds);
  }

  // ===== TOKEN OPERATIONS =====

  /**
   * Calculate token ID from contract ID and position
   * @param {string} contractId - Data contract ID
   * @param {number} tokenPosition - Token position in contract
   * @returns {string} Token ID
   */
  calculateTokenId(contractId, tokenPosition) {
    try {
      return calculate_token_id_from_contract(contractId, tokenPosition);
    } catch (error) {
      throw new WasmOperationError(`Token ID calculation failed: ${error.message}`, 'calculateTokenId', {
        contractId,
        tokenPosition,
        originalError: error
      });
    }
  }

  /**
   * Get token price by contract
   * @param {string} contractId - Contract ID
   * @param {number} tokenPosition - Token position
   * @returns {Promise<Object>} Token price information
   */
  async getTokenPriceByContract(contractId, tokenPosition) {
    return this._wrapOperation(get_token_price_by_contract, 'getTokenPriceByContract', contractId, tokenPosition);
  }

  /**
   * Get identity token balances
   * @param {string} identityId - Identity ID
   * @param {string[]} tokenIds - Array of token IDs
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Token balances
   */
  async getIdentityTokenBalances(identityId, tokenIds, prove = false) {
    const operation = prove ? get_identity_token_balances_with_proof_info : get_identity_token_balances;
    return this._wrapOperation(operation, 'getIdentityTokenBalances', identityId, tokenIds);
  }

  // ===== WALLET OPERATIONS =====

  /**
   * Derive key from seed with extended path
   * @param {string} mnemonic - Mnemonic phrase
   * @param {string|null} passphrase - Optional passphrase
   * @param {string} path - Derivation path
   * @param {string} network - Network ('mainnet' or 'testnet')
   * @returns {Object} Derived key information
   */
  deriveKey(mnemonic, passphrase, path, network = null) {
    const targetNetwork = network || this._configManager.get('network');
    
    try {
      return derive_key_from_seed_with_extended_path(mnemonic, passphrase, path, targetNetwork);
    } catch (error) {
      throw new WasmOperationError(`Key derivation failed: ${error.message}`, 'deriveKey', {
        path,
        network: targetNetwork,
        originalError: error
      });
    }
  }

  /**
   * Derive DashPay contact key
   * @param {string} mnemonic - Mnemonic phrase
   * @param {string|null} passphrase - Optional passphrase
   * @param {string} senderIdentityId - Sender identity ID
   * @param {string} receiverIdentityId - Receiver identity ID
   * @param {number} account - Account number
   * @param {number} addressIndex - Address index
   * @param {string} network - Network
   * @returns {Object} Contact key information
   */
  deriveDashPayContactKey(mnemonic, passphrase, senderIdentityId, receiverIdentityId, account, addressIndex, network = null) {
    const targetNetwork = network || this._configManager.get('network');
    
    try {
      return derive_dashpay_contact_key(
        mnemonic,
        passphrase,
        senderIdentityId,
        receiverIdentityId,
        account,
        addressIndex,
        targetNetwork
      );
    } catch (error) {
      throw new WasmOperationError(`DashPay contact key derivation failed: ${error.message}`, 'deriveDashPayContactKey', {
        senderIdentityId,
        receiverIdentityId,
        account,
        addressIndex,
        network: targetNetwork,
        originalError: error
      });
    }
  }

  // ===== EPOCH OPERATIONS =====

  /**
   * Get epochs info
   * @param {Object} options - Options
   * @returns {Promise<Object>} Epochs info
   */
  async getEpochsInfo(options = {}) {
    const { startEpoch, count, ascending, prove = false } = options;
    const operation = prove ? get_epochs_info_with_proof_info : get_epochs_info;
    return this._wrapOperation(operation, 'getEpochsInfo', startEpoch, count, ascending);
  }

  /**
   * Get current epoch
   * @param {boolean} prove - Whether to fetch with proof
   * @returns {Promise<Object>} Current epoch
   */
  async getCurrentEpoch(prove = false) {
    const operation = prove ? get_current_epoch_with_proof_info : get_current_epoch;
    return this._wrapOperation(operation, 'getCurrentEpoch');
  }

  // ===== IDENTITY CREATION OPERATIONS =====

  /**
   * Create a new identity
   * @param {string} assetLockProof - Asset lock proof transaction hex
   * @param {string} assetLockPrivateKey - Private key controlling the asset lock
   * @param {string} publicKeysJson - JSON array of public keys
   * @returns {Promise<Object>} New identity
   */
  async createIdentity(assetLockProof, assetLockPrivateKey, publicKeysJson) {
    this._ensureInitialized();
    
    try {
      return await this._wasmSdk.identityCreate(assetLockProof, assetLockPrivateKey, publicKeysJson);
    } catch (error) {
      throw new WasmOperationError(`Identity creation failed: ${error.message}`, 'createIdentity', {
        originalError: error
      });
    }
  }

  /**
   * Top up an existing identity
   * @param {string} identityId - Identity ID to top up
   * @param {string} assetLockProof - Asset lock proof transaction hex
   * @param {string} assetLockPrivateKey - Private key controlling the asset lock
   * @returns {Promise<Object>} Top up result
   */
  async topUpIdentity(identityId, assetLockProof, assetLockPrivateKey) {
    this._ensureInitialized();
    
    try {
      return await this._wasmSdk.identityTopUp(identityId, assetLockProof, assetLockPrivateKey);
    } catch (error) {
      throw new WasmOperationError(`Identity top up failed: ${error.message}`, 'topUpIdentity', {
        identityId,
        originalError: error
      });
    }
  }

  // ===== UTILITY METHODS =====

  /**
   * Get SDK version
   * @returns {number} SDK version
   */
  getVersion() {
    this._ensureInitialized();
    return this._wasmSdk.version();
  }

  /**
   * Get configuration
   * @returns {Object} Current configuration
   */
  getConfig() {
    return { ...this._configManager.config };
  }

  /**
   * Check if SDK is initialized
   * @returns {boolean} Whether SDK is initialized
   */
  isInitialized() {
    return this._initialized;
  }

  /**
   * Destroy the SDK and cleanup resources
   */
  destroy() {
    this._resourceManager.cleanupAll();
    this._wasmSdk = null;
    this._initialized = false;
  }
}

/**
 * Export default instance creation helper
 */
export default WasmSDK;

/**
 * Re-export utility functions for direct use if needed
 */
export {
  dpns_convert_to_homograph_safe as convertToHomographSafe,
  dpns_is_valid_username as isValidDpnsUsername,
  dpns_is_contested_username as isContestedDpnsUsername,
  calculate_token_id_from_contract as calculateTokenIdFromContract,
  derive_key_from_seed_with_extended_path as deriveKeyFromSeedWithExtendedPath,
  derive_dashpay_contact_key as deriveDashPayContactKey
};