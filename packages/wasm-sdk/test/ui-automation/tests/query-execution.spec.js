const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');
const { ParameterInjector } = require('../utils/parameter-injector');

/**
 * Helper function to execute a query with proof toggle enabled
 * @param {WasmSdkPage} wasmSdkPage - The page object instance
 * @param {ParameterInjector} parameterInjector - The parameter injector instance
 * @param {string} category - Query category (e.g., 'identity', 'documents')
 * @param {string} queryName - Query name (e.g., 'getIdentity')
 * @param {string} network - Network to use ('testnet' or 'mainnet')
 * @returns {Promise<Object>} - The query result object
 */
async function executeQueryWithProof(wasmSdkPage, parameterInjector, category, queryName, network = 'testnet') {
  await wasmSdkPage.setupQuery(category, queryName);
  
  // Enable proof info if available
  const proofEnabled = await wasmSdkPage.enableProofInfo();
  
  // If proof was enabled, wait for the toggle to be actually checked
  if (proofEnabled) {
    const proofToggle = wasmSdkPage.page.locator('#proofToggle');
    await expect(proofToggle).toBeChecked();
    console.log('Proof toggle confirmed as checked');
  }
  
  const success = await parameterInjector.injectParameters(category, queryName, network);
  expect(success).toBe(true);
  
  const result = await wasmSdkPage.executeQueryAndGetResult();
  
  return { result, proofEnabled };
}

/**
 * Helper function to parse balance/nonce responses that may contain large numbers
 * @param {string} resultStr - The raw result string from the query
 * @param {string} propertyName - The property name to extract (e.g., 'balance', 'nonce')
 * @returns {number} - The parsed number value
 */
function parseNumericResult(resultStr, propertyName = 'balance') {
  const trimmedStr = resultStr.trim();
  
  // Try to parse as JSON first (in case it's a JSON response)
  let numericValue;
  try {
    const parsed = JSON.parse(trimmedStr);
    
    // Check if it's a JSON object with the expected property
    if (typeof parsed === 'object' && parsed[propertyName] !== undefined) {
      numericValue = Number(parsed[propertyName]);
    } else if (typeof parsed === 'number') {
      numericValue = parsed;
    } else {
      numericValue = Number(parsed);
    }
  } catch {
    // If not JSON, try parsing directly as number
    numericValue = Number(trimmedStr);
    
    // If Number() fails, log the issue
    if (isNaN(numericValue)) {
      console.error(`Failed to parse ${propertyName}:`, trimmedStr, 'type:', typeof trimmedStr);
    }
  }
  
  return numericValue;
}

/**
 * Helper function to validate basic query result properties
 * @param {Object} result - The query result object
 */
function validateBasicQueryResult(result) {
  expect(result.success).toBe(true);
  expect(result.result).toBeDefined();
  expect(result.hasError).toBe(false);
  expect(result.result).not.toContain('Error executing query');
  expect(result.result).not.toContain('not found');
  expect(result.result).not.toContain('invalid');
}

/**
 * Helper function to validate proof content contains expected fields
 * @param {string} proofContent - The proof content string
 */
function validateProofContent(proofContent) {
  expect(proofContent).toBeDefined();
  expect(proofContent).not.toBe('');
  expect(proofContent).toContain('metadata');
  expect(proofContent).toContain('proof');
  expect(proofContent).toContain('grovedbProof');
  expect(proofContent).toContain('quorumHash');
  expect(proofContent).toContain('signature');
}

/**
 * Helper function to validate split view (proof mode) result
 * @param {Object} result - The query result object
 */
function validateSplitView(result) {
  expect(result.inSplitView).toBe(true);
  expect(result.proofContent).toBeDefined();
  expect(result.proofContent).not.toBe('');
  validateProofContent(result.proofContent);
}

/**
 * Helper function to validate single view (non-proof mode) result
 * @param {Object} result - The query result object
 */
function validateSingleView(result) {
  expect(result.inSplitView).toBe(false);
  expect(result.proofContent).toBeNull();
}

/**
 * Helper function to validate data contract result
 * @param {string} resultStr - The raw result string containing contract data
 */
function validateContractResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const contractData = JSON.parse(resultStr);
  expect(contractData).toBeDefined();
  expect(contractData).toHaveProperty('id');
  expect(contractData).toHaveProperty('config');
}

/**
 * Helper function to validate document result
 * @param {string} resultStr - The raw result string containing document data
 */
function validateDocumentResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const documentData = JSON.parse(resultStr);
  expect(documentData).toBeDefined();
  // Documents can be arrays or single objects
  if (Array.isArray(documentData)) {
    expect(documentData.length).toBeGreaterThanOrEqual(0);
  } else {
    expect(documentData).toBeInstanceOf(Object);
  }
}

/**
 * Helper function to validate numeric results and ensure they're valid
 * @param {string} resultStr - The raw result string
 * @param {string} propertyName - The property name to extract
 * @returns {number} - The validated numeric value
 */
function validateNumericResult(resultStr, propertyName = 'balance') {
  const numericValue = parseNumericResult(resultStr, propertyName);
  expect(numericValue).not.toBeNaN();
  expect(numericValue).toBeGreaterThanOrEqual(0);
  return numericValue;
}

/**
 * Specific validation functions for parameterized tests
 */
function validateIdentityResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const identityData = JSON.parse(resultStr);
  expect(identityData).toHaveProperty('id');
  expect(identityData).toHaveProperty('publicKeys');
  expect(identityData).toHaveProperty('balance');
}

function validateKeysResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const keysData = JSON.parse(resultStr);
  expect(keysData).toBeDefined();
}

function validateIdentitiesResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const identitiesData = JSON.parse(resultStr);
  expect(identitiesData).toBeDefined();
  
  if (Array.isArray(identitiesData)) {
    expect(identitiesData.length).toBeGreaterThanOrEqual(0);
    // Validate each identity using the single identity validator
    identitiesData.forEach(identity => {
      validateIdentityResult(JSON.stringify(identity));
    });
  } else {
    // Single identity - use the existing validator
    validateIdentityResult(JSON.stringify(identitiesData));
  }
}

function validateBalancesResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const balancesData = JSON.parse(resultStr);
  expect(balancesData).toBeDefined();
  if (Array.isArray(balancesData)) {
    expect(balancesData.length).toBeGreaterThanOrEqual(0);
  }
}

function validateBalanceAndRevisionResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const data = JSON.parse(resultStr);
  expect(data).toBeDefined();
  expect(data).toBeInstanceOf(Object);
}

function validateTokenBalanceResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const tokenData = JSON.parse(resultStr);
  expect(tokenData).toBeDefined();
}

function validateTokenInfoResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const tokenInfoData = JSON.parse(resultStr);
  expect(tokenInfoData).toBeDefined();
}

test.describe('WASM SDK Query Execution Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  test.describe('Data Contract Queries', () => {
    test('should execute getDataContract query', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContract');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContract', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Use helper functions for validation
      validateBasicQueryResult(result);
      validateSingleView(result);
      validateContractResult(result.result);
      
      console.log('✅ getDataContract single view without proof confirmed');
    });

    test('should execute getDataContracts query for multiple contracts', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContracts');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContracts', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Use helper functions for validation
      validateBasicQueryResult(result);
      validateSingleView(result);
      
      // Multiple contracts result should be valid JSON
      expect(() => JSON.parse(result.result)).not.toThrow();
      const contractsData = JSON.parse(result.result);
      expect(contractsData).toBeDefined();
      
      console.log('✅ getDataContracts single view without proof confirmed');
    });

    test('should execute getDataContractHistory query', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContractHistory');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContractHistory', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Use helper functions for validation
      validateBasicQueryResult(result);
      validateSingleView(result);
      
      // Contract history should be valid JSON (array of contract versions)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const historyData = JSON.parse(result.result);
      expect(historyData).toBeDefined();
      expect(Array.isArray(historyData) || typeof historyData === 'object').toBe(true);
      
      console.log('✅ getDataContractHistory single view without proof confirmed');
    });

    test('should execute getDataContract query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'dataContract', 
        'getDataContract',
        'testnet'
      );
      
      // Validate basic result
      validateBasicQueryResult(result);
      validateContractResult(result.result);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        console.log('✅ getDataContract split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getDataContract query');
      }
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getDataContracts
    test.skip('should execute getDataContracts query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'dataContract', 
        'getDataContracts',
        'testnet'
      );
      
      // Validate basic result
      validateBasicQueryResult(result);
      
      // Multiple contracts result should be valid JSON
      expect(() => JSON.parse(result.result)).not.toThrow();
      const contractsData = JSON.parse(result.result);
      expect(contractsData).toBeDefined();
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        console.log('✅ getDataContracts split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getDataContracts query');
      }
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getDataContractHistory
    test.skip('should execute getDataContractHistory query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'dataContract', 
        'getDataContractHistory',
        'testnet'
      );
      
      // Validate basic result
      validateBasicQueryResult(result);
      
      // Contract history should be valid JSON
      expect(() => JSON.parse(result.result)).not.toThrow();
      const historyData = JSON.parse(result.result);
      expect(historyData).toBeDefined();
      expect(Array.isArray(historyData) || typeof historyData === 'object').toBe(true);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        console.log('✅ getDataContractHistory split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getDataContractHistory query');
      }
    });
  });

  test.describe('Document Queries', () => {
    test('should execute getDocuments query', async () => {
      await wasmSdkPage.setupQuery('document', 'getDocuments');
      
      const success = await parameterInjector.injectParameters('document', 'getDocuments', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Use helper functions for validation
      validateBasicQueryResult(result);
      validateSingleView(result);
      validateDocumentResult(result.result);
      
      console.log('✅ getDocuments single view without proof confirmed');
    });

    test('should execute getDocument query for specific document', async () => {
      await wasmSdkPage.setupQuery('document', 'getDocument');
      
      const success = await parameterInjector.injectParameters('document', 'getDocument', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Use helper functions for validation
      validateBasicQueryResult(result);
      validateSingleView(result);
      validateDocumentResult(result.result);
      
      console.log('✅ getDocument single view without proof confirmed');
    });

    test('should execute getDocuments query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'document', 
        'getDocuments',
        'testnet'
      );
      
      // Validate basic result
      validateBasicQueryResult(result);
      validateDocumentResult(result.result);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        console.log('✅ getDocuments split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getDocuments query');
      }
    });

    test('should execute getDocument query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'document', 
        'getDocument',
        'testnet'
      );
      
      // Validate basic result
      validateBasicQueryResult(result);
      validateDocumentResult(result.result);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        console.log('✅ getDocument split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getDocument query');
      }
    });
  });

  test.describe('System Queries', () => {
    test('should execute getStatus query', async () => {
      await wasmSdkPage.setupQuery('system', 'getStatus');
      
      // Status query needs no parameters
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Status should generally succeed
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      expect(result.result).toContain('version');
      
    });

    test('should execute getCurrentEpoch query', async () => {
      await wasmSdkPage.setupQuery('epoch', 'getCurrentEpoch');
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain epoch data (number or JSON with epoch info)
      expect(result.result).toMatch(/\d+|epoch/i);
      
    });

    test('should execute getTotalCreditsInPlatform query', async () => {
      await wasmSdkPage.setupQuery('system', 'getTotalCreditsInPlatform');
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain credits data (number or JSON with credits info)
      expect(result.result).toMatch(/\d+|credits|balance/i);
      
    });
  });

  test.describe('Error Handling', () => {
    test('should handle invalid identity ID gracefully', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Fill with invalid ID (contains invalid base58 characters '0', 'O', 'I', 'l')
      await wasmSdkPage.fillQueryParameters({ id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4SOIl0' });
      
      // Click execute button directly
      const executeButton = wasmSdkPage.page.locator('#executeQuery');
      await executeButton.click();
      
      // Wait a bit for the error to appear
      await wasmSdkPage.page.waitForTimeout(1000);
      
      // Check for error status
      const statusBanner = wasmSdkPage.page.locator('#statusBanner');
      const statusClass = await statusBanner.getAttribute('class');
      const statusText = await wasmSdkPage.getStatusBannerText();
      
      // Should show error
      expect(statusClass).toContain('error');
      expect(statusText).toBeTruthy();
      
      console.log('Error handling result:', statusText);
    });

    test('should handle empty required fields', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Don't fill any parameters, try to execute
      const executeButton = wasmSdkPage.page.locator('#executeQuery');
      await executeButton.click();
      
      // Wait a bit for the error to appear
      await wasmSdkPage.page.waitForTimeout(1000);
      
      // Check for error status
      const statusBanner = wasmSdkPage.page.locator('#statusBanner');
      const statusClass = await statusBanner.getAttribute('class');
      const statusText = await wasmSdkPage.getStatusBannerText();
      
      // Should show error or validation message
      expect(statusClass).toContain('error');
      expect(statusText).toContain('required');
      
      console.log('Empty fields handling:', statusText);
    });
  });


  test.describe('Network Switching', () => {
    test('should execute queries on mainnet', async () => {
      // Switch to mainnet
      await wasmSdkPage.setNetwork('mainnet');
      
      await wasmSdkPage.setupQuery('system', 'getStatus');
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain status data with version info
      expect(result.result).toContain('version');
      
    });
  });

  // Test Identity Queries
  test.describe('Identity Queries', () => {
    // Complete set of all available identity queries with correct proof support
    const testQueries = [
      { name: 'getIdentity', hasProofSupport: true, validateFn: validateIdentityResult },
      { name: 'getIdentityBalance', hasProofSupport: true, validateFn: (result) => validateNumericResult(result, 'balance') },
      { name: 'getIdentityKeys', hasProofSupport: true, validateFn: validateKeysResult },
      { name: 'getIdentityNonce', hasProofSupport: true, validateFn: (result) => validateNumericResult(result, 'nonce') },
      { name: 'getIdentityContractNonce', hasProofSupport: true, validateFn: (result) => validateNumericResult(result, 'nonce') },
      { name: 'getIdentityByPublicKeyHash', hasProofSupport: true, validateFn: validateIdentityResult },
      { name: 'getIdentitiesContractKeys', hasProofSupport: true, validateFn: validateKeysResult },
      { name: 'getIdentitiesBalances', hasProofSupport: true, validateFn: validateBalancesResult },
      { name: 'getIdentityBalanceAndRevision', hasProofSupport: true, validateFn: validateBalanceAndRevisionResult },
      { name: 'getIdentityByNonUniquePublicKeyHash', hasProofSupport: true, validateFn: validateIdentitiesResult },
      { name: 'getIdentityTokenBalances', hasProofSupport: true, validateFn: validateTokenBalanceResult },
      { name: 'getIdentitiesTokenBalances', hasProofSupport: true, validateFn: validateTokenBalanceResult },
      { name: 'getIdentityTokenInfos', hasProofSupport: true, validateFn: validateTokenInfoResult },
      { name: 'getIdentitiesTokenInfos', hasProofSupport: true, validateFn: validateTokenInfoResult }
    ];

    testQueries.forEach(({ name, hasProofSupport, validateFn }) => {
      test.describe(`${name} query (parameterized)`, () => {
        test('without proof info', async () => {
          await wasmSdkPage.setupQuery('identity', name);
          await wasmSdkPage.disableProofInfo();
          
          const success = await parameterInjector.injectParameters('identity', name, 'testnet');
          expect(success).toBe(true);
          
          const result = await wasmSdkPage.executeQueryAndGetResult();
          validateBasicQueryResult(result);
          expect(result.result.length).toBeGreaterThan(0);
          validateSingleView(result);
          validateFn(result.result);
          
          console.log(`✅ ${name} without proof - PASSED`);
        });

        if (hasProofSupport) {
          test('with proof info', async () => {
            const { result, proofEnabled } = await executeQueryWithProof(
              wasmSdkPage, 
              parameterInjector, 
              'identity', 
              name,
              'testnet'
            );
            
            validateBasicQueryResult(result);
            expect(result.result.length).toBeGreaterThan(0);
            
            if (proofEnabled) {
              validateSplitView(result);
              validateFn(result.result);
              console.log(`✅ ${name} with proof - PASSED`);
            } else {
              console.log(`⚠️ Proof was not enabled for ${name} query`);
              validateFn(result.result);
            }
          });
        } else {
          test.skip('with proof info', async () => {
            // Proof support not yet implemented in WASM SDK for this query
          });
        }
      });
    });
  });
});
