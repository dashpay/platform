/**
 * System Service - Handles platform system operations
 * Extracted from monolithic WasmSDK class for better organization
 */

import { ErrorUtils, WasmOperationError } from '../error-handler.js';

export class SystemService {
    /**
     * Create system service
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