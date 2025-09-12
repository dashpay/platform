/**
 * Contract Service - Handles all data contract operations
 * Extracted from monolithic WasmSDK class for better organization
 */

import { ErrorUtils } from '../error-handler.js';

export class ContractService {
    /**
     * Create contract service
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
     * Create data contract
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractDefinition - JSON contract definition
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Contract creation result
     */
    async createDataContract(mnemonic, identityId, contractDefinition, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractDefinition, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractDefinition', 'keyIndex']);
        
        // Derive private key from mnemonic using working WASM function
        let privateKeyWif;
        
        if (mnemonic.length === 51 || mnemonic.length === 52) {
            // Already a WIF private key
            privateKeyWif = mnemonic;
        } else {
            // Derive from mnemonic
            try {
                const derivationPath = `m/44'/5'/0'/0/${keyIndex}`;
                const keyResult = await this._executeOperation(
                    () => this.wasmModule.derive_key_from_seed_with_path(mnemonic, null, derivationPath, 'testnet'),
                    'derive_key_from_seed_with_path',
                    { mnemonic: '[SANITIZED]', path: derivationPath, keyIndex }
                );
                privateKeyWif = keyResult.private_key_wif;
            } catch (keyError) {
                throw new Error(`Failed to derive private key: ${keyError.message}`);
            }
        }

        return this._executeOperation(
            () => this.wasmSdk.contractCreate(identityId, contractDefinition, privateKeyWif, keyIndex),
            'contractCreate',
            { identityId, contractDefinition: '[SANITIZED]', keyIndex, privateKey: '[SANITIZED]' }
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
    async updateDataContract(mnemonic, identityId, contractId, updateDefinition, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractId, updateDefinition, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractId', 'updateDefinition', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmSdk.contractUpdate(contractId, identityId, updateDefinition, mnemonic, keyIndex),
            'contractUpdate',
            { mnemonic: '[SANITIZED]', identityId, contractId, updateDefinition: '[SANITIZED]', keyIndex }
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
            'validate_document',
            { hasDocument: !!document, hasContract: !!dataContract }
        );
    }

    /**
     * Execute operation with proper error handling
     * @private
     * @param {Function} operation - Operation to execute
     * @param {string} operationName - Name of operation for error context
     * @param {Object} context - Additional context for errors
     * @returns {Promise<*>} Operation result
     */
    async _executeOperation(operation, operationName, context = {}) {
        return this.wasmSdkWrapper._executeOperation(operation, operationName, context);
    }
}