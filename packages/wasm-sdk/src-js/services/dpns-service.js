/**
 * DPNS Service - Handles Dash Platform Naming Service operations
 * Extracted from monolithic WasmSDK class for better organization
 */

import { ErrorUtils } from '../error-handler.js';

export class DPNSService {
    /**
     * Create DPNS service
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
     * Check if a username is valid for DPNS
     * @param {string} label - Username label to validate
     * @returns {Promise<boolean>} True if username is valid
     */
    async isValidUsername(label) {
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
    async convertToHomographSafe(input) {
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
    async isContestedUsername(label) {
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
    async resolveName(name) {
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
    async isNameAvailable(label) {
        ErrorUtils.validateRequired({ label }, ['label']);
        
        return this._executeOperation(
            () => this.wasmModule.dpns_is_name_available(this.wasmSdk, label),
            'dpns_is_name_available',
            { label }
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