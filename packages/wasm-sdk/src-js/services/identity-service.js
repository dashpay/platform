/**
 * Identity Service - Handles all identity-related operations
 * Extracted from monolithic WasmSDK class for better organization
 */

import { ErrorUtils } from '../error-handler.js';

export class IdentityService {
    /**
     * Create identity service
     * @param {Object} wasmSdkWrapper - Reference to main WasmSDK wrapper instance
     * @param {Object} wasmSdkInstance - Raw WASM SDK instance
     * @param {Object} wasmModule - WASM module for operations
     * @param {Object} configManager - Configuration manager
     */
    constructor(wasmSdkWrapper, wasmSdkInstance, wasmModule, configManager) {
        this.wasmSdkWrapper = wasmSdkWrapper;
        this.wasmSdk = wasmSdkInstance;
        this.wasmModule = wasmModule;
        this.configManager = configManager;
    }

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
     * Create identity with asset lock proof
     * @param {string} assetLockProof - Asset lock proof (hex-encoded JSON)
     * @param {string} assetLockPrivateKey - Asset lock private key (WIF)
     * @param {string} publicKeys - JSON string of public keys array
     * @returns {Promise<Object>} Identity creation result
     */
    async createIdentity(assetLockProof, assetLockPrivateKey, publicKeys) {
        ErrorUtils.validateRequired({ assetLockProof, assetLockPrivateKey, publicKeys }, 
                                   ['assetLockProof', 'assetLockPrivateKey', 'publicKeys']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_create(this.wasmSdk, assetLockProof, assetLockPrivateKey, publicKeys),
            'identity_create',
            { assetLockProof: '[SANITIZED]', assetLockPrivateKey: '[SANITIZED]', publicKeys: '[SANITIZED]' }
        );
    }

    /**
     * Top up identity with additional credits
     * @param {string} identityId - Identity ID
     * @param {string} assetLockProof - Asset lock proof (hex-encoded JSON)
     * @param {string} assetLockPrivateKey - Asset lock private key (WIF)
     * @returns {Promise<Object>} Top up result
     */
    async topUpIdentity(identityId, assetLockProof, assetLockPrivateKey) {
        ErrorUtils.validateRequired({ identityId, assetLockProof, assetLockPrivateKey }, 
                                   ['identityId', 'assetLockProof', 'assetLockPrivateKey']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_topup(this.wasmSdk, identityId, assetLockProof, assetLockPrivateKey),
            'identity_topup',
            { identityId, assetLockProof: '[SANITIZED]', assetLockPrivateKey: '[SANITIZED]' }
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
    async updateIdentity(mnemonic, identityId, updateData, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, updateData, keyIndex }, 
                                   ['mnemonic', 'identityId', 'updateData', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_update(this.wasmSdk, mnemonic, identityId, updateData, keyIndex),
            'identity_update',
            { mnemonic: '[SANITIZED]', identityId, updateData: '[SANITIZED]', keyIndex }
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
    async withdrawFromIdentity(mnemonic, identityId, toAddress, amount, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, toAddress, amount, keyIndex }, 
                                   ['mnemonic', 'identityId', 'toAddress', 'amount', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.identity_withdraw(this.wasmSdk, mnemonic, identityId, toAddress, amount, keyIndex),
            'identity_withdraw',
            { mnemonic: '[SANITIZED]', identityId, toAddress, amount, keyIndex }
        );
    }

    /**
     * Execute operation with proper error handling
     * @private
     * @param {Function} operation - Operation to execute
     * @param {string} operationName - Name of operation for error context
     * @param {Object} context - Additional context for errors (will be sanitized)
     * @returns {Promise<*>} Operation result
     */
    async _executeOperation(operation, operationName, context = {}) {
        return this.wasmSdkWrapper._executeOperation(operation, operationName, this._sanitizeContext(context));
    }

    /**
     * Sanitize context to prevent sensitive data exposure
     * @private
     * @param {Object} context - Context to sanitize
     * @returns {Object} Sanitized context
     */
    _sanitizeContext(context) {
        const sensitive = ['mnemonic', 'privateKey', 'assetLockPrivateKey', 'publicKeys', 'updateData'];
        const sanitized = { ...context };
        
        sensitive.forEach(key => {
            if (sanitized[key] && sanitized[key] !== '[SANITIZED]') {
                sanitized[key] = '[SANITIZED]';
            }
        });
        
        return sanitized;
    }
}