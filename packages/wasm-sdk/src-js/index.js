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
        
        const identity = await this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId),
            methodName,
            { identityId, proofs: useProofs }
        );
        
        // Return complete WASM SDK identity information as JSON
        if (identity && typeof identity.toJSON === 'function') {
            return identity.toJSON();
        }
        return identity;
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
        
        const contract = await this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, contractId),
            methodName,
            { contractId, proofs: useProofs }
        );
        
        // Return complete WASM SDK contract information as JSON
        if (contract && typeof contract.toJSON === 'function') {
            return contract.toJSON();
        }
        return contract;
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
                    // Return complete WASM SDK document information
                    return typeof doc.toJSON === 'function' ? doc.toJSON() : doc;
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
                    // Return complete WASM SDK document information
                    return typeof doc.toJSON === 'function' ? doc.toJSON() : doc;
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
     * Create identity with asset lock proof
     * @param {string} assetLockProof - Asset lock proof (hex-encoded JSON)
     * @param {string} assetLockPrivateKey - Asset lock private key (WIF)
     * @param {string} publicKeys - JSON string of public keys array
     * @returns {Promise<Object>} Identity creation result
     */
    async identityCreate(assetLockProof, assetLockPrivateKey, publicKeys) {
        ErrorUtils.validateRequired({ assetLockProof, assetLockPrivateKey, publicKeys }, 
                                   ['assetLockProof', 'assetLockPrivateKey', 'publicKeys']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_create(this.wasmSdk, assetLockProof, assetLockPrivateKey, publicKeys),
            'identity_create',
            { assetLockProof: '[REDACTED]', assetLockPrivateKey: '[REDACTED]', publicKeys: '[REDACTED]' }
        );
    }

    /**
     * Top up identity with additional credits
     * @param {string} identityId - Identity ID
     * @param {string} assetLockProof - Asset lock proof (hex-encoded JSON)
     * @param {string} assetLockPrivateKey - Asset lock private key (WIF)
     * @returns {Promise<Object>} Top up result
     */
    async identityTopUp(identityId, assetLockProof, assetLockPrivateKey) {
        ErrorUtils.validateRequired({ identityId, assetLockProof, assetLockPrivateKey }, 
                                   ['identityId', 'assetLockProof', 'assetLockPrivateKey']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_topup(this.wasmSdk, identityId, assetLockProof, assetLockPrivateKey),
            'identity_topup',
            { identityId, assetLockProof: '[REDACTED]', assetLockPrivateKey: '[REDACTED]' }
        );
    }

    /**
     * Update identity keys
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Identity ID to update
     * @param {string} updateData - JSON string of update operations
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Update result
     */
    async identityUpdate(mnemonic, identityId, updateData, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, updateData, keyIndex }, 
                                   ['mnemonic', 'identityId', 'updateData', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_update(this.wasmSdk, mnemonic, identityId, updateData, keyIndex),
            'identity_update',
            { mnemonic: '[REDACTED]', identityId, updateData: '[REDACTED]', keyIndex }
        );
    }

    /**
     * Withdraw credits from identity
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Identity ID
     * @param {string} toAddress - Destination address
     * @param {number} amount - Amount to withdraw
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Withdrawal result
     */
    async identityWithdraw(mnemonic, identityId, toAddress, amount, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, toAddress, amount, keyIndex }, 
                                   ['mnemonic', 'identityId', 'toAddress', 'amount', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_withdraw(this.wasmSdk, mnemonic, identityId, toAddress, amount, keyIndex),
            'identity_withdraw',
            { mnemonic: '[REDACTED]', identityId, toAddress, amount, keyIndex }
        );
    }

    /**
     * Create data contract
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractDefinition - JSON contract definition
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Contract creation result
     */
    async dataContractCreate(mnemonic, identityId, contractDefinition, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractDefinition, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractDefinition', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.data_contract_create(this.wasmSdk, mnemonic, identityId, contractDefinition, keyIndex),
            'data_contract_create',
            { mnemonic: '[REDACTED]', identityId, contractDefinition: '[REDACTED]', keyIndex }
        );
    }

    /**
     * Update data contract
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractId - Contract ID to update
     * @param {string} updateDefinition - JSON update definition
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Contract update result
     */
    async dataContractUpdate(mnemonic, identityId, contractId, updateDefinition, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractId, updateDefinition, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractId', 'updateDefinition', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.data_contract_update(this.wasmSdk, mnemonic, identityId, contractId, updateDefinition, keyIndex),
            'data_contract_update',
            { mnemonic: '[REDACTED]', identityId, contractId, updateDefinition: '[REDACTED]', keyIndex }
        );
    }

    /**
     * Create document
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractId - Contract ID
     * @param {string} documentType - Document type
     * @param {string} documentData - JSON document data
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Document creation result
     */
    async documentCreate(mnemonic, identityId, contractId, documentType, documentData, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractId, documentType, documentData, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractId', 'documentType', 'documentData', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.document_create(this.wasmSdk, mnemonic, identityId, contractId, documentType, documentData, keyIndex),
            'document_create',
            { mnemonic: '[REDACTED]', identityId, contractId, documentType, documentData: '[REDACTED]', keyIndex }
        );
    }

    /**
     * Update document
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractId - Contract ID
     * @param {string} documentType - Document type
     * @param {string} documentId - Document ID to update
     * @param {string} updateData - JSON update data
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Document update result
     */
    async documentUpdate(mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractId', 'documentType', 'documentId', 'updateData', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.document_update(this.wasmSdk, mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex),
            'document_update',
            { mnemonic: '[REDACTED]', identityId, contractId, documentType, documentId, updateData: '[REDACTED]', keyIndex }
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
     * @param {string} network - Network ('mainnet' or 'testnet')
     * @returns {Promise<Object>} Object containing private and public keys
     */
    async generateKeyPair(network = 'testnet') {
        ErrorUtils.validateRequired({ network }, ['network']);
        
        if (!['mainnet', 'testnet'].includes(network)) {
            throw new WasmOperationError(
                'Invalid network. Must be "mainnet" or "testnet"',
                'generate_key_pair',
                { network }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.generate_key_pair(network),
            'generate_key_pair',
            { network }
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

    // ========== DPNS Utility Operations ==========

    /**
     * Check if a username is valid for DPNS
     * @param {string} label - Username label to validate
     * @returns {Promise<boolean>} True if username is valid
     */
    async dpnsIsValidUsername(label) {
        ErrorUtils.validateRequired({ label }, ['label']);
        
        return this._executeOperation(
            () => this.wasmModule.dpns_is_valid_username(label),
            'dpns_is_valid_username',
            { label }
        );
    }

    /**
     * Convert input to homograph-safe format
     * @param {string} input - Input string to convert
     * @returns {Promise<string>} Homograph-safe version of input
     */
    async dpnsConvertToHomographSafe(input) {
        ErrorUtils.validateRequired({ input }, ['input']);
        
        return this._executeOperation(
            () => this.wasmModule.dpns_convert_to_homograph_safe(input),
            'dpns_convert_to_homograph_safe',
            { input }
        );
    }

    /**
     * Check if a username is contested
     * @param {string} label - Username label to check
     * @returns {Promise<boolean>} True if username is contested
     */
    async dpnsIsContestedUsername(label) {
        ErrorUtils.validateRequired({ label }, ['label']);
        
        return this._executeOperation(
            () => this.wasmModule.dpns_is_contested_username(label),
            'dpns_is_contested_username',
            { label }
        );
    }

    /**
     * Resolve a DPNS name to get associated data
     * @param {string} name - DPNS name to resolve
     * @returns {Promise<Object|null>} Resolved name data or null if not found
     */
    async dpnsResolveName(name) {
        ErrorUtils.validateRequired({ name }, ['name']);
        
        return this._executeOperation(
            () => this.wasmModule.dpns_resolve_name(this.wasmSdk, name),
            'dpns_resolve_name',
            { name }
        );
    }

    /**
     * Check if a DPNS name is available for registration
     * @param {string} label - Name label to check availability
     * @returns {Promise<boolean>} True if name is available
     */
    async dpnsIsNameAvailable(label) {
        ErrorUtils.validateRequired({ label }, ['label']);
        
        return this._executeOperation(
            () => this.wasmModule.dpns_is_name_available(this.wasmSdk, label),
            'dpns_is_name_available',
            { label }
        );
    }

    // ========== System & Status Query Operations ==========

    /**
     * Get platform status information
     * @returns {Promise<Object>} Platform status information
     */
    async getStatus() {
        return this._executeOperation(
            () => this.wasmModule.get_status(this.wasmSdk),
            'get_status'
        );
    }

    /**
     * Get current epoch number
     * @returns {Promise<number>} Current epoch number
     */
    async getCurrentEpoch() {
        return this._executeOperation(
            () => this.wasmModule.get_current_epoch(this.wasmSdk),
            'get_current_epoch'
        );
    }

    /**
     * Get epoch information for a range of epochs
     * @param {number} start - Starting epoch number
     * @param {number} count - Number of epochs to fetch
     * @param {boolean} ascending - Whether to return in ascending order (default: true)
     * @returns {Promise<Object[]>} Array of epoch information objects
     */
    async getEpochsInfo(start, count, ascending = true) {
        ErrorUtils.validateRequired({ start, count }, ['start', 'count']);
        
        if (typeof start !== 'number' || start < 0) {
            throw new WasmOperationError(
                'Start epoch must be a non-negative number',
                'get_epochs_info',
                { start }
            );
        }
        
        if (typeof count !== 'number' || count <= 0) {
            throw new WasmOperationError(
                'Count must be a positive number',
                'get_epochs_info',
                { count }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.get_epochs_info(this.wasmSdk, start, count, ascending),
            'get_epochs_info',
            { start, count, ascending }
        );
    }

    /**
     * Get current quorum information
     * @returns {Promise<Object>} Current quorum information
     */
    async getCurrentQuorumsInfo() {
        return this._executeOperation(
            () => this.wasmModule.get_current_quorums_info(this.wasmSdk),
            'get_current_quorums_info'
        );
    }

    /**
     * Get total credits in platform
     * @returns {Promise<number>} Total credits in platform
     */
    async getTotalCreditsInPlatform() {
        return this._executeOperation(
            () => this.wasmModule.get_total_credits_in_platform(this.wasmSdk),
            'get_total_credits_in_platform'
        );
    }

    /**
     * Get path elements for a given path
     * @param {string} path - Path to get elements for
     * @param {Array<string>} keys - Array of keys to retrieve
     * @returns {Promise<Object>} Path elements data
     */
    async getPathElements(path, keys) {
        ErrorUtils.validateRequired({ path, keys }, ['path', 'keys']);
        
        if (!Array.isArray(keys)) {
            throw new WasmOperationError(
                'Keys must be an array',
                'get_path_elements',
                { keys: typeof keys }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.get_path_elements(this.wasmSdk, path, keys),
            'get_path_elements',
            { path, keyCount: keys.length }
        );
    }

    // ========== Enhanced Identity Operations ==========

    /**
     * Get identity balance
     * @param {string} identityId - Identity ID
     * @returns {Promise<number>} Identity balance in credits
     */
    async getIdentityBalance(identityId) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_balance_with_proof_info' : 'get_identity_balance';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId),
            methodName,
            { identityId, proofs: useProofs }
        );
    }

    /**
     * Get identity keys
     * @param {string} identityId - Identity ID
     * @param {string} keyRequestType - Key request type ('all', 'specific', 'search')
     * @param {Array<number>} specificKeyIds - Specific key IDs (for 'specific' type)
     * @param {Object} searchPurposeMap - Search purpose map (for 'search' type)
     * @param {number} limit - Result limit
     * @param {number} offset - Result offset
     * @returns {Promise<Array>} Array of identity keys
     */
    async getIdentityKeys(identityId, keyRequestType = 'all', specificKeyIds = null, searchPurposeMap = null, limit = null, offset = null) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_keys_with_proof_info' : 'get_identity_keys';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, keyRequestType, specificKeyIds, searchPurposeMap, limit, offset),
            methodName,
            { identityId, keyRequestType, proofs: useProofs }
        );
    }

    /**
     * Get identity nonce
     * @param {string} identityId - Identity ID
     * @returns {Promise<number>} Identity nonce
     */
    async getIdentityNonce(identityId) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_nonce_with_proof_info' : 'get_identity_nonce';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId),
            methodName,
            { identityId, proofs: useProofs }
        );
    }

    /**
     * Get identity contract nonce
     * @param {string} identityId - Identity ID
     * @param {string} contractId - Contract ID
     * @returns {Promise<number>} Identity contract nonce
     */
    async getIdentityContractNonce(identityId, contractId) {
        ErrorUtils.validateRequired({ identityId, contractId }, ['identityId', 'contractId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_contract_nonce_with_proof_info' : 'get_identity_contract_nonce';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, contractId),
            methodName,
            { identityId, contractId, proofs: useProofs }
        );
    }

    /**
     * Get identity balance and revision
     * @param {string} identityId - Identity ID
     * @returns {Promise<Object>} Object containing balance and revision
     */
    async getIdentityBalanceAndRevision(identityId) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_balance_and_revision_with_proof_info' : 'get_identity_balance_and_revision';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId),
            methodName,
            { identityId, proofs: useProofs }
        );
    }

    /**
     * Get identity by unique public key hash
     * @param {string} publicKeyHash - Public key hash
     * @returns {Promise<Object|null>} Identity or null if not found
     */
    async getIdentityByPublicKeyHash(publicKeyHash) {
        ErrorUtils.validateRequired({ publicKeyHash }, ['publicKeyHash']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_by_public_key_hash_with_proof_info' : 'get_identity_by_public_key_hash';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, publicKeyHash),
            methodName,
            { publicKeyHash, proofs: useProofs }
        );
    }

    /**
     * Get identity by non-unique public key hash
     * @param {string} publicKeyHash - Public key hash
     * @param {string} startAfter - Start after identity ID (for pagination)
     * @returns {Promise<Array>} Array of identities with this public key hash
     */
    async getIdentityByNonUniquePublicKeyHash(publicKeyHash, startAfter = null) {
        ErrorUtils.validateRequired({ publicKeyHash }, ['publicKeyHash']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_by_non_unique_public_key_hash_with_proof_info' : 'get_identity_by_non_unique_public_key_hash';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, publicKeyHash, startAfter),
            methodName,
            { publicKeyHash, startAfter, proofs: useProofs }
        );
    }

    /**
     * Get balances for multiple identities
     * @param {Array<string>} identityIds - Array of identity IDs
     * @returns {Promise<Object>} Object mapping identity IDs to balances
     */
    async getIdentitiesBalances(identityIds) {
        ErrorUtils.validateRequired({ identityIds }, ['identityIds']);
        
        if (!Array.isArray(identityIds)) {
            throw new WasmOperationError(
                'identityIds must be an array',
                'get_identities_balances',
                { identityIds: typeof identityIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identities_balances_with_proof_info' : 'get_identities_balances';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityIds),
            methodName,
            { identityCount: identityIds.length, proofs: useProofs }
        );
    }

    /**
     * Get contract keys for multiple identities
     * @param {Array<string>} identityIds - Array of identity IDs
     * @param {string} contractId - Contract ID
     * @param {string} documentTypeName - Document type name (optional)
     * @param {Array<number>} purposes - Key purposes (optional)
     * @returns {Promise<Object>} Contract keys for identities
     */
    async getIdentitiesContractKeys(identityIds, contractId, documentTypeName = null, purposes = null) {
        ErrorUtils.validateRequired({ identityIds, contractId }, ['identityIds', 'contractId']);
        
        if (!Array.isArray(identityIds)) {
            throw new WasmOperationError(
                'identityIds must be an array',
                'get_identities_contract_keys',
                { identityIds: typeof identityIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identities_contract_keys_with_proof_info' : 'get_identities_contract_keys';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityIds, contractId, documentTypeName, purposes),
            methodName,
            { identityCount: identityIds.length, contractId, documentTypeName, proofs: useProofs }
        );
    }

    /**
     * Get token balances for an identity
     * @param {string} identityId - Identity ID
     * @param {Array<string>} tokenIds - Array of token IDs
     * @returns {Promise<Object>} Token balances for the identity
     */
    async getIdentityTokenBalances(identityId, tokenIds) {
        ErrorUtils.validateRequired({ identityId, tokenIds }, ['identityId', 'tokenIds']);
        
        if (!Array.isArray(tokenIds)) {
            throw new WasmOperationError(
                'tokenIds must be an array',
                'get_identity_token_balances',
                { tokenIds: typeof tokenIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_token_balances_with_proof_info' : 'get_identity_token_balances';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, tokenIds),
            methodName,
            { identityId, tokenCount: tokenIds.length, proofs: useProofs }
        );
    }

    /**
     * Get token information for an identity
     * @param {string} identityId - Identity ID
     * @param {Array<string>} tokenIds - Array of token IDs
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Array>} Token information for the identity
     */
    async getIdentityTokenInfos(identityId, tokenIds, limit = null, offset = null) {
        ErrorUtils.validateRequired({ identityId, tokenIds }, ['identityId', 'tokenIds']);
        
        if (!Array.isArray(tokenIds)) {
            throw new WasmOperationError(
                'tokenIds must be an array',
                'get_identity_token_infos',
                { tokenIds: typeof tokenIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_token_infos_with_proof_info' : 'get_identity_token_infos';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, tokenIds, limit, offset),
            methodName,
            { identityId, tokenCount: tokenIds.length, limit, offset, proofs: useProofs }
        );
    }

    /**
     * Get token balances for multiple identities
     * @param {Array<string>} identityIds - Array of identity IDs
     * @param {string} tokenId - Token ID
     * @returns {Promise<Object>} Token balances for identities
     */
    async getIdentitiesTokenBalances(identityIds, tokenId) {
        ErrorUtils.validateRequired({ identityIds, tokenId }, ['identityIds', 'tokenId']);
        
        if (!Array.isArray(identityIds)) {
            throw new WasmOperationError(
                'identityIds must be an array',
                'get_identities_token_balances',
                { identityIds: typeof identityIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identities_token_balances_with_proof_info' : 'get_identities_token_balances';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityIds, tokenId),
            methodName,
            { identityCount: identityIds.length, tokenId, proofs: useProofs }
        );
    }

    // ========== Token Operations ==========

    /**
     * Get token statuses
     * @param {Array<string>} tokenIds - Array of token IDs
     * @returns {Promise<Object>} Token statuses
     */
    async getTokenStatuses(tokenIds) {
        ErrorUtils.validateRequired({ tokenIds }, ['tokenIds']);
        
        if (!Array.isArray(tokenIds)) {
            throw new WasmOperationError(
                'tokenIds must be an array',
                'get_token_statuses',
                { tokenIds: typeof tokenIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_token_statuses_with_proof_info' : 'get_token_statuses';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, tokenIds),
            methodName,
            { tokenCount: tokenIds.length, proofs: useProofs }
        );
    }

    /**
     * Get token direct purchase prices
     * @param {Array<string>} tokenIds - Array of token IDs
     * @returns {Promise<Object>} Token direct purchase prices
     */
    async getTokenDirectPurchasePrices(tokenIds) {
        ErrorUtils.validateRequired({ tokenIds }, ['tokenIds']);
        
        if (!Array.isArray(tokenIds)) {
            throw new WasmOperationError(
                'tokenIds must be an array',
                'get_token_direct_purchase_prices',
                { tokenIds: typeof tokenIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_token_direct_purchase_prices_with_proof_info' : 'get_token_direct_purchase_prices';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, tokenIds),
            methodName,
            { tokenCount: tokenIds.length, proofs: useProofs }
        );
    }

    /**
     * Get token contract information
     * @param {string} contractId - Contract ID
     * @returns {Promise<Object>} Token contract information
     */
    async getTokenContractInfo(contractId) {
        ErrorUtils.validateRequired({ contractId }, ['contractId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_token_contract_info_with_proof_info' : 'get_token_contract_info';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, contractId),
            methodName,
            { contractId, proofs: useProofs }
        );
    }

    /**
     * Get token total supply
     * @param {string} tokenId - Token ID
     * @returns {Promise<number>} Token total supply
     */
    async getTokenTotalSupply(tokenId) {
        ErrorUtils.validateRequired({ tokenId }, ['tokenId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_token_total_supply_with_proof_info' : 'get_token_total_supply';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, tokenId),
            methodName,
            { tokenId, proofs: useProofs }
        );
    }

    /**
     * Get token price by contract
     * @param {string} contractId - Contract ID
     * @param {number} tokenPosition - Token position in contract
     * @returns {Promise<Object>} Token price information
     */
    async getTokenPriceByContract(contractId, tokenPosition) {
        ErrorUtils.validateRequired({ contractId, tokenPosition }, ['contractId', 'tokenPosition']);
        
        if (typeof tokenPosition !== 'number') {
            throw new WasmOperationError(
                'tokenPosition must be a number',
                'get_token_price_by_contract',
                { tokenPosition: typeof tokenPosition }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_token_price_by_contract_with_proof_info' : 'get_token_price_by_contract';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, contractId, tokenPosition),
            methodName,
            { contractId, tokenPosition, proofs: useProofs }
        );
    }

    /**
     * Calculate token ID from contract
     * @param {string} contractId - Contract ID
     * @param {number} tokenPosition - Token position in contract
     * @returns {Promise<string>} Calculated token ID
     */
    async calculateTokenIdFromContract(contractId, tokenPosition) {
        ErrorUtils.validateRequired({ contractId, tokenPosition }, ['contractId', 'tokenPosition']);
        
        if (typeof tokenPosition !== 'number') {
            throw new WasmOperationError(
                'tokenPosition must be a number',
                'calculate_token_id_from_contract',
                { tokenPosition: typeof tokenPosition }
            );
        }
        
        return this._executeOperation(
            () => this.wasmModule.calculate_token_id_from_contract(contractId, tokenPosition),
            'calculate_token_id_from_contract',
            { contractId, tokenPosition }
        );
    }

    /**
     * Get token perpetual distribution last claim
     * @param {string} identityId - Identity ID
     * @param {string} tokenId - Token ID
     * @returns {Promise<Object>} Last claim information
     */
    async getTokenPerpetualDistributionLastClaim(identityId, tokenId) {
        ErrorUtils.validateRequired({ identityId, tokenId }, ['identityId', 'tokenId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_token_perpetual_distribution_last_claim_with_proof_info' : 'get_token_perpetual_distribution_last_claim';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, tokenId),
            methodName,
            { identityId, tokenId, proofs: useProofs }
        );
    }

    /**
     * Get token information for multiple identities
     * @param {Array<string>} identityIds - Array of identity IDs
     * @param {string} tokenId - Token ID
     * @returns {Promise<Object>} Token information for identities
     */
    async getIdentitiesTokenInfos(identityIds, tokenId) {
        ErrorUtils.validateRequired({ identityIds, tokenId }, ['identityIds', 'tokenId']);
        
        if (!Array.isArray(identityIds)) {
            throw new WasmOperationError(
                'identityIds must be an array',
                'get_identities_token_infos',
                { identityIds: typeof identityIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identities_token_infos_with_proof_info' : 'get_identities_token_infos';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityIds, tokenId),
            methodName,
            { identityCount: identityIds.length, tokenId, proofs: useProofs }
        );
    }

    // ========== Group Operations ==========

    /**
     * Get group information
     * @param {string} groupId - Group ID
     * @returns {Promise<Object>} Group information
     */
    async getGroupInfo(groupId) {
        ErrorUtils.validateRequired({ groupId }, ['groupId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_group_info_with_proof_info' : 'get_group_info';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, groupId),
            methodName,
            { groupId, proofs: useProofs }
        );
    }

    /**
     * Get information for multiple groups
     * @param {Array<string>} groupIds - Array of group IDs
     * @returns {Promise<Object>} Group information for multiple groups
     */
    async getGroupInfos(groupIds) {
        ErrorUtils.validateRequired({ groupIds }, ['groupIds']);
        
        if (!Array.isArray(groupIds)) {
            throw new WasmOperationError(
                'groupIds must be an array',
                'get_group_infos',
                { groupIds: typeof groupIds }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_group_infos_with_proof_info' : 'get_group_infos';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, groupIds),
            methodName,
            { groupCount: groupIds.length, proofs: useProofs }
        );
    }

    /**
     * Get group members
     * @param {string} groupId - Group ID
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Array>} Array of group members
     */
    async getGroupMembers(groupId, limit = null, offset = null) {
        ErrorUtils.validateRequired({ groupId }, ['groupId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_group_members_with_proof_info' : 'get_group_members';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, groupId, limit, offset),
            methodName,
            { groupId, limit, offset, proofs: useProofs }
        );
    }

    /**
     * Get groups for an identity
     * @param {string} identityId - Identity ID
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Array>} Array of groups the identity belongs to
     */
    async getIdentityGroups(identityId, limit = null, offset = null) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_identity_groups_with_proof_info' : 'get_identity_groups';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, limit, offset),
            methodName,
            { identityId, limit, offset, proofs: useProofs }
        );
    }

    // ========== Voting & Contested Resources ==========

    /**
     * Get contested resources
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Array>} Array of contested resources
     */
    async getContestedResources(limit = null, offset = null) {
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_contested_resources_with_proof_info' : 'get_contested_resources';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, limit, offset),
            methodName,
            { limit, offset, proofs: useProofs }
        );
    }

    /**
     * Get contested resource vote state
     * @param {string} contractId - Contract ID
     * @param {string} documentTypeName - Document type name
     * @param {string} indexName - Index name
     * @param {Array} indexValues - Index values
     * @returns {Promise<Object>} Vote state for contested resource
     */
    async getContestedResourceVoteState(contractId, documentTypeName, indexName, indexValues) {
        ErrorUtils.validateRequired({ contractId, documentTypeName, indexName, indexValues }, 
                                   ['contractId', 'documentTypeName', 'indexName', 'indexValues']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_contested_resource_vote_state_with_proof_info' : 'get_contested_resource_vote_state';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, contractId, documentTypeName, indexName, indexValues),
            methodName,
            { contractId, documentTypeName, indexName, proofs: useProofs }
        );
    }

    /**
     * Get contested resource voters for identity
     * @param {string} identityId - Identity ID
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Array>} Array of contested resource votes for identity
     */
    async getContestedResourceVotersForIdentity(identityId, limit = null, offset = null) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_contested_resource_voters_for_identity_with_proof_info' : 'get_contested_resource_voters_for_identity';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId, limit, offset),
            methodName,
            { identityId, limit, offset, proofs: useProofs }
        );
    }

    /**
     * Get vote polls by end date
     * @param {number} startTimeMs - Start time in milliseconds
     * @param {number} endTimeMs - End time in milliseconds
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Array>} Array of vote polls
     */
    async getVotePollsByEndDate(startTimeMs, endTimeMs, limit = null, offset = null) {
        ErrorUtils.validateRequired({ startTimeMs, endTimeMs }, ['startTimeMs', 'endTimeMs']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_vote_polls_by_end_date_with_proof_info' : 'get_vote_polls_by_end_date';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, startTimeMs, endTimeMs, limit, offset),
            methodName,
            { startTimeMs, endTimeMs, limit, offset, proofs: useProofs }
        );
    }

    // ========== Protocol & Version Queries ==========

    /**
     * Get protocol version upgrade state
     * @returns {Promise<Object>} Protocol version upgrade state
     */
    async getProtocolVersionUpgradeState() {
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_protocol_version_upgrade_state_with_proof_info' : 'get_protocol_version_upgrade_state';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk),
            methodName,
            { proofs: useProofs }
        );
    }

    /**
     * Get protocol version upgrade vote status
     * @param {string} startIdentityId - Start identity ID (for pagination)
     * @param {number} limit - Result limit (optional)
     * @param {number} offset - Result offset (optional)
     * @returns {Promise<Object>} Protocol version upgrade vote status
     */
    async getProtocolVersionUpgradeVoteStatus(startIdentityId = null, limit = null, offset = null) {
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_protocol_version_upgrade_vote_status_with_proof_info' : 'get_protocol_version_upgrade_vote_status';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, startIdentityId, limit, offset),
            methodName,
            { startIdentityId, limit, offset, proofs: useProofs }
        );
    }

    /**
     * Get prefunded specialized balance
     * @param {string} identityId - Identity ID
     * @returns {Promise<number>} Prefunded specialized balance
     */
    async getPrefundedSpecializedBalance(identityId) {
        ErrorUtils.validateRequired({ identityId }, ['identityId']);
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_prefunded_specialized_balance_with_proof_info' : 'get_prefunded_specialized_balance';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, identityId),
            methodName,
            { identityId, proofs: useProofs }
        );
    }

    // ========== Additional Utility Functions ==========

    /**
     * Get finalized epoch information
     * @param {number} count - Number of epochs to retrieve
     * @param {boolean} ascending - Whether to sort in ascending order
     * @returns {Promise<Array>} Array of finalized epoch information
     */
    async getFinalizedEpochInfos(count, ascending = true) {
        ErrorUtils.validateRequired({ count }, ['count']);
        
        if (typeof count !== 'number' || count <= 0) {
            throw new WasmOperationError(
                'Count must be a positive number',
                'get_finalized_epoch_infos',
                { count }
            );
        }
        
        const useProofs = this.configManager.getProofs();
        const methodName = useProofs ? 'get_finalized_epoch_infos_with_proof_info' : 'get_finalized_epoch_infos';
        
        return this._executeOperation(
            () => this.wasmModule[methodName](this.wasmSdk, count, ascending),
            methodName,
            { count, ascending, proofs: useProofs }
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