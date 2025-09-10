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
        
        return this._executeOperation(
            () => this.wasmModule.data_contract_create(this.wasmSdk, mnemonic, identityId, contractDefinition, keyIndex),
            'data_contract_create',
            { mnemonic: '[SANITIZED]', identityId, contractDefinition: '[SANITIZED]', keyIndex }
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
            () => this.wasmModule.data_contract_update(this.wasmSdk, mnemonic, identityId, contractId, updateDefinition, keyIndex),
            'data_contract_update',
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