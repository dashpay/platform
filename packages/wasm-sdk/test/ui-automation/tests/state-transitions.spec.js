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
 * Helper function to validate data contract creation result
 * @param {string} resultStr - The raw result string from data contract creation
 */
function validateDataContractCreateResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const contractData = JSON.parse(resultStr);
  expect(contractData).toBeDefined();
  expect(contractData).toBeInstanceOf(Object);
  
  // Validate the expected response structure
  expect(contractData.status).toBe('success');
  expect(contractData.contractId).toBeDefined();
  expect(contractData.ownerId).toBeDefined();
  expect(contractData.version).toBeDefined();
  expect(contractData.documentTypes).toBeDefined();
  expect(Array.isArray(contractData.documentTypes)).toBe(true);
  expect(contractData.message).toBeDefined();
  expect(contractData.message).toContain('successfully');
}

/**
 * Helper function to validate data contract update result
 * @param {string} resultStr - The raw result string from data contract update
 */
function validateDataContractUpdateResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const contractData = JSON.parse(resultStr);
  expect(contractData).toBeDefined();
  expect(contractData).toBeInstanceOf(Object);
  
  // Validate the expected response structure for updates
  expect(contractData.status).toBe('success');
  expect(contractData.contractId).toBeDefined();
  expect(contractData.version).toBeDefined();
  expect(typeof contractData.version).toBe('number');
  expect(contractData.version).toBeGreaterThan(1); // Updates should increment version
  expect(contractData.message).toBeDefined();
  expect(contractData.message).toContain('updated successfully');
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

test.describe('WASM SDK State Transition Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  test.describe('Data Contract State Transitions', () => {
    test('should execute data contract create transition', async () => {
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
      validateDataContractCreateResult(result.result);
      
      console.log('✅ Data contract create state transition completed successfully');
    });

    test('should execute data contract update transition', async () => {
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
      validateDataContractUpdateResult(result.result);
      
      console.log('✅ Data contract update state transition completed successfully');
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

  test.describe.skip('Error Handling for State Transitions', () => {
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
      await wasmSdkPage.setupStateTransition('document', 'documentCreate');
      
      // Fill with invalid private key
      const invalidParams = {
        contractId: "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
        documentType: "domain",
        documentData: '{"normalizedLabel":"testdomain","normalizedParentDomainName":"dash","label":"testdomain","parentDomainName":"dash"}',
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

  test.describe.skip('UI State and Navigation', () => {
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
      
      // Get available categories
      const categories = await wasmSdkPage.getAvailableQueryCategories();
      
      // Should contain identity transitions
      expect(categories).toContain('Identity Transitions');
      
      console.log('✅ State transition categories populated correctly:', categories);
    });

    test('should populate identity transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('identity');
      
      // Get available transition types
      const transitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      
      // Should contain identity create and top-up
      expect(transitionTypes).toContain('Identity Create');
      expect(transitionTypes).toContain('Identity Top Up');
      
      console.log('✅ Identity transition types populated correctly:', transitionTypes);
    });
  });

  test.describe.skip('Network Switching for State Transitions', () => {
    test('should execute state transitions on testnet', async () => {
      // Ensure we're on testnet
      await wasmSdkPage.setNetwork('testnet');
      
      const result = await executeStateTransition(
        wasmSdkPage, 
        parameterInjector, 
        'dataContract', 
        'dataContractCreate',
        'testnet'
      );
      
      // Verify transition executed successfully on testnet
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      expect(result.hasError).toBe(false);
      
      console.log('✅ Data contract state transition executed successfully on testnet');
    });
  });
});