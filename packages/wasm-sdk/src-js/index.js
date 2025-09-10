/**
 * Modern JavaScript wrapper for Dash Platform WASM SDK
 * Simplified orchestrator that delegates to focused service classes
 */

import { ConfigManager } from './config-manager.js';
import { ResourceManager } from './resource-manager.js';
import { 
    WasmSDKError, 
    WasmInitializationError, 
    WasmOperationError,
    ErrorMapper,
    ErrorUtils 
} from './error-handler.js';

// Import service classes
import { IdentityService } from './services/identity-service.js';
import { DocumentService } from './services/document-service.js';
import { ContractService } from './services/contract-service.js';
import { CryptoService } from './services/crypto-service.js';
import { SystemService } from './services/system-service.js';
import { DPNSService } from './services/dpns-service.js';

/**
 * Main WASM SDK class - simplified orchestrator
 * Delegates operations to focused service classes
 */
export class WasmSDK {
    /**
     * Create a WASM SDK instance
     * @param {Object} config - Configuration object
     * @param {string} config.network - Network to use ('testnet' or 'mainnet')
     * @param {Object} config.transport - Transport configuration
     * @param {string} config.transport.url - Custom endpoint URL (optional)
     * @param {number} config.transport.timeout - Request timeout in milliseconds
     * @param {number} config.transport.retries - Number of retries
     * @param {boolean} config.proofs - Enable proof verification
     * @param {boolean} config.debug - Enable debug mode
     */
    constructor(config = {}) {
        this.configManager = new ConfigManager(config);
        this.resourceManager = new ResourceManager();
        this.initialized = false;
        this.wasmModule = null;
        this.wasmSdk = null;
        
        // Service instances - initialized after WASM SDK is ready
        this.identityService = null;
        this.documentService = null;
        this.contractService = null;
        this.cryptoService = null;
        this.systemService = null;
        this.dpnsService = null;
        
        // Bind methods to preserve 'this' context
        this.initialize = this.initialize.bind(this);
        this.destroy = this.destroy.bind(this);
    }

    // ========== Initialization & Core Methods ==========

    /**
     * Initialize the WASM SDK and all services
     * @returns {Promise<void>} Promise that resolves when initialization is complete
     * @throws {WasmInitializationError} If initialization fails
     */
    async initialize() {
        if (this.initialized) {
            if (this.configManager.getDebug()) {
                console.debug('WasmSDK already initialized');
            }
            return;
        }

        try {
            // Import the WASM module dynamically
            const wasmModule = await this._loadWasmModule();
            this.wasmModule = wasmModule;

            // Initialize the WASM module
            await wasmModule.default();
            
            // Prefetch trusted quorums for proof verification (required for trusted mode)
            await this._prefetchTrustedQuorums(wasmModule);
            
            // Create the SDK builder based on network (with custom endpoint if provided)
            const builder = this._createSdkBuilder();
            
            // Configure the builder
            this._configureSdkBuilder(builder);
            
            // Build the SDK instance
            this.wasmSdk = builder.build();
            
            // Register the SDK instance for resource management
            this.resourceManager.register(this.wasmSdk, 'wasm_sdk');
            
            // Initialize all service classes
            this._initializeServices();
            
            this.initialized = true;
            
            if (this.configManager.getDebug()) {
                console.debug('WasmSDK initialized successfully', {
                    network: this.configManager.getNetwork(),
                    customEndpoint: this.configManager.getCustomEndpoint(),
                    proofs: this.configManager.getProofs()
                });
            }
            
        } catch (error) {
            throw new WasmInitializationError(
                `Failed to initialize WASM SDK: ${error.message}`,
                {
                    network: this.configManager.getNetwork(),
                    customEndpoint: this.configManager.getCustomEndpoint(),
                    originalError: error.message
                }
            );
        }
    }

    /**
     * Check if SDK is initialized
     * @returns {boolean} True if initialized
     */
    isInitialized() {
        return this.initialized;
    }

    /**
     * Get current configuration
     * @returns {Object} Current configuration
     */
    getConfig() {
        return this.configManager.getConfig();
    }

    /**
     * Get current network
     * @returns {string} Current network
     */
    getNetwork() {
        return this.configManager.getNetwork();
    }

    /**
     * Get current endpoint (custom endpoint or WASM SDK's internal endpoint)
     * @returns {string} Current endpoint URL
     */
    getCurrentEndpoint() {
        const customEndpoint = this.configManager.getCustomEndpoint();
        if (customEndpoint) {
            return customEndpoint;
        }
        
        // Delegate to WASM SDK for endpoint discovery
        try {
            return this.wasmSdk?.getCurrentEndpoint?.() || 'WASM SDK managed';
        } catch (error) {
            return 'WASM SDK managed';
        }
    }

    // ========== Identity Operations - Delegated to IdentityService ==========

    async getIdentity(identityId) {
        this._ensureInitialized();
        return this.identityService.getIdentity(identityId);
    }

    async getIdentities(identityIds) {
        this._ensureInitialized();
        return this.identityService.getIdentities(identityIds);
    }

    async getIdentityBalance(identityId) {
        this._ensureInitialized();
        return this.identityService.getIdentityBalance(identityId);
    }

    async getIdentityKeys(identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset) {
        this._ensureInitialized();
        return this.identityService.getIdentityKeys(identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset);
    }

    async getIdentityNonce(identityId) {
        this._ensureInitialized();
        return this.identityService.getIdentityNonce(identityId);
    }

    async getIdentityContractNonce(identityId, contractId) {
        this._ensureInitialized();
        return this.identityService.getIdentityContractNonce(identityId, contractId);
    }

    async getIdentityBalanceAndRevision(identityId) {
        this._ensureInitialized();
        return this.identityService.getIdentityBalanceAndRevision(identityId);
    }

    async getIdentityByPublicKeyHash(publicKeyHash) {
        this._ensureInitialized();
        return this.identityService.getIdentityByPublicKeyHash(publicKeyHash);
    }

    async getIdentitiesBalances(identityIds) {
        this._ensureInitialized();
        return this.identityService.getIdentitiesBalances(identityIds);
    }

    // Identity State Transitions
    async identityCreate(assetLockProof, assetLockPrivateKey, publicKeys) {
        this._ensureInitialized();
        return this.identityService.createIdentity(assetLockProof, assetLockPrivateKey, publicKeys);
    }

    async identityTopUp(identityId, assetLockProof, assetLockPrivateKey) {
        this._ensureInitialized();
        return this.identityService.topUpIdentity(identityId, assetLockProof, assetLockPrivateKey);
    }

    async identityUpdate(mnemonic, identityId, updateData, keyIndex) {
        this._ensureInitialized();
        return this.identityService.updateIdentity(mnemonic, identityId, updateData, keyIndex);
    }

    async identityWithdraw(mnemonic, identityId, toAddress, amount, keyIndex) {
        this._ensureInitialized();
        return this.identityService.withdrawFromIdentity(mnemonic, identityId, toAddress, amount, keyIndex);
    }

    // ========== Document Operations - Delegated to DocumentService ==========

    async getDocuments(contractId, documentType, options) {
        this._ensureInitialized();
        return this.documentService.getDocuments(contractId, documentType, options);
    }

    async getDocument(contractId, documentType, documentId) {
        this._ensureInitialized();
        return this.documentService.getDocument(contractId, documentType, documentId);
    }

    async documentCreate(mnemonic, identityId, contractId, documentType, documentData, keyIndex) {
        this._ensureInitialized();
        return this.documentService.createDocument(mnemonic, identityId, contractId, documentType, documentData, keyIndex);
    }

    async documentUpdate(mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex) {
        this._ensureInitialized();
        return this.documentService.updateDocument(mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex);
    }

    // ========== Contract Operations - Delegated to ContractService ==========

    async getDataContract(contractId) {
        this._ensureInitialized();
        return this.contractService.getDataContract(contractId);
    }

    async dataContractCreate(mnemonic, identityId, contractDefinition, keyIndex) {
        this._ensureInitialized();
        return this.contractService.createDataContract(mnemonic, identityId, contractDefinition, keyIndex);
    }

    async dataContractUpdate(mnemonic, identityId, contractId, updateDefinition, keyIndex) {
        this._ensureInitialized();
        return this.contractService.updateDataContract(mnemonic, identityId, contractId, updateDefinition, keyIndex);
    }

    async validateDocument(document, dataContract) {
        this._ensureInitialized();
        return this.contractService.validateDocument(document, dataContract);
    }

    // ========== Crypto Operations - Delegated to CryptoService ==========

    async generateMnemonic(wordCount) {
        this._ensureInitialized();
        return this.cryptoService.generateMnemonic(wordCount);
    }

    async validateMnemonic(mnemonic) {
        this._ensureInitialized();
        return this.cryptoService.validateMnemonic(mnemonic);
    }

    async mnemonicToSeed(mnemonic, passphrase) {
        this._ensureInitialized();
        return this.cryptoService.mnemonicToSeed(mnemonic, passphrase);
    }

    async deriveKeyFromSeedWithPath(mnemonic, passphrase, path, network) {
        this._ensureInitialized();
        return this.cryptoService.deriveKeyFromSeedWithPath(mnemonic, passphrase, path, network);
    }

    async generateKeyPair(network) {
        this._ensureInitialized();
        return this.cryptoService.generateKeyPair(network);
    }

    async pubkeyToAddress(publicKey, network) {
        this._ensureInitialized();
        return this.cryptoService.pubkeyToAddress(publicKey, network);
    }

    async validateAddress(address, network) {
        this._ensureInitialized();
        return this.cryptoService.validateAddress(address, network);
    }

    async signMessage(message, privateKey) {
        this._ensureInitialized();
        return this.cryptoService.signMessage(message, privateKey);
    }

    // ========== System Operations - Delegated to SystemService ==========

    async getStatus() {
        this._ensureInitialized();
        return this.systemService.getStatus();
    }

    async getPlatformVersion() {
        this._ensureInitialized();
        return this.systemService.getPlatformVersion();
    }

    async getNetworkStatus() {
        this._ensureInitialized();
        return this.systemService.getNetworkStatus();
    }

    async getCurrentEpoch() {
        this._ensureInitialized();
        return this.systemService.getCurrentEpoch();
    }

    async getEpochsInfo(start, count, ascending) {
        this._ensureInitialized();
        return this.systemService.getEpochsInfo(start, count, ascending);
    }

    async getFinalizedEpochInfos(count, ascending) {
        this._ensureInitialized();
        return this.systemService.getFinalizedEpochInfos(count, ascending);
    }

    async getCurrentQuorumsInfo() {
        this._ensureInitialized();
        return this.systemService.getCurrentQuorumsInfo();
    }

    async getTotalCreditsInPlatform() {
        this._ensureInitialized();
        return this.systemService.getTotalCreditsInPlatform();
    }

    async getPathElements(path, keys) {
        this._ensureInitialized();
        return this.systemService.getPathElements(path, keys);
    }

    async getProtocolVersionUpgradeState() {
        this._ensureInitialized();
        return this.systemService.getProtocolVersionUpgradeState();
    }

    async getProtocolVersionUpgradeVoteStatus(startIdentityId, limit, offset) {
        this._ensureInitialized();
        return this.systemService.getProtocolVersionUpgradeVoteStatus(startIdentityId, limit, offset);
    }

    async getPrefundedSpecializedBalance(identityId) {
        this._ensureInitialized();
        return this.systemService.getPrefundedSpecializedBalance(identityId);
    }

    // ========== DPNS Operations - Delegated to DPNSService ==========

    async dpnsIsValidUsername(label) {
        this._ensureInitialized();
        return this.dpnsService.isValidUsername(label);
    }

    async dpnsConvertToHomographSafe(input) {
        this._ensureInitialized();
        return this.dpnsService.convertToHomographSafe(input);
    }

    async dpnsIsContestedUsername(label) {
        this._ensureInitialized();
        return this.dpnsService.isContestedUsername(label);
    }

    async dpnsResolveName(name) {
        this._ensureInitialized();
        return this.dpnsService.resolveName(name);
    }

    async dpnsIsNameAvailable(label) {
        this._ensureInitialized();
        return this.dpnsService.isNameAvailable(label);
    }

    // ========== Resource Management ==========

    /**
     * Get resource manager statistics
     * @returns {Object} Resource statistics
     */
    getResourceStats() {
        return this.resourceManager.getStats();
    }

    /**
     * Clean up stale resources
     * @param {Object} options - Cleanup options
     * @returns {number} Number of resources cleaned up
     */
    cleanupResources(options) {
        return this.resourceManager.cleanup(options);
    }

    /**
     * Destroy the SDK and clean up all resources
     * @returns {Promise<void>} Promise that resolves when cleanup is complete
     */
    async destroy() {
        if (!this.initialized) {
            return;
        }
        
        try {
            // Clean up all managed resources
            this.resourceManager.destroy();
            
            // Reset services
            this.identityService = null;
            this.documentService = null;
            this.contractService = null;
            this.cryptoService = null;
            this.systemService = null;
            this.dpnsService = null;
            
            // Reset state
            this.wasmSdk = null;
            this.wasmModule = null;
            this.initialized = false;
            
            if (this.configManager.getDebug()) {
                console.debug('WasmSDK destroyed successfully');
            }
            
        } catch (error) {
            throw new WasmOperationError(
                `Error during SDK destruction: ${error.message}`,
                'destroy_sdk',
                { originalError: error.message }
            );
        }
    }

    // ========== Private Helper Methods ==========

    /**
     * Load the WASM module dynamically
     * @private
     * @returns {Promise<Object>} WASM module
     */
    async _loadWasmModule() {
        try {
            // Try to import from the built package
            return await import('../pkg/wasm_sdk.js');
        } catch (error) {
            throw new WasmInitializationError(
                'Failed to load WASM module. Make sure the package is built correctly.',
                { originalError: error.message }
            );
        }
    }

    /**
     * Prefetch trusted quorums for proof verification
     * @private
     * @param {Object} wasmModule - The loaded WASM module
     * @throws {WasmInitializationError} If quorum prefetching fails
     */
    async _prefetchTrustedQuorums(wasmModule) {
        const network = this.configManager.getNetwork();
        
        if (this.configManager.getDebug()) {
            console.debug(`Prefetching trusted quorums for ${network}...`);
        }
        
        try {
            switch (network) {
                case 'mainnet':
                    await wasmModule.prefetch_trusted_quorums_mainnet();
                    break;
                case 'testnet':
                default:
                    await wasmModule.prefetch_trusted_quorums_testnet();
                    break;
            }
            
            if (this.configManager.getDebug()) {
                console.debug(`Trusted quorums prefetched successfully for ${network}`);
            }
        } catch (error) {
            throw new WasmInitializationError(
                `Failed to prefetch trusted quorums for ${network}: ${error.message}`,
                { network, originalError: error.message }
            );
        }
    }

    /**
     * Create appropriate SDK builder based on network (TRUSTED MODE ONLY)
     * @private
     * @returns {Object} WASM SDK builder
     * @throws {WasmInitializationError} If trusted mode initialization fails
     */
    _createSdkBuilder() {
        const { WasmSdkBuilder } = this.wasmModule;
        const network = this.configManager.getNetwork();
        
        try {
            // WASM SDK only supports trusted mode - use trusted builders
            let builder;
            switch (network) {
                case 'mainnet':
                    builder = WasmSdkBuilder.new_mainnet_trusted();
                    break;
                case 'testnet':
                default:
                    builder = WasmSdkBuilder.new_testnet_trusted();
                    break;
            }
            
            // Trusted builders might return a Result that needs unwrapping
            if (builder && typeof builder === 'object' && builder.constructor && builder.constructor.name === 'WasmSdkBuilder') {
                return builder;
            } else {
                throw new Error(`Trusted builder returned unexpected type: ${typeof builder}`);
            }
        } catch (error) {
            throw new WasmInitializationError(
                `Failed to create trusted SDK builder for ${network}: ${error.message}`,
                { network, originalError: error.message }
            );
        }
    }

    /**
     * Configure the SDK builder with transport and other settings
     * @private
     * @param {Object} builder - WASM SDK builder
     */
    _configureSdkBuilder(builder) {
        const transport = this.configManager.getTransport();
        const customEndpoint = this.configManager.getCustomEndpoint();
        
        // Set custom endpoint if provided (otherwise WASM SDK handles endpoint discovery)
        if (customEndpoint && typeof builder.with_endpoint === 'function') {
            builder.with_endpoint(customEndpoint);
        }
        
        // Configure proof verification
        if (typeof builder.with_proofs === 'function') {
            builder.with_proofs(this.configManager.getProofs());
        }
        
        // Configure timeout if supported
        if (typeof builder.with_timeout === 'function' && transport.timeout) {
            builder.with_timeout(transport.timeout);
        }
    }

    /**
     * Initialize all service classes after WASM SDK is ready
     * @private
     */
    _initializeServices() {
        this.identityService = new IdentityService(this, this.wasmSdk, this.wasmModule, this.configManager);
        this.documentService = new DocumentService(this, this.wasmSdk, this.wasmModule, this.configManager);
        this.contractService = new ContractService(this, this.wasmSdk, this.wasmModule, this.configManager);
        this.cryptoService = new CryptoService(this, this.wasmSdk, this.wasmModule, this.configManager);
        this.systemService = new SystemService(this, this.wasmSdk, this.wasmModule, this.configManager);
        this.dpnsService = new DPNSService(this, this.wasmSdk, this.wasmModule, this.configManager);
    }

    /**
     * Ensure SDK is initialized before operations
     * @private
     * @throws {WasmOperationError} If SDK is not initialized
     */
    _ensureInitialized() {
        if (!this.initialized || !this.wasmSdk) {
            throw new WasmOperationError(
                'SDK not initialized. Call initialize() first.',
                'check_initialization'
            );
        }
    }

    /**
     * Execute a WASM SDK operation with error handling and resource management
     * Used by service classes for consistent error handling
     * @param {Function} operation - Operation to execute
     * @param {string} operationName - Name of operation for error context
     * @param {Object} context - Additional context for errors
     * @returns {Promise<*>} Operation result
     */
    async _executeOperation(operation, operationName, context = {}) {
        this._ensureInitialized();
        
        return this.resourceManager.wrapOperation(
            operation,
            operationName,
            { autoRegister: true }
        )();
    }
}

// Export error classes for consumer use
export {
    WasmSDKError,
    WasmInitializationError,
    WasmOperationError,
    WasmConfigurationError,
    WasmTransportError
} from './error-handler.js';

// Export configuration utilities
export { ConfigUtils } from './config-manager.js';

// Export resource utilities
export { ResourceUtils } from './resource-manager.js';

// Default export
export default WasmSDK;