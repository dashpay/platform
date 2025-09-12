/**
 * Credit Consumption Helper - TEST ENVIRONMENT ONLY
 * 
 * Provides credit consumption tracking for test validation as specified in PRD.
 * This functionality is explicitly NOT included in production SDK responses.
 * 
 * PRD Reference: Section 5.3 - "Credit consumption tracking is TESTING ONLY"
 */

/**
 * Execute a platform operation with credit consumption tracking
 * @param {Object} sdk - The WasmSDK instance
 * @param {string} identityId - Identity ID to track credits for
 * @param {Function} operation - The operation to execute
 * @param {string} operationType - Type of operation for result formatting
 * @returns {Promise<Object>} Test response with credit consumption data
 */
export async function executeWithCreditTracking(sdk, identityId, operation, operationType) {
    // Get balance before operation
    const beforeBalance = await sdk.getIdentityBalance(identityId);
    const startTime = Date.now();
    
    // Execute the operation
    const result = await operation();
    
    // Get balance after operation
    const afterBalance = await sdk.getIdentityBalance(identityId);
    const confirmationTime = Date.now() - startTime;
    
    // Calculate credit consumption
    const creditsConsumed = Math.max(0, beforeBalance.balance - afterBalance.balance);
    
    // Return test response format with credit tracking (PRD Section 5.3)
    return {
        // Core result data (same as production):
        documentId: result.documentId,
        contractId: result.contractId,
        transactionId: result.transactionId || 'pending',
        
        // Credit consumption tracking (TEST ENVIRONMENT ONLY):
        creditsConsumed: creditsConsumed,
        creditsBefore: beforeBalance.balance,
        creditsAfter: afterBalance.balance,
        
        // Platform metadata (same as production):
        blockHeight: result.blockHeight || 0,
        timestamp: Date.now(),
        revision: result.revision || 1,
        network: 'testnet',
        confirmationTime: confirmationTime,
        
        // Test validation flags
        testMetadata: {
            operationType: operationType,
            creditConsumptionValidated: creditsConsumed > 0,
            executionTime: confirmationTime,
            balanceChangeDetected: creditsConsumed !== 0
        }
    };
}

/**
 * Test document creation with credit consumption validation
 * @param {Object} sdk - The WasmSDK instance  
 * @param {string} mnemonic - Mnemonic for authentication
 * @param {string} identityId - Identity ID
 * @param {string} contractId - Contract ID
 * @param {string} documentType - Document type
 * @param {string} documentData - Document data JSON
 * @param {number} keyIndex - Key index for signing
 * @returns {Promise<Object>} Test result with credit validation
 */
export async function testDocumentCreation(sdk, mnemonic, identityId, contractId, documentType, documentData, keyIndex) {
    return executeWithCreditTracking(
        sdk,
        identityId,
        () => sdk.createDocument(mnemonic, identityId, contractId, documentType, documentData, keyIndex),
        'DocumentCreated'
    );
}

/**
 * Test contract creation with credit consumption validation
 * @param {Object} sdk - The WasmSDK instance
 * @param {string} mnemonic - Mnemonic for authentication
 * @param {string} identityId - Identity ID
 * @param {string} contractDefinition - Contract definition JSON
 * @param {number} keyIndex - Key index for signing
 * @returns {Promise<Object>} Test result with credit validation
 */
export async function testContractCreation(sdk, mnemonic, identityId, contractDefinition, keyIndex) {
    return executeWithCreditTracking(
        sdk,
        identityId,
        () => sdk.createDataContract(mnemonic, identityId, contractDefinition, keyIndex),
        'ContractCreated'
    );
}

/**
 * Validate credit consumption meets PRD requirements
 * @param {Object} testResult - Result from executeWithCreditTracking
 * @param {number} expectedMinCredits - Minimum expected credit consumption
 * @param {number} expectedMaxCredits - Maximum expected credit consumption
 * @returns {Object} Validation result
 */
export function validateCreditConsumption(testResult, expectedMinCredits = 1, expectedMaxCredits = 50000000) {
    const validations = {
        hasCreditsConsumed: testResult.creditsConsumed > 0,
        creditsInRange: testResult.creditsConsumed >= expectedMinCredits && testResult.creditsConsumed <= expectedMaxCredits,
        balanceDecreased: testResult.creditsBefore > testResult.creditsAfter,
        operationCompleted: !!testResult.documentId || !!testResult.contractId,
        executionTime: testResult.confirmationTime
    };
    
    const allValid = validations.hasCreditsConsumed && 
                    validations.creditsInRange && 
                    validations.balanceDecreased && 
                    validations.operationCompleted;
    
    return {
        valid: allValid,
        validations: validations,
        creditsConsumed: testResult.creditsConsumed,
        summary: allValid ? 'PRD credit consumption requirements met' : 'Credit consumption validation failed'
    };
}

/**
 * Test helper to run PRD dual verification pattern
 * Both credit consumption AND existence verification
 * @param {Object} sdk - The WasmSDK instance
 * @param {Object} testResult - Result from credit tracking operation
 * @param {string} contractId - Contract ID to verify against
 * @param {string} documentType - Document type
 * @returns {Promise<Object>} Dual verification result
 */
export async function dualVerificationTest(sdk, testResult, contractId, documentType) {
    // Verification 1: Credit consumption (already done)
    const creditValidation = validateCreditConsumption(testResult);
    
    // Verification 2: Item existence verification
    let existenceValidation = {
        itemExists: false,
        itemAccessible: false,
        dataMatches: false
    };
    
    try {
        if (testResult.documentId) {
            const createdDocument = await sdk.getDocument(contractId, documentType, testResult.documentId);
            existenceValidation = {
                itemExists: !!createdDocument,
                itemAccessible: true,
                dataMatches: createdDocument?.ownerId === testResult.testMetadata?.identityId
            };
        }
    } catch (error) {
        existenceValidation.error = error.message;
    }
    
    return {
        creditValidation: creditValidation,
        existenceValidation: existenceValidation,
        bothVerified: creditValidation.valid && existenceValidation.itemExists,
        prdCompliant: creditValidation.valid && existenceValidation.itemExists
    };
}