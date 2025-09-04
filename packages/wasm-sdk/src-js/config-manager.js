/**
 * Configuration management and validation for WASM SDK
 * Handles network configuration, transport settings, and parameter validation
 */

import { WasmConfigurationError, ErrorUtils } from './error-handler.js';

/**
 * Default endpoints for different networks
 */
const DEFAULT_ENDPOINTS = {
    testnet: [
        'https://52.12.176.90:1443/',
        'https://54.191.132.137:1443/',
        'https://18.144.106.20:1443/'
    ],
    mainnet: [
        'https://54.186.248.81:1443/',
        'https://52.12.176.90:1443/',
        'https://54.191.132.137:1443/'
    ]
};

/**
 * Default configuration values
 */
const DEFAULT_CONFIG = {
    network: 'testnet',
    proofs: true,
    debug: false,
    transport: {
        timeout: 30000,
        retries: 3,
        retryDelay: 1000,
        keepAlive: true
    }
};

/**
 * Configuration schema for validation
 */
const CONFIG_SCHEMA = {
    network: {
        type: 'string',
        enum: ['testnet', 'mainnet'],
        required: false
    },
    proofs: {
        type: 'boolean',
        required: false
    },
    debug: {
        type: 'boolean',
        required: false
    },
    transport: {
        type: 'object',
        required: false,
        properties: {
            url: {
                type: 'string',
                required: false
            },
            urls: {
                type: 'array',
                items: { type: 'string' },
                required: false
            },
            timeout: {
                type: 'number',
                min: 1000,
                max: 300000,
                required: false
            },
            retries: {
                type: 'number',
                min: 0,
                max: 10,
                required: false
            },
            retryDelay: {
                type: 'number',
                min: 100,
                max: 10000,
                required: false
            },
            keepAlive: {
                type: 'boolean',
                required: false
            }
        }
    }
};

/**
 * Configuration manager class
 */
export class ConfigManager {
    /**
     * Create a configuration manager
     * @param {Object} userConfig - User provided configuration
     */
    constructor(userConfig = {}) {
        this.config = this._mergeConfig(userConfig);
        this._validateConfig(this.config);
        this._resolveEndpoints();
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
     * Get primary endpoint URL
     * @returns {string} Primary endpoint URL
     */
    getPrimaryEndpoint() {
        return this.config.transport.urls[0];
    }

    /**
     * Get all endpoint URLs
     * @returns {string[]} All endpoint URLs
     */
    getAllEndpoints() {
        return [...this.config.transport.urls];
    }

    /**
     * Update configuration
     * @param {Object} updates - Configuration updates
     */
    updateConfig(updates) {
        const newConfig = this._mergeConfig(updates, this.config);
        this._validateConfig(newConfig);
        this.config = newConfig;
        this._resolveEndpoints();
    }

    /**
     * Merge user configuration with defaults
     * @private
     * @param {Object} userConfig - User configuration
     * @param {Object} baseConfig - Base configuration to merge with
     * @returns {Object} Merged configuration
     */
    _mergeConfig(userConfig, baseConfig = DEFAULT_CONFIG) {
        const merged = JSON.parse(JSON.stringify(baseConfig));
        
        if (userConfig.network !== undefined) {
            merged.network = userConfig.network;
        }
        
        if (userConfig.proofs !== undefined) {
            merged.proofs = userConfig.proofs;
        }
        
        if (userConfig.debug !== undefined) {
            merged.debug = userConfig.debug;
        }
        
        if (userConfig.transport) {
            merged.transport = { ...merged.transport, ...userConfig.transport };
        }
        
        return merged;
    }

    /**
     * Validate configuration against schema
     * @private
     * @param {Object} config - Configuration to validate
     * @throws {WasmConfigurationError} If configuration is invalid
     */
    _validateConfig(config) {
        this._validateObject(config, CONFIG_SCHEMA, '');
    }

    /**
     * Validate an object against a schema
     * @private
     * @param {*} value - Value to validate
     * @param {Object} schema - Schema to validate against
     * @param {string} path - Current path for error reporting
     */
    _validateObject(value, schema, path) {
        for (const [key, fieldSchema] of Object.entries(schema)) {
            const fieldPath = path ? `${path}.${key}` : key;
            const fieldValue = value[key];
            
            // Check required fields
            if (fieldSchema.required && (fieldValue === undefined || fieldValue === null)) {
                throw new WasmConfigurationError(
                    `Required field '${fieldPath}' is missing`,
                    fieldPath,
                    fieldValue
                );
            }
            
            // Skip validation if field is undefined and not required
            if (fieldValue === undefined) continue;
            
            // Type validation
            this._validateType(fieldValue, fieldSchema, fieldPath);
            
            // Enum validation
            if (fieldSchema.enum && !fieldSchema.enum.includes(fieldValue)) {
                throw new WasmConfigurationError(
                    `Field '${fieldPath}' must be one of: ${fieldSchema.enum.join(', ')}`,
                    fieldPath,
                    fieldValue
                );
            }
            
            // Range validation
            if (fieldSchema.min !== undefined && fieldValue < fieldSchema.min) {
                throw new WasmConfigurationError(
                    `Field '${fieldPath}' must be at least ${fieldSchema.min}`,
                    fieldPath,
                    fieldValue
                );
            }
            
            if (fieldSchema.max !== undefined && fieldValue > fieldSchema.max) {
                throw new WasmConfigurationError(
                    `Field '${fieldPath}' must be at most ${fieldSchema.max}`,
                    fieldPath,
                    fieldValue
                );
            }
            
            // Nested object validation
            if (fieldSchema.type === 'object' && fieldSchema.properties) {
                this._validateObject(fieldValue, fieldSchema.properties, fieldPath);
            }
            
            // Array validation
            if (fieldSchema.type === 'array' && fieldSchema.items) {
                if (Array.isArray(fieldValue)) {
                    fieldValue.forEach((item, index) => {
                        this._validateType(item, fieldSchema.items, `${fieldPath}[${index}]`);
                    });
                }
            }
        }
    }

    /**
     * Validate field type
     * @private
     * @param {*} value - Value to validate
     * @param {Object} schema - Field schema
     * @param {string} path - Field path for error reporting
     */
    _validateType(value, schema, path) {
        const expectedType = schema.type;
        const actualType = Array.isArray(value) ? 'array' : typeof value;
        
        if (actualType !== expectedType) {
            throw new WasmConfigurationError(
                `Field '${path}' must be of type '${expectedType}', got '${actualType}'`,
                path,
                value
            );
        }
        
        // URL validation for string types
        if (expectedType === 'string' && path.includes('url')) {
            this._validateUrl(value, path);
        }
    }

    /**
     * Validate URL format
     * @private
     * @param {string} url - URL to validate
     * @param {string} path - Field path for error reporting
     */
    _validateUrl(url, path) {
        try {
            new URL(url);
            
            // Ensure HTTPS for security
            if (!url.startsWith('https://')) {
                throw new WasmConfigurationError(
                    `URL '${path}' must use HTTPS protocol`,
                    path,
                    url
                );
            }
        } catch (error) {
            throw new WasmConfigurationError(
                `Field '${path}' must be a valid HTTPS URL`,
                path,
                url
            );
        }
    }

    /**
     * Resolve endpoint URLs based on network and user configuration
     * @private
     */
    _resolveEndpoints() {
        let urls = [];
        
        // Use user-provided URL(s) if available
        if (this.config.transport.url) {
            urls.push(this.config.transport.url);
        }
        
        if (this.config.transport.urls && this.config.transport.urls.length > 0) {
            urls = urls.concat(this.config.transport.urls);
        }
        
        // Fall back to default endpoints if none provided
        if (urls.length === 0) {
            urls = DEFAULT_ENDPOINTS[this.config.network] || DEFAULT_ENDPOINTS.testnet;
        }
        
        // Remove duplicates while preserving order
        const uniqueUrls = [...new Set(urls)];
        
        // Validate all URLs
        uniqueUrls.forEach((url, index) => {
            this._validateUrl(url, `transport.urls[${index}]`);
        });
        
        this.config.transport.urls = uniqueUrls;
        
        // Remove singular url property as we now use urls array
        delete this.config.transport.url;
    }

    /**
     * Create a default configuration
     * @static
     * @returns {Object} Default configuration
     */
    static createDefault() {
        return JSON.parse(JSON.stringify(DEFAULT_CONFIG));
    }

    /**
     * Validate a configuration object without creating an instance
     * @static
     * @param {Object} config - Configuration to validate
     * @throws {WasmConfigurationError} If configuration is invalid
     */
    static validate(config) {
        const manager = new ConfigManager(config);
        return manager.getConfig();
    }
}

/**
 * Configuration utilities
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