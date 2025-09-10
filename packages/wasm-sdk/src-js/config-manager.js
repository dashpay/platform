/**
 * Simplified Configuration management for WASM SDK
 * Handles essential configuration with security focus, delegates endpoint discovery to WASM SDK
 */

import { WasmConfigurationError } from './error-handler.js';

/**
 * Default configuration values - simplified and essential only
 */
const DEFAULT_CONFIG = {
    network: 'testnet',
    proofs: true,
    debug: false,
    transport: {
        timeout: 30000,
        retries: 3,
        retryDelay: 1000
    }
};

/**
 * Simplified Configuration manager class
 * Focuses on essential validation and delegates endpoint management to WASM SDK
 */
export class ConfigManager {
    /**
     * Create a configuration manager
     * @param {Object} userConfig - User provided configuration
     */
    constructor(userConfig = {}) {
        this.config = this._mergeConfig(userConfig);
        this._validateEssentials();
    }

    /**
     * Get the current configuration
     * @returns {Object} Current configuration
     */
    getConfig() {
        return { ...this.config };
    }

    /**
     * Get network configuration
     * @returns {string} Current network
     */
    getNetwork() {
        return this.config.network;
    }

    /**
     * Get transport configuration
     * @returns {Object} Transport configuration
     */
    getTransport() {
        return { ...this.config.transport };
    }

    /**
     * Get proof verification setting
     * @returns {boolean} Proof verification enabled
     */
    getProofs() {
        return this.config.proofs;
    }

    /**
     * Get debug mode setting
     * @returns {boolean} Debug mode enabled
     */
    getDebug() {
        return this.config.debug;
    }

    /**
     * Get custom endpoint if provided by user
     * @returns {string|null} Custom endpoint URL or null
     */
    getCustomEndpoint() {
        return this.config.transport.customEndpoint || null;
    }

    /**
     * Check if user provided a custom endpoint
     * @returns {boolean} True if custom endpoint provided
     */
    hasCustomEndpoint() {
        return !!this.config.transport.customEndpoint;
    }

    /**
     * Update configuration
     * @param {Object} updates - Configuration updates
     */
    updateConfig(updates) {
        const newConfig = this._mergeConfig(updates, this.config);
        this._validateEssentials(newConfig);
        this.config = newConfig;
    }

    /**
     * Merge user configuration with defaults using efficient shallow merge
     * @private
     * @param {Object} userConfig - User configuration
     * @param {Object} baseConfig - Base configuration to merge with
     * @returns {Object} Merged configuration
     */
    _mergeConfig(userConfig, baseConfig = DEFAULT_CONFIG) {
        // Efficient shallow merge instead of expensive JSON deep clone
        return {
            network: userConfig.network || baseConfig.network,
            proofs: userConfig.proofs !== undefined ? userConfig.proofs : baseConfig.proofs,
            debug: userConfig.debug !== undefined ? userConfig.debug : baseConfig.debug,
            transport: {
                ...baseConfig.transport,
                ...userConfig.transport,
                // Handle custom endpoint from user's transport.url (check for undefined, not falsy)
                customEndpoint: userConfig.transport?.url !== undefined ? userConfig.transport.url : baseConfig.transport?.customEndpoint
            }
        };
    }

    /**
     * Validate essential configuration - simplified and security-focused
     * @private
     * @param {Object} config - Configuration to validate (optional, uses this.config if not provided)
     * @throws {WasmConfigurationError} If configuration is invalid
     */
    _validateEssentials(config = this.config) {
        // Validate network
        if (!['testnet', 'mainnet'].includes(config.network)) {
            throw new WasmConfigurationError(
                `Invalid network: ${config.network}. Must be 'testnet' or 'mainnet'`,
                'network',
                config.network
            );
        }

        // Validate boolean fields
        if (typeof config.proofs !== 'boolean') {
            throw new WasmConfigurationError(
                'proofs must be a boolean',
                'proofs',
                config.proofs
            );
        }

        if (typeof config.debug !== 'boolean') {
            throw new WasmConfigurationError(
                'debug must be a boolean',
                'debug',
                config.debug
            );
        }

        // Validate transport settings
        if (typeof config.transport !== 'object' || config.transport === null) {
            throw new WasmConfigurationError(
                'transport must be an object',
                'transport',
                config.transport
            );
        }

        // Validate timeout range
        const timeout = config.transport.timeout;
        if (typeof timeout !== 'number' || timeout < 1000 || timeout > 300000) {
            throw new WasmConfigurationError(
                'transport.timeout must be a number between 1000 and 300000 milliseconds',
                'transport.timeout',
                timeout
            );
        }

        // Validate retries range
        const retries = config.transport.retries;
        if (typeof retries !== 'number' || retries < 0 || retries > 10) {
            throw new WasmConfigurationError(
                'transport.retries must be a number between 0 and 10',
                'transport.retries',
                retries
            );
        }

        // Validate custom endpoint if provided and not empty (security-focused)
        if (config.transport.customEndpoint !== undefined && config.transport.customEndpoint !== null) {
            if (config.transport.customEndpoint === '') {
                throw new WasmConfigurationError(
                    'Custom endpoint cannot be empty string',
                    'transport.customEndpoint',
                    config.transport.customEndpoint
                );
            }
            this._validateCustomEndpoint(config.transport.customEndpoint);
        }
    }

    /**
     * Validate custom endpoint URL with security focus
     * @private
     * @param {string} url - URL to validate
     * @throws {WasmConfigurationError} If URL is invalid or insecure
     */
    _validateCustomEndpoint(url) {
        if (typeof url !== 'string') {
            throw new WasmConfigurationError(
                'Custom endpoint must be a string',
                'transport.customEndpoint',
                url
            );
        }

        let parsedUrl;
        try {
            parsedUrl = new URL(url);
        } catch (error) {
            throw new WasmConfigurationError(
                'Custom endpoint must be a valid URL',
                'transport.customEndpoint',
                url
            );
        }

        // Security: Enforce HTTPS
        if (parsedUrl.protocol !== 'https:') {
            throw new WasmConfigurationError(
                'Custom endpoint must use HTTPS protocol for security',
                'transport.customEndpoint',
                url
            );
        }

        // Security: Validate port range (Dash Platform typically uses 1443)
        const port = parsedUrl.port;
        if (port && (parseInt(port) < 1 || parseInt(port) > 65535)) {
            throw new WasmConfigurationError(
                'Custom endpoint port must be between 1 and 65535',
                'transport.customEndpoint',
                url
            );
        }

        // Security: Basic hostname validation (no localhost, internal IPs in production)
        if (parsedUrl.hostname === 'localhost' || parsedUrl.hostname.startsWith('127.')) {
            console.warn('Warning: Using localhost endpoint - only suitable for development');
        }
    }

    /**
     * Create a default configuration
     * @static
     * @returns {Object} Default configuration
     */
    static createDefault() {
        return { ...DEFAULT_CONFIG };
    }

    /**
     * Validate a configuration object without creating an instance
     * @static
     * @param {Object} config - Configuration to validate
     * @returns {Object} Validated configuration
     * @throws {WasmConfigurationError} If configuration is invalid
     */
    static validate(config) {
        const manager = new ConfigManager(config);
        return manager.getConfig();
    }
}

/**
 * Configuration utilities - simplified
 */
export const ConfigUtils = {
    /**
     * Create testnet configuration
     * @param {Object} overrides - Configuration overrides
     * @returns {Object} Testnet configuration
     */
    createTestnetConfig: (overrides = {}) => {
        return { network: 'testnet', ...overrides };
    },

    /**
     * Create mainnet configuration
     * @param {Object} overrides - Configuration overrides
     * @returns {Object} Mainnet configuration
     */
    createMainnetConfig: (overrides = {}) => {
        return { network: 'mainnet', ...overrides };
    },

    /**
     * Create configuration with custom endpoint
     * @param {string} url - Custom endpoint URL
     * @param {Object} overrides - Additional configuration overrides
     * @returns {Object} Configuration with custom endpoint
     */
    createCustomEndpointConfig: (url, overrides = {}) => {
        return {
            transport: { url },
            ...overrides
        };
    }
};