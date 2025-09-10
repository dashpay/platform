/**
 * Custom error classes and error handling for WASM SDK
 * Provides structured error handling with comprehensive data sanitization
 */

/**
 * Sensitive field names that should never be logged or exposed
 */
const SENSITIVE_FIELD_NAMES = [
    'mnemonic', 'privateKey', 'assetLockPrivateKey', 'publicKeys', 
    'signature', 'seed', 'updateData', 'documentData', 'contractDefinition',
    'updateDefinition', 'passphrase', 'privateKeyWif', 'wif', 'key',
    'secret', 'token', 'password', 'credentials'
];

/**
 * Data sanitizer for removing sensitive information from contexts
 */
export class DataSanitizer {
    /**
     * Sanitize context object by removing or redacting sensitive data
     * @param {Object} context - Context to sanitize
     * @param {Object} options - Sanitization options
     * @returns {Object} Sanitized context
     */
    static sanitizeContext(context, options = {}) {
        const { preserveStructure = true, redactValue = '[SANITIZED]' } = options;
        
        if (!context || typeof context !== 'object') {
            return context;
        }
        
        const sanitized = Array.isArray(context) ? [] : {};
        
        for (const [key, value] of Object.entries(context)) {
            if (DataSanitizer._isSensitiveField(key)) {
                sanitized[key] = preserveStructure ? redactValue : undefined;
            } else if (typeof value === 'object' && value !== null) {
                sanitized[key] = DataSanitizer.sanitizeContext(value, options);
            } else if (typeof value === 'string' && DataSanitizer._looksLikeSensitiveValue(value)) {
                sanitized[key] = DataSanitizer._redactSensitiveString(value, redactValue);
            } else {
                sanitized[key] = value;
            }
        }
        
        return sanitized;
    }

    /**
     * Sanitize error message to remove embedded sensitive data
     * @param {string} message - Error message to sanitize
     * @returns {string} Sanitized error message
     */
    static sanitizeErrorMessage(message) {
        if (typeof message !== 'string') {
            return message;
        }
        
        // Redact potential private keys, mnemonics, etc. in error messages
        const patterns = [
            /([a-fA-F0-9]{64})/g,           // Hex private keys
            /([a-zA-Z0-9]{12,24}(\s+[a-zA-Z0-9]{3,8}){11,23})/g,  // Mnemonic patterns
            /([15KLC][a-zA-HJ-NP-Z1-9]{25,34})/g,  // Bitcoin-style addresses in errors
            /(cw[a-zA-Z0-9]{32,})/g         // WIF private keys
        ];
        
        let sanitizedMessage = message;
        patterns.forEach(pattern => {
            sanitizedMessage = sanitizedMessage.replace(pattern, '[REDACTED_SENSITIVE_DATA]');
        });
        
        return sanitizedMessage;
    }

    /**
     * Check if a field name indicates sensitive data
     * @private
     * @param {string} fieldName - Field name to check
     * @returns {boolean} True if field appears to contain sensitive data
     */
    static _isSensitiveField(fieldName) {
        if (typeof fieldName !== 'string') {
            return false;
        }
        
        const lowerField = fieldName.toLowerCase();
        return SENSITIVE_FIELD_NAMES.some(sensitive => 
            lowerField.includes(sensitive.toLowerCase())
        );
    }

    /**
     * Check if a string value looks like sensitive data
     * @private
     * @param {string} value - Value to check
     * @returns {boolean} True if value looks sensitive
     */
    static _looksLikeSensitiveValue(value) {
        if (typeof value !== 'string' || value.length < 10) {
            return false;
        }
        
        // Check for hex patterns that look like private keys
        if (/^[a-fA-F0-9]{64}$/.test(value)) {
            return true;
        }
        
        // Check for mnemonic-like patterns (12+ words)
        const words = value.trim().split(/\s+/);
        if (words.length >= 12 && words.every(word => /^[a-z]{3,8}$/.test(word))) {
            return true;
        }
        
        // Check for WIF private key patterns
        if (/^[15KLC][a-zA-HJ-NP-Z1-9]{25,34}$/.test(value)) {
            return true;
        }
        
        return false;
    }

    /**
     * Redact sensitive string values
     * @private
     * @param {string} value - Value to redact
     * @param {string} redactValue - Replacement value
     * @returns {string} Redacted value
     */
    static _redactSensitiveString(value, redactValue) {
        if (value.length > 20) {
            // For long values, show first few chars for debugging
            return `${value.substring(0, 4)}...${redactValue}`;
        }
        return redactValue;
    }
}

/**
 * Base class for all WASM SDK related errors with automatic data sanitization
 * @extends Error
 */
export class WasmSDKError extends Error {
    /**
     * Create a WASM SDK error with automatic context sanitization
     * @param {string} message - Error message
     * @param {string} code - Error code for programmatic handling
     * @param {Object} context - Additional error context (will be sanitized)
     */
    constructor(message, code = 'WASM_SDK_ERROR', context = {}) {
        // Sanitize the error message first
        const sanitizedMessage = DataSanitizer.sanitizeErrorMessage(message);
        super(sanitizedMessage);
        
        this.name = 'WasmSDKError';
        this.code = code;
        this.context = DataSanitizer.sanitizeContext(context);
        this.timestamp = new Date().toISOString();
        
        // Store original context for internal debugging (never logged)
        this._internalContext = context;
        
        // Maintain proper stack trace for where our error was thrown
        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, WasmSDKError);
        }
    }

    /**
     * Convert error to JSON for logging/debugging - context is pre-sanitized
     * @returns {Object} JSON representation of error with sanitized context
     */
    toJSON() {
        return {
            name: this.name,
            message: this.message, // Already sanitized in constructor
            code: this.code,
            context: this.context, // Already sanitized in constructor
            timestamp: this.timestamp,
            stack: this.stack
        };
    }

    /**
     * Get sanitized context for safe logging
     * @returns {Object} Sanitized context
     */
    getSanitizedContext() {
        return this.context;
    }

    /**
     * Get internal context for debugging (should never be logged)
     * @returns {Object} Original unsanitized context
     * @internal
     */
    _getInternalContext() {
        return this._internalContext;
    }
}

/**
 * Error thrown during WASM initialization
 * @extends WasmSDKError
 */
export class WasmInitializationError extends WasmSDKError {
    constructor(message, context = {}) {
        super(message, 'WASM_INIT_ERROR', context);
        this.name = 'WasmInitializationError';
    }
}

/**
 * Error thrown during WASM operations
 * @extends WasmSDKError
 */
export class WasmOperationError extends WasmSDKError {
    constructor(message, operation, context = {}) {
        super(message, 'WASM_OPERATION_ERROR', { ...context, operation });
        this.name = 'WasmOperationError';
        this.operation = operation;
    }
}

/**
 * Error thrown for invalid configuration
 * @extends WasmSDKError
 */
export class WasmConfigurationError extends WasmSDKError {
    constructor(message, field, value, context = {}) {
        super(message, 'WASM_CONFIG_ERROR', { ...context, field, value });
        this.name = 'WasmConfigurationError';
        this.field = field;
        this.value = value;
    }
}

/**
 * Error thrown for network/transport issues
 * @extends WasmSDKError
 */
export class WasmTransportError extends WasmSDKError {
    constructor(message, endpoint, context = {}) {
        super(message, 'WASM_TRANSPORT_ERROR', { ...context, endpoint });
        this.name = 'WasmTransportError';
        this.endpoint = endpoint;
    }
}

/**
 * Error mapper to convert WASM errors to structured JS errors
 */
export class ErrorMapper {
    /**
     * Map WASM error to appropriate JS error class
     * @param {Error} wasmError - Original WASM error
     * @param {string} operation - Operation that failed
     * @param {Object} context - Additional context
     * @returns {WasmSDKError} Mapped error
     */
    static mapWasmError(wasmError, operation = 'unknown', context = {}) {
        // Extract error message and try to categorize
        const message = wasmError.message || 'Unknown WASM error';
        const stack = wasmError.stack || '';
        
        // Map common WASM error patterns to specific error types
        if (message.includes('initialization') || message.includes('init')) {
            return new WasmInitializationError(message, {
                ...context,
                originalStack: stack
            });
        }
        
        if (message.includes('network') || message.includes('connection') || message.includes('timeout')) {
            return new WasmTransportError(message, context.endpoint, {
                ...context,
                originalStack: stack
            });
        }
        
        if (message.includes('config') || message.includes('parameter') || message.includes('invalid')) {
            return new WasmConfigurationError(message, context.field, context.value, {
                ...context,
                originalStack: stack
            });
        }
        
        // Default to operation error
        return new WasmOperationError(message, operation, {
            ...context,
            originalStack: stack
        });
    }

    /**
     * Check if error is a WASM SDK error
     * @param {Error} error - Error to check
     * @returns {boolean} True if it's a WASM SDK error
     */
    static isWasmSDKError(error) {
        return error instanceof WasmSDKError;
    }

    /**
     * Get error details for debugging
     * @param {Error} error - Error to analyze
     * @returns {Object} Error details
     */
    static getErrorDetails(error) {
        if (ErrorMapper.isWasmSDKError(error)) {
            return error.toJSON();
        }
        
        return {
            name: error.name || 'Error',
            message: error.message || 'Unknown error',
            stack: error.stack || 'No stack trace available',
            timestamp: new Date().toISOString()
        };
    }
}

/**
 * Utility functions for error handling with security enhancements
 */
export const ErrorUtils = {
    /**
     * Create a promise that wraps WASM operations with proper error handling
     * @param {Function} wasmOperation - WASM operation to wrap
     * @param {string} operationName - Name of operation for error context
     * @param {Object} context - Additional context (will be sanitized)
     * @returns {Promise} Promise with error handling
     */
    wrapWasmOperation: async (wasmOperation, operationName, context = {}) => {
        try {
            return await wasmOperation();
        } catch (error) {
            // Context is automatically sanitized by ErrorMapper
            throw ErrorMapper.mapWasmError(error, operationName, context);
        }
    },

    /**
     * Validate required parameters with enhanced security
     * @param {Object} params - Parameters to validate
     * @param {string[]} requiredFields - List of required field names
     * @throws {WasmConfigurationError} If required fields are missing
     */
    validateRequired: (params, requiredFields) => {
        for (const field of requiredFields) {
            if (params[field] === undefined || params[field] === null) {
                throw new WasmConfigurationError(
                    `Required parameter '${field}' is missing`,
                    field,
                    params[field]
                );
            }
        }
    },

    /**
     * Create error handler for async operations with context sanitization
     * @param {string} operation - Operation name
     * @param {Object} context - Operation context (will be sanitized)
     * @returns {Function} Error handler function
     */
    createAsyncErrorHandler: (operation, context = {}) => {
        return (error) => {
            throw ErrorMapper.mapWasmError(error, operation, context);
        };
    },

    /**
     * Sanitize context for safe logging (direct access to DataSanitizer)
     * @param {Object} context - Context to sanitize
     * @param {Object} options - Sanitization options
     * @returns {Object} Sanitized context
     */
    sanitizeContext: (context, options = {}) => {
        return DataSanitizer.sanitizeContext(context, options);
    },

    /**
     * Sanitize error message (direct access to DataSanitizer)
     * @param {string} message - Message to sanitize
     * @returns {string} Sanitized message
     */
    sanitizeErrorMessage: (message) => {
        return DataSanitizer.sanitizeErrorMessage(message);
    }
};