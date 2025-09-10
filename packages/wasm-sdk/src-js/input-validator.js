/**
 * Comprehensive input validation for WASM SDK security
 * Prevents injection attacks and validates data formats
 */

import { WasmConfigurationError } from './error-handler.js';

/**
 * Input validation patterns and rules
 */
const VALIDATION_PATTERNS = {
    // Dash-specific patterns
    identityId: /^[A-HJ-NP-Z1-9a-km-z]{43,44}$/,           // Base58 44-char
    contractId: /^[A-HJ-NP-Z1-9a-km-z]{43,44}$/,           // Base58 44-char
    documentId: /^[A-HJ-NP-Z1-9a-km-z]{43,44}$/,           // Base58 44-char
    publicKeyHash: /^[A-HJ-NP-Z1-9a-km-z]{43,44}$/,        // Base58 44-char
    address: /^[yX][A-HJ-NP-Z1-9a-km-z]{33}$/,             // Dash address format
    
    // Cryptographic patterns
    hexPrivateKey: /^[a-fA-F0-9]{64}$/,                     // 64-char hex
    publicKeyHex: /^[a-fA-F0-9]{66}$/,                      // 66-char compressed pubkey
    wifPrivateKey: /^[15KLC][A-HJ-NP-Z1-9a-km-z]{50,51}$/, // WIF format
    
    // Network values
    network: /^(testnet|mainnet)$/,
    
    // Safe string patterns (alphanumeric with limited special chars)
    safeName: /^[a-zA-Z0-9][a-zA-Z0-9_\-\.]{0,62}$/,       // DNS-like names
    safeLabel: /^[a-z0-9][a-z0-9\-]{0,61}[a-z0-9]$/,       // Domain labels
    
    // URL patterns
    httpsUrl: /^https:\/\/[a-zA-Z0-9][a-zA-Z0-9\-\.]*[a-zA-Z0-9](:[0-9]{1,5})?(\/.*)?$/
};

/**
 * Input size limits for security
 */
const SIZE_LIMITS = {
    identityId: { min: 43, max: 44 },
    contractId: { min: 43, max: 44 },
    documentId: { min: 43, max: 44 },
    mnemonicWords: { min: 12, max: 24 },
    jsonString: { max: 1048576 },        // 1MB max JSON
    whereClause: { max: 10000 },         // 10KB max where clause
    orderByClause: { max: 1000 },        // 1KB max order by
    documentType: { min: 1, max: 64 },
    fieldName: { min: 1, max: 128 },
    label: { min: 1, max: 63 },
    url: { max: 2048 },
    arrayLength: { max: 1000 }           // Max array size
};

/**
 * Input validation utilities with security focus
 */
export class InputValidator {
    /**
     * Validate Dash identity ID
     * @param {string} identityId - Identity ID to validate
     * @param {string} fieldName - Field name for error reporting
     * @throws {WasmConfigurationError} If identity ID is invalid
     */
    static validateIdentityId(identityId, fieldName = 'identityId') {
        InputValidator._validateRequired(identityId, fieldName);
        InputValidator._validateString(identityId, fieldName);
        InputValidator._validatePattern(identityId, VALIDATION_PATTERNS.identityId, fieldName, 'Base58 format (44 characters)');
        InputValidator._validateLength(identityId, SIZE_LIMITS.identityId, fieldName);
    }

    /**
     * Validate Dash contract ID
     * @param {string} contractId - Contract ID to validate
     * @param {string} fieldName - Field name for error reporting
     */
    static validateContractId(contractId, fieldName = 'contractId') {
        InputValidator._validateRequired(contractId, fieldName);
        InputValidator._validateString(contractId, fieldName);
        InputValidator._validatePattern(contractId, VALIDATION_PATTERNS.contractId, fieldName, 'Base58 format (44 characters)');
        InputValidator._validateLength(contractId, SIZE_LIMITS.contractId, fieldName);
    }

    /**
     * Validate document ID
     * @param {string} documentId - Document ID to validate
     * @param {string} fieldName - Field name for error reporting
     */
    static validateDocumentId(documentId, fieldName = 'documentId') {
        InputValidator._validateRequired(documentId, fieldName);
        InputValidator._validateString(documentId, fieldName);
        InputValidator._validatePattern(documentId, VALIDATION_PATTERNS.documentId, fieldName, 'Base58 format (44 characters)');
        InputValidator._validateLength(documentId, SIZE_LIMITS.documentId, fieldName);
    }

    /**
     * Validate document type name
     * @param {string} documentType - Document type to validate
     * @param {string} fieldName - Field name for error reporting
     */
    static validateDocumentType(documentType, fieldName = 'documentType') {
        InputValidator._validateRequired(documentType, fieldName);
        InputValidator._validateString(documentType, fieldName);
        InputValidator._validateLength(documentType, SIZE_LIMITS.documentType, fieldName);
        InputValidator._validatePattern(documentType, VALIDATION_PATTERNS.safeName, fieldName, 'alphanumeric with limited special characters');
    }

    /**
     * Validate network name
     * @param {string} network - Network to validate
     * @param {string} fieldName - Field name for error reporting
     */
    static validateNetwork(network, fieldName = 'network') {
        InputValidator._validateRequired(network, fieldName);
        InputValidator._validateString(network, fieldName);
        InputValidator._validatePattern(network, VALIDATION_PATTERNS.network, fieldName, '"testnet" or "mainnet"');
    }

    /**
     * Validate DPNS label
     * @param {string} label - DPNS label to validate
     * @param {string} fieldName - Field name for error reporting
     */
    static validateDpnsLabel(label, fieldName = 'label') {
        InputValidator._validateRequired(label, fieldName);
        InputValidator._validateString(label, fieldName);
        InputValidator._validateLength(label, SIZE_LIMITS.label, fieldName);
        InputValidator._validatePattern(label, VALIDATION_PATTERNS.safeLabel, fieldName, 'lowercase alphanumeric with hyphens');
    }

    /**
     * Validate and sanitize JSON string input
     * @param {string} jsonString - JSON string to validate
     * @param {string} fieldName - Field name for error reporting
     * @param {Object} options - Validation options
     * @returns {Object} Parsed and validated JSON object
     */
    static validateJsonString(jsonString, fieldName = 'jsonString', options = {}) {
        const { maxSize = SIZE_LIMITS.jsonString.max, allowEmpty = false } = options;
        
        if (!jsonString && allowEmpty) {
            return null;
        }
        
        InputValidator._validateRequired(jsonString, fieldName);
        InputValidator._validateString(jsonString, fieldName);
        
        if (jsonString.length > maxSize) {
            throw new WasmConfigurationError(
                `${fieldName} is too large (${jsonString.length} bytes, max ${maxSize} bytes)`,
                fieldName,
                `${jsonString.length} bytes`
            );
        }
        
        let parsed;
        try {
            parsed = JSON.parse(jsonString);
        } catch (error) {
            throw new WasmConfigurationError(
                `${fieldName} is not valid JSON: ${error.message}`,
                fieldName,
                'invalid JSON'
            );
        }
        
        // Additional JSON security checks
        InputValidator._validateJsonSecurity(parsed, fieldName);
        
        return parsed;
    }

    /**
     * Validate where clause for document queries
     * @param {string|Array} whereClause - Where clause to validate
     * @param {string} fieldName - Field name for error reporting
     * @returns {Array} Validated where clause array
     */
    static validateWhereClause(whereClause, fieldName = 'whereClause') {
        if (!whereClause) {
            return [];
        }
        
        let parsedWhere;
        
        if (typeof whereClause === 'string') {
            if (whereClause.length > SIZE_LIMITS.whereClause.max) {
                throw new WasmConfigurationError(
                    `${fieldName} string is too large (max ${SIZE_LIMITS.whereClause.max} characters)`,
                    fieldName,
                    `${whereClause.length} chars`
                );
            }
            parsedWhere = InputValidator.validateJsonString(whereClause, fieldName);
        } else if (Array.isArray(whereClause)) {
            parsedWhere = whereClause;
        } else {
            throw new WasmConfigurationError(
                `${fieldName} must be an array or JSON string`,
                fieldName,
                typeof whereClause
            );
        }
        
        if (!Array.isArray(parsedWhere)) {
            throw new WasmConfigurationError(
                `${fieldName} must be an array`,
                fieldName,
                typeof parsedWhere
            );
        }
        
        // Validate where clause structure
        InputValidator._validateWhereClauseStructure(parsedWhere, fieldName);
        
        return parsedWhere;
    }

    /**
     * Validate order by clause
     * @param {string|Array} orderByClause - Order by clause to validate
     * @param {string} fieldName - Field name for error reporting
     * @returns {Array} Validated order by clause array
     */
    static validateOrderByClause(orderByClause, fieldName = 'orderByClause') {
        if (!orderByClause) {
            return [];
        }
        
        let parsedOrderBy;
        
        if (typeof orderByClause === 'string') {
            if (orderByClause.length > SIZE_LIMITS.orderByClause.max) {
                throw new WasmConfigurationError(
                    `${fieldName} string is too large (max ${SIZE_LIMITS.orderByClause.max} characters)`,
                    fieldName,
                    `${orderByClause.length} chars`
                );
            }
            parsedOrderBy = InputValidator.validateJsonString(orderByClause, fieldName);
        } else if (Array.isArray(orderByClause)) {
            parsedOrderBy = orderByClause;
        } else {
            throw new WasmConfigurationError(
                `${fieldName} must be an array or JSON string`,
                fieldName,
                typeof orderByClause
            );
        }
        
        if (!Array.isArray(parsedOrderBy)) {
            throw new WasmConfigurationError(
                `${fieldName} must be an array`,
                fieldName,
                typeof parsedOrderBy
            );
        }
        
        // Validate order by structure
        InputValidator._validateOrderByStructure(parsedOrderBy, fieldName);
        
        return parsedOrderBy;
    }

    /**
     * Validate array input with size limits
     * @param {Array} array - Array to validate
     * @param {string} fieldName - Field name for error reporting
     * @param {Object} options - Validation options
     */
    static validateArray(array, fieldName, options = {}) {
        const { maxLength = SIZE_LIMITS.arrayLength.max, minLength = 0, required = true } = options;
        
        if (!array && required) {
            throw new WasmConfigurationError(
                `${fieldName} is required`,
                fieldName,
                array
            );
        }
        
        if (!array) {
            return;
        }
        
        if (!Array.isArray(array)) {
            throw new WasmConfigurationError(
                `${fieldName} must be an array`,
                fieldName,
                typeof array
            );
        }
        
        if (array.length < minLength) {
            throw new WasmConfigurationError(
                `${fieldName} must have at least ${minLength} items`,
                fieldName,
                array.length
            );
        }
        
        if (array.length > maxLength) {
            throw new WasmConfigurationError(
                `${fieldName} exceeds maximum length of ${maxLength} items`,
                fieldName,
                array.length
            );
        }
    }

    /**
     * Validate URL with security checks
     * @param {string} url - URL to validate
     * @param {string} fieldName - Field name for error reporting
     */
    static validateUrl(url, fieldName = 'url') {
        InputValidator._validateRequired(url, fieldName);
        InputValidator._validateString(url, fieldName);
        InputValidator._validateLength(url, SIZE_LIMITS.url, fieldName);
        
        // Security: Only allow HTTPS URLs
        if (!url.startsWith('https://')) {
            throw new WasmConfigurationError(
                `${fieldName} must use HTTPS protocol for security`,
                fieldName,
                url
            );
        }
        
        InputValidator._validatePattern(url, VALIDATION_PATTERNS.httpsUrl, fieldName, 'valid HTTPS URL');
        
        // Additional URL security checks with proper error handling
        let parsed;
        try {
            parsed = new URL(url);
        } catch (error) {
            throw new WasmConfigurationError(
                `${fieldName} is not a valid URL: ${error.message}`,
                fieldName,
                url
            );
        }
        
        // Block localhost and internal IPs in production
        if (parsed.hostname === 'localhost' || 
            parsed.hostname.startsWith('127.') || 
            parsed.hostname.startsWith('192.168.') ||
            parsed.hostname.startsWith('10.') ||
            parsed.hostname.match(/^172\.(1[6-9]|2[0-9]|3[01])\./)) {
            console.warn(`Warning: ${fieldName} uses internal/localhost address - only suitable for development`);
        }
        
        // Validate port range
        if (parsed.port && (parseInt(parsed.port) < 1 || parseInt(parsed.port) > 65535)) {
            throw new WasmConfigurationError(
                `${fieldName} port must be between 1 and 65535`,
                fieldName,
                parsed.port
            );
        }
    }

    // ========== Private Validation Helpers ==========

    /**
     * Validate required field
     * @private
     */
    static _validateRequired(value, fieldName) {
        if (value === undefined || value === null) {
            throw new WasmConfigurationError(
                `${fieldName} is required`,
                fieldName,
                value
            );
        }
    }

    /**
     * Validate string type
     * @private
     */
    static _validateString(value, fieldName) {
        if (typeof value !== 'string') {
            throw new WasmConfigurationError(
                `${fieldName} must be a string`,
                fieldName,
                typeof value
            );
        }
    }

    /**
     * Validate pattern match
     * @private
     */
    static _validatePattern(value, pattern, fieldName, description) {
        if (!pattern.test(value)) {
            throw new WasmConfigurationError(
                `${fieldName} must be ${description}`,
                fieldName,
                value
            );
        }
    }

    /**
     * Validate string length
     * @private
     */
    static _validateLength(value, limits, fieldName) {
        if (limits.min !== undefined && value.length < limits.min) {
            throw new WasmConfigurationError(
                `${fieldName} must be at least ${limits.min} characters`,
                fieldName,
                value.length
            );
        }
        
        if (limits.max !== undefined && value.length > limits.max) {
            throw new WasmConfigurationError(
                `${fieldName} must be at most ${limits.max} characters`,
                fieldName,
                value.length
            );
        }
    }

    /**
     * Validate JSON security (prevent prototype pollution, etc.)
     * @private
     */
    static _validateJsonSecurity(obj, fieldName) {
        if (obj && typeof obj === 'object') {
            // Check for prototype pollution attempts - only check own properties
            const dangerousKeys = ['__proto__', 'constructor', 'prototype'];
            
            for (const key of dangerousKeys) {
                if (Object.prototype.hasOwnProperty.call(obj, key)) {
                    throw new WasmConfigurationError(
                        `${fieldName} contains dangerous key: ${key}`,
                        fieldName,
                        'prototype pollution attempt'
                    );
                }
            }
            
            // Recursively check nested objects
            for (const [key, value] of Object.entries(obj)) {
                if (value && typeof value === 'object' && !Array.isArray(value)) {
                    InputValidator._validateJsonSecurity(value, `${fieldName}.${key}`);
                }
            }
        }
    }

    /**
     * Validate where clause structure
     * @private
     */
    static _validateWhereClauseStructure(whereClause, fieldName) {
        if (!Array.isArray(whereClause)) {
            return;
        }
        
        for (const [index, clause] of whereClause.entries()) {
            if (!Array.isArray(clause) || clause.length < 3) {
                throw new WasmConfigurationError(
                    `${fieldName}[${index}] must be an array with at least 3 elements [field, operator, value]`,
                    fieldName,
                    clause
                );
            }
            
            const [field, operator, value] = clause;
            
            // Validate field name
            if (typeof field !== 'string' || field.length === 0) {
                throw new WasmConfigurationError(
                    `${fieldName}[${index}] field must be a non-empty string`,
                    fieldName,
                    field
                );
            }
            
            // Validate operator
            const validOperators = ['=', '==', '!=', '>', '>=', '<', '<=', 'in', 'In', 'startsWith', 'StartsWith'];
            if (typeof operator !== 'string' || !validOperators.includes(operator)) {
                throw new WasmConfigurationError(
                    `${fieldName}[${index}] operator must be one of: ${validOperators.join(', ')}`,
                    fieldName,
                    operator
                );
            }
        }
    }

    /**
     * Validate order by clause structure
     * @private
     */
    static _validateOrderByStructure(orderByClause, fieldName) {
        if (!Array.isArray(orderByClause)) {
            return;
        }
        
        for (const [index, clause] of orderByClause.entries()) {
            if (!Array.isArray(clause) || clause.length !== 2) {
                throw new WasmConfigurationError(
                    `${fieldName}[${index}] must be an array with exactly 2 elements [field, direction]`,
                    fieldName,
                    clause
                );
            }
            
            const [field, direction] = clause;
            
            // Validate field name
            if (typeof field !== 'string' || field.length === 0) {
                throw new WasmConfigurationError(
                    `${fieldName}[${index}] field must be a non-empty string`,
                    fieldName,
                    field
                );
            }
            
            // Validate direction
            if (typeof direction !== 'string' || !['asc', 'desc'].includes(direction.toLowerCase())) {
                throw new WasmConfigurationError(
                    `${fieldName}[${index}] direction must be "asc" or "desc"`,
                    fieldName,
                    direction
                );
            }
        }
    }
}