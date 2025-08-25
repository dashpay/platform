const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');
const { ParameterInjector } = require('../utils/parameter-injector');

/**
 * Helper function to execute a state transition
 * @param {WasmSdkPage} wasmSdkPage - The page object instance
 * @param {ParameterInjector} parameterInjector - The parameter injector instance
 * @param {string} category - State transition category (e.g., 'identity', 'dataContract')
 * @param {string} transitionType - Transition type (e.g., 'identityCreate')
 * @param {string} network - Network to use ('testnet' or 'mainnet')
 * @returns {Promise<Object>} - The transition result object
 */
async function executeStateTransition(wasmSdkPage, parameterInjector, category, transitionType, network = 'testnet') {
  await wasmSdkPage.setupStateTransition(category, transitionType);
  
  const success = await parameterInjector.injectStateTransitionParameters(category, transitionType, network);
  expect(success).toBe(true);
  
  const result = await wasmSdkPage.executeStateTransitionAndGetResult();
  
  return result;
}

/**
 * Helper function to validate basic state transition result properties
 * @param {Object} result - The state transition result object
 */
function validateBasicStateTransitionResult(result) {
  expect(result.success).toBe(true);
  expect(result.result).toBeDefined();
  expect(result.hasError).toBe(false);
  expect(result.result).not.toContain('Error executing');
  expect(result.result).not.toContain('invalid');
  expect(result.result).not.toContain('failed');
}

/**
 * Filter out placeholder options from dropdown arrays
 * @param {string[]} options - Array of dropdown options
 * @returns {string[]} - Filtered array without placeholders
 */
function filterPlaceholderOptions(options) {
  return options.filter(option => 
    !option.toLowerCase().includes('select') && 
    option.trim() !== ''
  );
}

/**
 * Parse and validate JSON response structure
 * @param {string} resultStr - The raw result string
 * @returns {Object} - The parsed contract data
 */
function parseContractResponse(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const contractData = JSON.parse(resultStr);
  expect(contractData).toBeDefined();
  expect(contractData).toBeInstanceOf(Object);
  expect(contractData.status).toBe('success');
  expect(contractData.contractId).toBeDefined();
  expect(contractData.version).toBeDefined();
  expect(typeof contractData.version).toBe('number');
  expect(contractData.message).toBeDefined();
  return contractData;
}

/**
 * Helper function to validate data contract result (both create and update)
 * @param {string} resultStr - The raw result string from data contract operation
 * @param {boolean} isUpdate - Whether this is an update operation (default: false for create)
 * @returns {Object} - The parsed contract data for further use
 */
function validateDataContractResult(resultStr, isUpdate = false) {
  const contractData = parseContractResponse(resultStr);
  
  // Conditional validations based on operation type
  if (isUpdate) {
    // Update: only has version and message specifics
    expect(contractData.version).toBeGreaterThan(1); // Updates should increment version
    expect(contractData.message).toContain('updated successfully');
  } else {
    // Create: has additional fields that updates don't have
    expect(contractData.ownerId).toBeDefined();
    expect(contractData.documentTypes).toBeDefined();
    expect(Array.isArray(contractData.documentTypes)).toBe(true);
    expect(contractData.version).toBe(1); // Creates start at version 1
    expect(contractData.message).toContain('created successfully');
  }
  
  return contractData;
}

/**
 * Helper function to validate document creation result
 * @param {string} resultStr - The raw result string from document creation
 */
function validateDocumentCreateResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const documentData = JSON.parse(resultStr);
  expect(documentData).toBeDefined();
  expect(documentData).toBeInstanceOf(Object);
  
  // Should contain document or transaction data
  const hasDocumentField = 'document' in documentData || 
                          'documentId' in documentData || 
                          'id' in documentData ||
                          'transitionHash' in documentData ||
                          'stateTransitionHash' in documentData;
  
  expect(hasDocumentField).toBe(true);
}

/**
 * Execute a state transition with custom parameters
 * @param {WasmSdkPage} wasmSdkPage - The page object instance
 * @param {ParameterInjector} parameterInjector - The parameter injector instance
 * @param {string} category - State transition category
 * @param {string} transitionType - Transition type
 * @param {string} network - Network to use
 * @param {Object} customParams - Custom parameters to override test data
 * @returns {Promise<Object>} - The transition result object
 */
async function executeStateTransitionWithCustomParams(wasmSdkPage, parameterInjector, category, transitionType, network = 'testnet', customParams = {}) {
  await wasmSdkPage.setupStateTransition(category, transitionType);
  
  const success = await parameterInjector.injectStateTransitionParameters(category, transitionType, network, customParams);
  expect(success).toBe(true);
  
  const result = await wasmSdkPage.executeStateTransitionAndGetResult();
  
  return result;
}

test.describe('WASM SDK State Transition Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  test.describe('Data Contract State Transitions', () => {
    test.skip('should execute data contract create transition', async () => {
      // Execute the data contract create transition
      const result = await executeStateTransition(
        wasmSdkPage, 
        parameterInjector, 
        'dataContract', 
        'dataContractCreate',
        'testnet'
      );
      
      // Validate basic result structure
      validateBasicStateTransitionResult(result);
      
      // Validate data contract creation specific result
      validateDataContractResult(result.result, false);
      
      console.log('✅ Data contract create state transition completed successfully');
    });

    test.skip('should execute data contract update transition', async () => {
      // Execute the data contract update transition
      const result = await executeStateTransition(
        wasmSdkPage, 
        parameterInjector, 
        'dataContract', 
        'dataContractUpdate',
        'testnet'
      );
      
      // Validate basic result structure
      validateBasicStateTransitionResult(result);
      
      // Validate data contract update specific result
      validateDataContractResult(result.result, true);
      
      console.log('✅ Data contract update state transition completed successfully');
    });

    test('should create data contract and then update it with author field', async () => {
      // Set extended timeout for combined create+update operation
      test.setTimeout(180000);
      
      let contractId;
      
      // Step 1: Create contract (reported separately)
      await test.step('Create data contract', async () => {
        console.log('Creating new data contract...');
        const createResult = await executeStateTransition(
          wasmSdkPage, 
          parameterInjector, 
          'dataContract', 
          'dataContractCreate',
          'testnet'
        );
        
        // Validate create result
        validateBasicStateTransitionResult(createResult);
        validateDataContractResult(createResult.result, false);
        
        // Get the contract ID from create result
        contractId = JSON.parse(createResult.result).contractId;
        console.log('✅ Data contract created with ID:', contractId);
      });
      
      // Step 2: Update contract (reported separately) 
      await test.step('Update data contract with author field', async () => {
        console.log('🔄 Updating data contract to add author field...');
        const updateResult = await executeStateTransitionWithCustomParams(
          wasmSdkPage, 
          parameterInjector, 
          'dataContract', 
          'dataContractUpdate',
          'testnet',
          { dataContractId: contractId } // Override with dynamic contract ID
        );
        
        // Validate update result
        validateBasicStateTransitionResult(updateResult);
        validateDataContractResult(updateResult.result, true);
        
        console.log('✅ Data contract updated successfully with author field');
      });
    });

    test('should show authentication inputs for data contract transitions', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');
      
      // Check that authentication inputs are visible
      const hasAuthInputs = await wasmSdkPage.hasAuthenticationInputs();
      expect(hasAuthInputs).toBe(true);
      
      console.log('✅ Data contract state transition authentication inputs are visible');
    });
  });

  test.describe.skip('Document State Transitions', () => {
    test('should execute document create transition', async () => {
      // Execute the document create transition
      const result = await executeStateTransition(
        wasmSdkPage, 
        parameterInjector, 
        'document', 
        'documentCreate',
        'testnet'
      );
      
      // Validate basic result structure
      validateBasicStateTransitionResult(result);
      
      // Validate document creation specific result
      validateDocumentCreateResult(result.result);
      
      console.log('✅ Document create state transition completed successfully');
    });

    test('should execute document replace transition', async () => {
      // Execute the document replace transition
      const result = await executeStateTransition(
        wasmSdkPage, 
        parameterInjector, 
        'document', 
        'documentReplace',
        'testnet'
      );
      
      // Validate basic result structure
      validateBasicStateTransitionResult(result);
      
      // Validate document replace specific result
      validateDocumentCreateResult(result.result); // Same validation for replace
      
      console.log('✅ Document replace state transition completed successfully');
    });

    test('should execute document delete transition', async () => {
      // Execute the document delete transition
      const result = await executeStateTransition(
        wasmSdkPage, 
        parameterInjector, 
        'document', 
        'documentDelete',
        'testnet'
      );
      
      // Validate basic result structure
      validateBasicStateTransitionResult(result);
      
      // Document delete may have different response structure
      expect(result.result).toBeDefined();
      
      console.log('✅ Document delete state transition completed successfully');
    });

    test('should show authentication inputs for document transitions', async () => {
      await wasmSdkPage.setupStateTransition('document', 'documentCreate');
      
      // Check that authentication inputs are visible
      const hasAuthInputs = await wasmSdkPage.hasAuthenticationInputs();
      expect(hasAuthInputs).toBe(true);
      
      console.log('✅ Document state transition authentication inputs are visible');
    });
  });

  test.describe('Error Handling for State Transitions', () => {
    test('should handle invalid JSON schema gracefully', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');
      
      // Fill with invalid JSON schema
      const invalidParams = {
        canBeDeleted: false,
        readonly: false,
        keepsHistory: false,
        documentSchemas: 'invalid_json_here',
        identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
        privateKey: "XFfpaSbZq52HPy3WWwe1dXsZMiU1bQn8vQd34HNXkSZThevBWRn1"
      };
      
      await wasmSdkPage.fillStateTransitionParameters(invalidParams);
      
      // Execute the transition
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();
      
      // Should show error
      expect(result.hasError).toBe(true);
      expect(result.statusText.toLowerCase()).toMatch(/error|invalid|failed/);
      
      console.log('✅ Invalid JSON schema error handling works correctly');
    });

    test('should handle missing required fields', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');
      
      // Don't fill any parameters, try to execute
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();
      
      // Should show error or validation message
      expect(result.hasError).toBe(true);
      expect(result.statusText.toLowerCase()).toMatch(/error|required|missing/);
      
      console.log('✅ Missing required fields error handling works correctly');
    });

    test('should handle invalid private key gracefully', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');
      
      // Fill with invalid private key
      const invalidParams = {
        canBeDeleted: false,
        readonly: false,
        keepsHistory: false,
        documentSchemas: '{"note": {"type": "object", "properties": {"message": {"type": "string", "position": 0}}, "additionalProperties": false}}',
        identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
        privateKey: "invalid_private_key_here"
      };
      
      await wasmSdkPage.fillStateTransitionParameters(invalidParams);
      
      // Execute the transition
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();
      
      // Should show error
      expect(result.hasError).toBe(true);
      expect(result.statusText.toLowerCase()).toMatch(/error|invalid|failed/);
      
      console.log('✅ Invalid private key error handling works correctly');
    });
  });

  test.describe('UI State and Navigation', () => {
    test('should switch to state transitions operation type correctly', async () => {
      // Start with queries, then switch to transitions
      await wasmSdkPage.setOperationType('queries');
      await wasmSdkPage.page.waitForTimeout(500);
      
      await wasmSdkPage.setOperationType('transitions');
      
      // Verify the operation type is set correctly
      const operationType = await wasmSdkPage.page.locator('#operationType').inputValue();
      expect(operationType).toBe('transitions');
      
      console.log('✅ Successfully switched to state transitions operation type');
    });

    test('should populate transition categories correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      
      // Get available categories and filter out placeholders
      const allCategories = await wasmSdkPage.getAvailableQueryCategories();
      const categories = filterPlaceholderOptions(allCategories);
      
      // Define expected state transition categories
      const expectedCategories = [
        'Identity Transitions',
        'Data Contract Transitions', 
        'Document Transitions',
        'Token Transitions',
        'Voting Transitions'
      ];
      
      // Verify exact match - contains all expected and no unexpected ones
      expect(categories).toHaveLength(expectedCategories.length);
      expectedCategories.forEach(expectedCategory => {
        expect(categories).toContain(expectedCategory);
      });
      
      console.log('✅ State transition categories populated correctly:', categories);
    });

    test('should populate identity transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('identity');
      
      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);
      
      // Define expected identity transition types
      const expectedTransitionTypes = [
        'Identity Create',
        'Identity Top Up', 
        'Identity Update',
        'Identity Credit Transfer',
        'Identity Credit Withdrawal'
      ];
      
      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });
      
      console.log('✅ Identity transition types populated correctly:', transitionTypes);
    });

    test('should populate data contract transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('dataContract');
      
      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);
      
      // Define expected data contract transition types
      const expectedTransitionTypes = [
        'Data Contract Create',
        'Data Contract Update'
      ];
      
      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });
      
      console.log('✅ Data contract transition types populated correctly:', transitionTypes);
    });

    test('should populate document transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('document');
      
      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);
      
      // Define expected document transition types
      const expectedTransitionTypes = [
        'Document Create',
        'Document Replace',
        'Document Delete',
        'Document Transfer',
        'Document Purchase',
        'Document Set Price',
        'DPNS Register Name'
      ];
      
      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });
      
      console.log('✅ Document transition types populated correctly:', transitionTypes);
    });

    test('should populate token transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('token');
      
      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);
      
      // Define expected token transition types (based on docs.html)
      const expectedTransitionTypes = [
        'Token Burn',
        'Token Mint',
        'Token Claim',
        'Token Set Price',
        'Token Direct Purchase', 
        'Token Config Update',
        'Token Transfer',
        'Token Freeze',
        'Token Unfreeze',
        'Token Destroy Frozen'
      ];
      
      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });
      
      console.log('✅ Token transition types populated correctly:', transitionTypes);
    });

    test('should populate voting transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('voting');
      
      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);
      
      // Define expected voting transition types
      const expectedTransitionTypes = [
        'DPNS Username',
        'Contested Resource'
      ];
      
      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });
      
      console.log('✅ Voting transition types populated correctly:', transitionTypes);
    });
  });

});
