/**
 * Custom error classes and error handling for WASM SDK
 * Provides structured error handling with context preservation
 */

/**
 * Base class for all WASM SDK related errors
 * @extends Error
 */
export class WasmSDKError extends Error {
    /**
     * Create a WASM SDK error
     * @param {string} message - Error message
     * @param {string} code - Error code for programmatic handling
     * @param {Object} context - Additional error context
     */
    constructor(message, code = 'WASM_SDK_ERROR', context = {}) {
        super(message);
        this.name = 'WasmSDKError';
        this.code = code;
        this.context = context;
        this.timestamp = new Date().toISOString();
        
        // Maintain proper stack trace for where our error was thrown
        if (Error.captureStackTrace) {
            Error.captureStackTrace(this, WasmSDKError);
        }
    }

    /**
     * Convert error to JSON for logging/debugging
     * @returns {Object} JSON representation of error
     */
    toJSON() {
        return {
            name: this.name,
            message: this.message,
            code: this.code,
            context: this.context,
            timestamp: this.timestamp,
            stack: this.stack
        };
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
 * Utility functions for error handling
 */
export const ErrorUtils = {
    /**
     * Create a promise that wraps WASM operations with proper error handling
     * @param {Function} wasmOperation - WASM operation to wrap
     * @param {string} operationName - Name of operation for error context
     * @param {Object} context - Additional context
     * @returns {Promise} Promise with error handling
     */
    wrapWasmOperation: async (wasmOperation, operationName, context = {}) => {
        try {
            return await wasmOperation();
        } catch (error) {
            throw ErrorMapper.mapWasmError(error, operationName, context);
        }
    },

    /**
     * Validate required parameters
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
     * Create error handler for async operations
     * @param {string} operation - Operation name
     * @param {Object} context - Operation context
     * @returns {Function} Error handler function
     */
    createAsyncErrorHandler: (operation, context = {}) => {
        return (error) => {
            throw ErrorMapper.mapWasmError(error, operation, context);
        };
    }
};