/**
 * Modern JavaScript wrapper for Dash Platform WASM SDK
 * Provides clean, Promise-based API with configuration-driven initialization
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

/**
 * Main WASM SDK class with modern initialization pattern
 */
export class WasmSDK {
    /**
     * Create a WASM SDK instance
     * @param {Object} config - Configuration object
     * @param {string} config.network - Network to use ('testnet' or 'mainnet')
     * @param {Object} config.transport - Transport configuration
     * @param {string|string[]} config.transport.url - Endpoint URL(s)
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
        this.currentEndpointIndex = 0;
        
        // Bind methods to preserve 'this' context
        this.initialize = this.initialize.bind(this);
        this.destroy = this.destroy.bind(this);
    }

    /**
     * Initialize the WASM SDK
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
            
            // Create the SDK builder based on network
            const builder = this._createSdkBuilder();
            
            // Configure the builder
            this._configureSdkBuilder(builder);
            
            // Build the SDK instance
            this.wasmSdk = builder.build();
            
            // Register the SDK instance for resource management
            this.resourceManager.register(this.wasmSdk, 'wasm_sdk');
            
            this.initialized = true;
            
            if (this.configManager.getDebug()) {
                console.debug('WasmSDK initialized successfully', {
                    network: this.configManager.getNetwork(),
                    endpoint: this.configManager.getPrimaryEndpoint(),
                    proofs: this.configManager.getProofs()
                });
            }
            
        } catch (error) {
            throw new WasmInitializationError(
                `Failed to initialize WASM SDK: ${error.message}`,
                {
                    network: this.configManager.getNetwork(),
                    endpoint: this.configManager.getPrimaryEndpoint(),
                    originalError: error.message
                }
            );
        }
    }

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
        const endpoint = this.configManager.getPrimaryEndpoint();
        
        // Set the endpoint
        if (typeof builder.with_endpoint === 'function') {
            builder.with_endpoint(endpoint);
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
     * Get current endpoint
     * @returns {string} Current endpoint URL
     */
    getCurrentEndpoint() {
        return this.configManager.getPrimaryEndpoint();
    }

    /**
     * Execute a WASM SDK operation with error handling and resource management
     * @private
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

    // ========== Query Operations ==========

    /**
     * Get identity by ID
     * @param {string} identityId - Identity ID
     * @returns {Promise<Object|null>} Identity or null if not found
     */
    async getIdentity(identityId) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'identity_fetch' : 'identity_fetch_unproved';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId),
            methodName,
            { identityId, proofs: useProofs }
        );
    }

    /**
     * Get identities by IDs
     * @param {string[]} identityIds - Array of identity IDs
     * @returns {Promise<Object[]>} Array of identities
     */
    async getIdentities(identityIds) {
        ErrorUtils.validateRequired({ identityIds }, ['identityIds']);
        
        if (!Array.isArray(identityIds)) {
            throw new WasmConfigurationError(
                'identityIds must be an array',
                'identityIds',
                identityIds
            );
        }
        
        return this._executeOperation(
            () => this.wasmSdk.get_identities(identityIds),
            'get_identities',
            { identityIds }
        );
    }

    /**
     * Get data contract by ID
     * @param {string} contractId - Data contract ID
     * @returns {Promise<Object|null>} Data contract or null if not found
     */
    async getDataContract(contractId) {
        ErrorUtils.validateRequired({ contractId }, ['contractId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'data_contract_fetch_with_proof_info' : 'data_contract_fetch';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, contractId),
            methodName,
            { contractId, proofs: useProofs }
        );
    }

    /**
     * Get documents by contract and type
     * @param {string} contractId - Data contract ID
     * @param {string} documentType - Document type
     * @param {Object} options - Query options
     * @param {Array} options.where - Where conditions
     * @param {Array} options.orderBy - Order by conditions
     * @param {number} options.limit - Result limit
     * @param {number} options.offset - Result offset
     * @returns {Promise<Object[]>} Array of documents
     */
    async getDocuments(contractId, documentType, options = {}) {
        ErrorUtils.validateRequired({ contractId, documentType }, ['contractId', 'documentType']);
        
        const { where = [], orderBy = [], limit, offset = 0, startAfter, startAt, getAllDocuments = false } = options;
        const useProofs = this.configManager.getProofs();
        // Note: Document proof verification has issues, use non-proof version for now
        const methodName = 'get_documents'; // Always use non-proof version until proof issues fixed
        
        if (useProofs && this.configManager.getDebug()) {
            console.debug('Note: Using non-proof document query due to proof verification issues');
        }
        
        // Convert where and orderBy to JSON strings if they're arrays
        const whereClause = Array.isArray(where) ? JSON.stringify(where) : where;
        const orderByClause = Array.isArray(orderBy) ? JSON.stringify(orderBy) : orderBy;
        
        if (getAllDocuments) {
            // Get ALL documents using internal pagination (ignore user limit)
            // Implement pagination to get all documents
            const allDocuments = [];
            const batchSize = 50; // Use smaller batch size for testing
            let startAfter = null;
            let hasMore = true;
            let batchCount = 0;
            
            // Pagination loop (debug output suppressed for clean results)
            while (hasMore) {
                batchCount++;
                
                const batch = await this._executeOperation(
                    () => this.wasmModule[methodName](
                        this.wasmSdk,
                        contractId,
                        documentType,
                        whereClause || null,
                        orderByClause || null,
                        batchSize,
                        startAfter,
                        null  // startAt
                    ),
                    methodName,
                    { contractId, documentType, batch: true, proofs: false }
                );
                
                if (batch && batch.length > 0) {
                    allDocuments.push(...batch);
                    
                    if (batch.length < batchSize) {
                        hasMore = false; // Last batch was partial, no more documents
                    } else {
                        // Set startAfter to the ID of the last document for next batch
                        const lastDoc = batch[batch.length - 1];
                        const lastDocData = typeof lastDoc.toJSON === 'function' ? lastDoc.toJSON() : lastDoc;
                        const nextStartAfter = lastDocData.id || lastDocData.$id || lastDocData.identifier;
                        
                        if (nextStartAfter === startAfter) {
                            // Prevent infinite loop if same ID returned
                            hasMore = false;
                        } else {
                            startAfter = nextStartAfter;
                        }
                    }
                } else {
                    hasMore = false;
                }
                
                // Safety limit to prevent infinite loops
                if (batchCount > 50) {
                    hasMore = false;
                }
            }
            
            // Return structured JSON response with all documents
            return {
                contractId,
                documentType,
                totalCount: allDocuments.length,
                documents: allDocuments.map(doc => {
                    const docData = typeof doc.toJSON === 'function' ? doc.toJSON() : doc;
                    return {
                        id: docData.$id || docData.id || docData.identifier,
                        ownerId: docData.$ownerId || docData.ownerId,
                        revision: docData.revision,
                        createdAt: docData.$createdAt || docData.createdAt,
                        updatedAt: docData.$updatedAt || docData.updatedAt,
                        data: docData.data || docData
                    };
                }),
                query: {
                    where: where,
                    orderBy: orderBy,
                    getAllDocuments: true
                }
            };
        } else {
            // Single query with user-specified parameters
            const documents = await this._executeOperation(
                () => this.wasmModule[methodName](
                    this.wasmSdk,
                    contractId,
                    documentType,
                    whereClause || null,
                    orderByClause || null,
                    limit || null, // Use user limit or WASM default
                    startAfter || null,
                    startAt || null
                ),
                methodName,
                { contractId, documentType, options, proofs: false }
            );
            
            // Return structured JSON response
            const documentArray = documents || [];
            return {
                contractId,
                documentType,
                totalCount: documentArray.length,
                documents: documentArray.map(doc => {
                    const docData = typeof doc.toJSON === 'function' ? doc.toJSON() : doc;
                    return {
                        id: docData.$id || docData.id || docData.identifier,
                        ownerId: docData.$ownerId || docData.ownerId,
                        revision: docData.revision,
                        createdAt: docData.$createdAt || docData.createdAt,
                        updatedAt: docData.$updatedAt || docData.updatedAt,
                        data: docData.data || docData
                    };
                }),
                query: {
                    where: where,
                    orderBy: orderBy,
                    limit: limit,
                    offset: offset,
                    startAfter: startAfter,
                    startAt: startAt
                }
            };
        }
    }

    /**
     * Get document by ID
     * @param {string} contractId - Data contract ID
     * @param {string} documentType - Document type
     * @param {string} documentId - Document ID
     * @returns {Promise<Object|null>} Document or null if not found
     */
    async getDocument(contractId, documentType, documentId) {
        ErrorUtils.validateRequired({ contractId, documentType, documentId }, 
                                   ['contractId', 'documentType', 'documentId']);
        
        return this._executeOperation(
            () => this.wasmSdk.get_document(contractId, documentType, documentId),
            'get_document',
            { contractId, documentType, documentId }
        );
    }

    // ========== State Transition Operations ==========

    /**
     * Create and submit an identity creation state transition
     * @param {Object} identityData - Identity data
     * @param {string} privateKey - Private key for signing
     * @returns {Promise<Object>} State transition result
     */
    async createIdentity(identityData, privateKey) {
        ErrorUtils.validateRequired({ identityData, privateKey }, ['identityData', 'privateKey']);
        
        return this._executeOperation(
            () => this.wasmSdk.create_identity(identityData, privateKey),
            'create_identity',
            { identityData: '[REDACTED]', privateKey: '[REDACTED]' }
        );
    }

    /**
     * Create and submit a data contract state transition
     * @param {Object} contractData - Data contract data
     * @param {string} identityId - Owner identity ID
     * @param {string} privateKey - Private key for signing
     * @returns {Promise<Object>} State transition result
     */
    async createDataContract(contractData, identityId, privateKey) {
        ErrorUtils.validateRequired({ contractData, identityId, privateKey }, 
                                   ['contractData', 'identityId', 'privateKey']);
        
        return this._executeOperation(
            () => this.wasmSdk.create_data_contract(contractData, identityId, privateKey),
            'create_data_contract',
            { identityId, privateKey: '[REDACTED]' }
        );
    }

    /**
     * Create and submit a document creation state transition
     * @param {Object} documentData - Document data
     * @param {string} contractId - Data contract ID
     * @param {string} documentType - Document type
     * @param {string} identityId - Owner identity ID
     * @param {string} privateKey - Private key for signing
     * @returns {Promise<Object>} State transition result
     */
    async createDocument(documentData, contractId, documentType, identityId, privateKey) {
        ErrorUtils.validateRequired(
            { documentData, contractId, documentType, identityId, privateKey },
            ['documentData', 'contractId', 'documentType', 'identityId', 'privateKey']
        );
        
        return this._executeOperation(
            () => this.wasmSdk.create_document(
                documentData, 
                contractId, 
                documentType, 
                identityId, 
                privateKey
            ),
            'create_document',
            { contractId, documentType, identityId, privateKey: '[REDACTED]' }
        );
    }

    // ========== Utility Operations ==========

    /**
     * Get platform version information
     * @returns {Promise<Object>} Platform version info
     */
    async getPlatformVersion() {
        return this._executeOperation(
            () => this.wasmSdk.get_platform_version(),
            'get_platform_version'
        );
    }

    /**
     * Get network status
     * @returns {Promise<Object>} Network status information
     */
    async getNetworkStatus() {
        return this._executeOperation(
            () => this.wasmSdk.get_network_status(),
            'get_network_status'
        );
    }

    /**
     * Validate a document against its data contract
     * @param {Object} document - Document to validate
     * @param {Object} dataContract - Data contract
     * @returns {Promise<boolean>} True if valid
     */
    async validateDocument(document, dataContract) {
        ErrorUtils.validateRequired({ document, dataContract }, ['document', 'dataContract']);
        
        return this._executeOperation(
            () => this.wasmSdk.validate_document(document, dataContract),
            'validate_document'
        );
    }

    // ========== Key Generation & Crypto Operations ==========

    /**
     * Generate a mnemonic phrase
     * @param {number} wordCount - Number of words (12, 15, 18, 21, or 24)
     * @returns {Promise<string>} Generated mnemonic phrase
     */
    async generateMnemonic(wordCount = 12) {
        ErrorUtils.validateRequired({ wordCount }, ['wordCount']);
        
        if (![12, 15, 18, 21, 24].includes(wordCount)) {
            throw new WasmOperationError(
                'Invalid word count. Must be 12, 15, 18, 21, or 24',
                'generate_mnemonic',
                { wordCount }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.generate_mnemonic(wordCount),
            'generate_mnemonic',
            { wordCount }
        );
    }

    /**
     * Validate a mnemonic phrase
     * @param {string} mnemonic - Mnemonic phrase to validate
     * @returns {Promise<boolean>} True if mnemonic is valid
     */
    async validateMnemonic(mnemonic) {
        ErrorUtils.validateRequired({ mnemonic }, ['mnemonic']);
        
        return this._executeOperation(
            () => this.wasmModule.validate_mnemonic(mnemonic),
            'validate_mnemonic',
            { mnemonic: '[REDACTED]' }
        );
    }

    /**
     * Convert mnemonic to seed
     * @param {string} mnemonic - Mnemonic phrase
     * @param {string} passphrase - Optional passphrase
     * @returns {Promise<Uint8Array>} Generated seed
     */
    async mnemonicToSeed(mnemonic, passphrase = '') {
        ErrorUtils.validateRequired({ mnemonic }, ['mnemonic']);
        
        return this._executeOperation(
            () => this.wasmModule.mnemonic_to_seed(mnemonic, passphrase),
            'mnemonic_to_seed',
            { mnemonic: '[REDACTED]', passphrase: passphrase ? '[REDACTED]' : 'none' }
        );
    }

    /**
     * Derive key from seed with derivation path
     * @param {string} mnemonic - Mnemonic phrase
     * @param {string} passphrase - Optional passphrase
     * @param {string} path - Derivation path (e.g., "m/44'/5'/0'/0/0")
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<Object>} Object containing address, private_key_wif, and public_key
     */
    async deriveKeyFromSeedWithPath(mnemonic, passphrase = '', path, network = 'testnet') {
        ErrorUtils.validateRequired({ mnemonic, path, network }, ['mnemonic', 'path', 'network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'derive_key_from_seed_with_path',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.derive_key_from_seed_with_path(mnemonic, passphrase, path, network),
            'derive_key_from_seed_with_path',
            { mnemonic: '[REDACTED]', passphrase: passphrase ? '[REDACTED]' : 'none', path, network }
        );
    }

    /**
     * Generate a key pair
     * @returns {Promise<Object>} Object containing private and public keys
     */
    async generateKeyPair() {
        return this._executeOperation(
            () => this.wasmModule.generate_key_pair(),
            'generate_key_pair'
        );
    }

    /**
     * Convert public key to address
     * @param {string} publicKey - Public key in hex format
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<string>} Generated address
     */
    async pubkeyToAddress(publicKey, network = 'testnet') {
        ErrorUtils.validateRequired({ publicKey, network }, ['publicKey', 'network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'pubkey_to_address',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.pubkey_to_address(publicKey, network),
            'pubkey_to_address',
            { publicKey, network }
        );
    }

    /**
     * Validate an address
     * @param {string} address - Address to validate
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<boolean>} True if address is valid for the network
     */
    async validateAddress(address, network = 'testnet') {
        ErrorUtils.validateRequired({ address, network }, ['address', 'network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'validate_address',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.validate_address(address, network),
            'validate_address',
            { address, network }
        );
    }

    /**
     * Sign a message
     * @param {string} message - Message to sign
     * @param {string} privateKey - Private key for signing
     * @returns {Promise<string>} Signature
     */
    async signMessage(message, privateKey) {
        ErrorUtils.validateRequired({ message, privateKey }, ['message', 'privateKey']);
        
        return this._executeOperation(
            () => this.wasmModule.sign_message(message, privateKey),
            'sign_message',
            { message, privateKey: '[REDACTED]' }
        );
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