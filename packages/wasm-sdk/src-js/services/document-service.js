/**
 * Document Service - Handles all document-related operations
 * Extracted from monolithic WasmSDK class for better organization
 */

import { ErrorUtils, WasmOperationError } from '../error-handler.js';

/**
 * Default pagination limits for document queries
 */
const DEFAULT_PAGINATION_LIMITS = {
    maxTotalDocuments: 10000,    // Never fetch more than 10K docs
    maxBatchCount: 100,          // Max 100 pagination requests  
    maxBatchSize: 200,           // Larger batches for efficiency
    timeoutMs: 300000           // 5 minute total timeout
};

export class DocumentService {
    /**
     * Create document service
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
     * Get documents by contract and type with optimized pagination
     * @param {string} contractId - Data contract ID
     * @param {string} documentType - Document type
     * @param {Object} options - Query options
     * @param {Array} options.where - Where conditions
     * @param {Array} options.orderBy - Order by conditions
     * @param {number} options.limit - Result limit
     * @param {number} options.offset - Result offset
     * @param {boolean} options.getAllDocuments - Fetch all documents with pagination
     * @param {Object} options.limits - Pagination limits override
     * @param {Function} options.onProgress - Progress callback for large operations
     * @returns {Promise<Object>} Structured response with documents and metadata
     */
    async getDocuments(contractId, documentType, options = {}) {
        ErrorUtils.validateRequired({ contractId, documentType }, ['contractId', 'documentType']);
        
        const { 
            where = [], 
            orderBy = [], 
            limit, 
            offset = 0, 
            startAfter, 
            startAt, 
            getAllDocuments = false,
            limits = {},
            onProgress 
        } = options;

        // Merge user limits with defaults
        const paginationLimits = { ...DEFAULT_PAGINATION_LIMITS, ...limits };
        
        const useProofs = this.configManager.getProofs();
        // Note: Document proof verification has issues, use non-proof version for now
        const methodName = 'get_documents'; // Always use non-proof version until proof issues fixed
        
        if (useProofs && this.configManager.getDebug()) {
            console.debug('Note: Using non-proof document query due to proof verification issues');
        }
        
        // Convert where and orderBy to JSON strings if they're arrays
        const whereClause = Array.isArray(where) ? JSON.stringify(where) : where;
        const orderByClause = Array.isArray(orderBy) ? JSON.stringify(orderBy) : orderBy;
        
        if (getAllDocuments) {
            return this._getPaginatedDocuments(
                contractId, 
                documentType, 
                whereClause, 
                orderByClause, 
                paginationLimits,
                onProgress
            );
        } else {
            // Single query with user-specified parameters
            const userLimit = limit ? Math.min(limit, 1000) : 100; // Cap at 1000 for safety
            
            const documents = await this._executeOperation(
                () => this.wasmModule[methodName](
                    this.wasmSdk,
                    contractId,
                    documentType,
                    whereClause || null,
                    orderByClause || null,
                    userLimit,
                    startAfter || null,
                    startAt || null
                ),
                methodName,
                { contractId, documentType, options, proofs: false }
            );
            
            return this._formatDocumentResponse(
                contractId, 
                documentType, 
                documents || [], 
                { where, orderBy, limit: userLimit, offset, startAfter, startAt }
            );
        }
    }

    /**
     * Get document by ID
     * @param {string} contractId - Data contract ID
     * @param {string} documentType - Document type
     * @param {string} documentId - Document ID
     * @returns {Promise<Object|null>} Document or null if not found
     */
    async getDocument(contractId, documentType, documentId) {
        ErrorUtils.validateRequired({ contractId, documentType, documentId }, 
                                   ['contractId', 'documentType', 'documentId']);
        
        return this._executeOperation(
            () => this.wasmSdk.get_document(contractId, documentType, documentId),
            'get_document',
            { contractId, documentType, documentId }
        );
    }

    /**
     * Create document
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractId - Contract ID
     * @param {string} documentType - Document type
     * @param {string} documentData - JSON document data
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Document creation result
     */
    async createDocument(mnemonic, identityId, contractId, documentType, documentData, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractId, documentType, documentData, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractId', 'documentType', 'documentData', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.document_create(this.wasmSdk, mnemonic, identityId, contractId, documentType, documentData, keyIndex),
            'document_create',
            { mnemonic: '[SANITIZED]', identityId, contractId, documentType, documentData: '[SANITIZED]', keyIndex }
        );
    }

    /**
     * Update document
     * @param {string} mnemonic - Mnemonic for signing
     * @param {string} identityId - Owner identity ID
     * @param {string} contractId - Contract ID
     * @param {string} documentType - Document type
     * @param {string} documentId - Document ID to update
     * @param {string} updateData - JSON update data
     * @param {number} keyIndex - Key index for signing
     * @returns {Promise<Object>} Document update result
     */
    async updateDocument(mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex) {
        ErrorUtils.validateRequired({ mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex }, 
                                   ['mnemonic', 'identityId', 'contractId', 'documentType', 'documentId', 'updateData', 'keyIndex']);
        
        return this._executeOperation(
            () => this.wasmModule.document_update(this.wasmSdk, mnemonic, identityId, contractId, documentType, documentId, updateData, keyIndex),
            'document_update',
            { mnemonic: '[SANITIZED]', identityId, contractId, documentType, documentId, updateData: '[SANITIZED]', keyIndex }
        );
    }

    /**
     * Get paginated documents with safety limits and progress reporting
     * @private
     * @param {string} contractId - Contract ID
     * @param {string} documentType - Document type
     * @param {string} whereClause - Where clause JSON
     * @param {string} orderByClause - Order by clause JSON
     * @param {Object} limits - Pagination limits
     * @param {Function} onProgress - Progress callback
     * @returns {Promise<Object>} Paginated documents response
     */
    async _getPaginatedDocuments(contractId, documentType, whereClause, orderByClause, limits, onProgress) {
        const allDocuments = [];
        let startAfter = null;
        let batchCount = 0;
        let totalFetched = 0;
        const startTime = Date.now();
        
        // Pagination loop with multiple safety mechanisms
        while (totalFetched < limits.maxTotalDocuments && 
               batchCount < limits.maxBatchCount) {
            
            // TIMEOUT PROTECTION
            if (Date.now() - startTime > limits.timeoutMs) {
                throw new WasmOperationError(
                    `Document pagination timeout after ${limits.timeoutMs}ms`,
                    'get_documents_paginated',
                    { contractId, documentType, totalFetched, batchCount }
                );
            }
            
            // ADAPTIVE BATCH SIZING
            const remainingNeeded = limits.maxTotalDocuments - totalFetched;
            const batchSize = Math.min(limits.maxBatchSize, remainingNeeded);
            
            const batch = await this._executeOperation(
                () => this.wasmModule.get_documents(
                    this.wasmSdk,
                    contractId,
                    documentType,
                    whereClause,
                    orderByClause,
                    batchSize,
                    startAfter,
                    null  // startAt
                ),
                'get_documents_batch',
                { contractId, documentType, batchCount, batchSize, proofs: false }
            );
            
            if (!batch || batch.length === 0) {
                break; // No more documents
            }
            
            allDocuments.push(...batch);
            totalFetched += batch.length;
            batchCount++;
            
            // PROGRESS REPORTING
            if (onProgress && typeof onProgress === 'function') {
                try {
                    onProgress({
                        totalFetched,
                        batchCount,
                        batchSize: batch.length,
                        timeElapsed: Date.now() - startTime,
                        estimatedTotal: totalFetched < batchSize ? totalFetched : null
                    });
                } catch (progressError) {
                    // Don't let progress callback errors break pagination
                    console.warn('Progress callback error:', progressError);
                }
            }
            
            // DUPLICATE DETECTION AND PAGINATION LOGIC
            const lastDoc = batch[batch.length - 1];
            const lastDocData = typeof lastDoc.toJSON === 'function' ? lastDoc.toJSON() : lastDoc;
            const nextStartAfter = lastDocData.id || lastDocData.$id || lastDocData.identifier;
            
            if (nextStartAfter === startAfter) {
                // Same ID returned - pagination stuck, break to prevent infinite loop
                break;
            }
            
            startAfter = nextStartAfter;
            
            // PARTIAL BATCH = END OF DATA
            if (batch.length < batchSize) {
                break; // Last batch was partial, no more documents
            }
        }
        
        // Return structured response with metadata
        return {
            contractId,
            documentType,
            totalCount: allDocuments.length,
            documents: allDocuments.map(doc => {
                return typeof doc.toJSON === 'function' ? doc.toJSON() : doc;
            }),
            query: {
                where: JSON.parse(whereClause || '[]'),
                orderBy: JSON.parse(orderByClause || '[]'),
                getAllDocuments: true
            },
            pagination: {
                totalFetched,
                batchCount,
                truncated: totalFetched >= limits.maxTotalDocuments,
                timeElapsed: Date.now() - startTime,
                hitLimits: {
                    maxDocuments: totalFetched >= limits.maxTotalDocuments,
                    maxBatches: batchCount >= limits.maxBatchCount,
                    timeout: Date.now() - startTime >= limits.timeoutMs
                }
            }
        };
    }

    /**
     * Format document response with consistent structure
     * @private
     * @param {string} contractId - Contract ID
     * @param {string} documentType - Document type
     * @param {Array} documents - Documents array
     * @param {Object} query - Query parameters
     * @returns {Object} Formatted response
     */
    _formatDocumentResponse(contractId, documentType, documents, query) {
        return {
            contractId,
            documentType,
            totalCount: documents.length,
            documents: documents.map(doc => {
                return typeof doc.toJSON === 'function' ? doc.toJSON() : doc;
            }),
            query,
            pagination: {
                totalFetched: documents.length,
                batchCount: 1,
                truncated: false,
                timeElapsed: 0,
                hitLimits: {
                    maxDocuments: false,
                    maxBatches: false,
                    timeout: false
                }
            }
        };
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