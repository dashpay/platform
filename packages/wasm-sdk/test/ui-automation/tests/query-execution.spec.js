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

test.describe('WASM SDK Query Execution Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  test.describe('Identity Queries', () => {
    test('should execute getIdentity query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      // Inject test parameters
      const success = await parameterInjector.injectParameters('identity', 'getIdentity', 'testnet');
      expect(success).toBe(true);
      
      // Execute query
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed without network errors
      validateBasicQueryResult(result);
      expect(result.result.length).toBeGreaterThan(0);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain identity data (valid JSON with expected fields)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toHaveProperty('id');
      expect(identityData).toHaveProperty('publicKeys');
      expect(identityData).toHaveProperty('balance');
      
      console.log('getIdentity single view without proof confirmed');
    });

    test('should execute getIdentity query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentity',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      expect(result.result.length).toBeGreaterThan(0);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        // Verify data section still contains identity data (valid JSON with expected fields)
        expect(() => JSON.parse(result.result)).not.toThrow();
        const identityData = JSON.parse(result.result);
        expect(identityData).toHaveProperty('id');
        expect(identityData).toHaveProperty('publicKeys');
        expect(identityData).toHaveProperty('balance');
        
        console.log('getIdentity split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentity query');
        // Should still contain identity data (valid JSON with expected fields)
        expect(() => JSON.parse(result.result)).not.toThrow();
        const identityData = JSON.parse(result.result);
        expect(identityData).toHaveProperty('id');
        expect(identityData).toHaveProperty('publicKeys');
        expect(identityData).toHaveProperty('balance');
      }
    });

    test('should execute getIdentityBalance query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityBalance');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityBalance', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain balance data (should be a number or numeric string)
      validateNumericResult(result.result, 'balance');
      
      console.log('getIdentityBalance single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityBalance
    test.skip('should execute getIdentityBalance query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityBalance',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain balance data in data section
      validateNumericResult(result.result, 'balance');
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentityBalance split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityBalance query');
      }
      
    });

    test('should execute getIdentityKeys query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityKeys');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityKeys', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain keys data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const keysData = JSON.parse(result.result);
      expect(keysData).toBeDefined();
      
      console.log('getIdentityKeys single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityKeys
    test.skip('should execute getIdentityKeys query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityKeys',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain keys data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const keysData = JSON.parse(result.result);
      expect(keysData).toBeDefined();
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentityKeys split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityKeys query');
      }
      
    });

    test('should execute getIdentitiesContractKeys query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesContractKeys');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesContractKeys', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain contract keys data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const contractKeysData = JSON.parse(result.result);
      expect(contractKeysData).toBeDefined();
      
      console.log('getIdentitiesContractKeys single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentitiesContractKeys
    test.skip('should execute getIdentitiesContractKeys query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentitiesContractKeys',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain contract keys data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const contractKeysData = JSON.parse(result.result);
      expect(contractKeysData).toBeDefined();
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentitiesContractKeys split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentitiesContractKeys query');
      }
      
    });

    test('should execute getIdentityNonce query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityNonce');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityNonce', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain nonce data (should be a number)
      validateNumericResult(result.result, 'nonce');
      
      console.log('getIdentityNonce single view without proof confirmed');
    });

    test('should execute getIdentityNonce query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityNonce',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain nonce data in data section
      validateNumericResult(result.result, 'nonce');
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentityNonce split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityNonce query');
      }
      
    });

    test('should execute getIdentityContractNonce query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityContractNonce');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityContractNonce', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain contract nonce data (should be a number)
      validateNumericResult(result.result, 'nonce');
      
      console.log('getIdentityContractNonce single view without proof confirmed');
    });

    test('should execute getIdentityContractNonce query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityContractNonce',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain contract nonce data in data section (should be a number)
      validateNumericResult(result.result, 'nonce');
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentityContractNonce split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityContractNonce query');
      }
      
    });

    test('should execute getIdentitiesBalances query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesBalances');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesBalances', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain balances data (valid JSON with multiple balance entries)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const balancesData = JSON.parse(result.result);
      expect(balancesData).toBeDefined();
      
      // Should be an array or object with balance information
      expect(Array.isArray(balancesData) || typeof balancesData === 'object').toBe(true);
      
      console.log('getIdentitiesBalances single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentitiesBalances
    test.skip('should execute getIdentitiesBalances query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentitiesBalances',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain balances data in data section (valid JSON with multiple balance entries)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const balancesData = JSON.parse(result.result);
      expect(balancesData).toBeDefined();
      expect(Array.isArray(balancesData) || typeof balancesData === 'object').toBe(true);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentitiesBalances split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentitiesBalances query');
      }
      
    });

    test('should execute getIdentityBalanceAndRevision query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityBalanceAndRevision');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityBalanceAndRevision', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain balance and revision data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const balanceRevisionData = JSON.parse(result.result);
      expect(balanceRevisionData).toBeDefined();
      
      // Should have both balance and revision properties
      expect(balanceRevisionData).toHaveProperty('balance');
      expect(balanceRevisionData).toHaveProperty('revision');
      
      // Validate balance using helper function
      validateNumericResult(JSON.stringify(balanceRevisionData), 'balance');
      
      // Validate revision (should be a number >= 0)
      const revision = Number(balanceRevisionData.revision);
      expect(revision).not.toBeNaN();
      expect(revision).toBeGreaterThanOrEqual(0);
      
      console.log('getIdentityBalanceAndRevision single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityBalanceAndRevision
    test.skip('should execute getIdentityBalanceAndRevision query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityBalanceAndRevision',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain balance and revision data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const balanceRevisionData = JSON.parse(result.result);
      expect(balanceRevisionData).toBeDefined();
      expect(balanceRevisionData).toHaveProperty('balance');
      expect(balanceRevisionData).toHaveProperty('revision');
      
      // Validate balance using helper function
      validateNumericResult(JSON.stringify(balanceRevisionData), 'balance');
      
      // Validate revision (should be a number >= 0)
      const revision = Number(balanceRevisionData.revision);
      expect(revision).not.toBeNaN();
      expect(revision).toBeGreaterThanOrEqual(0);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('getIdentityBalanceAndRevision split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityBalanceAndRevision query');
      }
      
    });

    test('should execute getIdentityByNonUniquePublicKeyHash query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityByNonUniquePublicKeyHash');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityByNonUniquePublicKeyHash', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain identity data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toBeDefined();
      
      console.log('getIdentityByNonUniquePublicKeyHash single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityByNonUniquePublicKeyHash
    test.skip('should execute getIdentityByNonUniquePublicKeyHash query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityByNonUniquePublicKeyHash',
        'testnet'
      );
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain identity data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toBeDefined();
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('getIdentityByNonUniquePublicKeyHash split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityByNonUniquePublicKeyHash query');
      }
      
    });

    test('should execute getIdentityByPublicKeyHash query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityByPublicKeyHash');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityByPublicKeyHash', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain identity data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toBeDefined();
      
      // Should be a single identity object (unique public key hash)
      expect(identityData).toHaveProperty('id');
      expect(identityData.id).toBeDefined();
      expect(identityData.publicKeys).toBeDefined();
      
      console.log('getIdentityByPublicKeyHash single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityByPublicKeyHash
    test.skip('should execute getIdentityByPublicKeyHash query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityByPublicKeyHash',
        'testnet'
      );
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain identity data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toBeDefined();
      expect(identityData).toHaveProperty('id');
      expect(identityData.id).toBeDefined();
      expect(identityData.publicKeys).toBeDefined();
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('getIdentityByPublicKeyHash split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityByPublicKeyHash query');
      }
      
    });

    test('should execute getIdentityTokenBalances query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityTokenBalances');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityTokenBalances', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain token balance data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenBalancesData = JSON.parse(result.result);
      expect(tokenBalancesData).toBeDefined();
      expect(Array.isArray(tokenBalancesData) || typeof tokenBalancesData === 'object').toBe(true);
      
      console.log('getIdentityTokenBalances single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityTokenBalances
    test.skip('should execute getIdentityTokenBalances query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityTokenBalances',
        'testnet'
      );
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token balance data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenBalancesData = JSON.parse(result.result);
      expect(tokenBalancesData).toBeDefined();
      expect(Array.isArray(tokenBalancesData) || typeof tokenBalancesData === 'object').toBe(true);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('getIdentityTokenBalances split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityTokenBalances query');
      }
      
    });

    test('should execute getIdentitiesTokenBalances query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesTokenBalances');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesTokenBalances', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain token balance data for multiple identities (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenBalancesData = JSON.parse(result.result);
      expect(tokenBalancesData).toBeDefined();
      expect(Array.isArray(tokenBalancesData) || typeof tokenBalancesData === 'object').toBe(true);
      
      console.log('getIdentitiesTokenBalances single view without proof confirmed');
    });

    test('should execute getIdentitiesTokenBalances query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentitiesTokenBalances',
        'testnet'
      );
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should contain token balance data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenBalancesData = JSON.parse(result.result);
      expect(tokenBalancesData).toBeDefined();
      expect(Array.isArray(tokenBalancesData) || typeof tokenBalancesData === 'object').toBe(true);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        validateSplitView(result);
        
        console.log('getIdentitiesTokenBalances split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentitiesTokenBalances query');
      }
      
    });

    test('should execute getIdentityTokenInfos query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityTokenInfos');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityTokenInfos', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain token info data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenInfoData = JSON.parse(result.result);
      expect(tokenInfoData).toBeDefined();
      expect(Array.isArray(tokenInfoData) || typeof tokenInfoData === 'object').toBe(true);
      
      console.log('getIdentityTokenInfos single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentityTokenInfos
    test.skip('should execute getIdentityTokenInfos query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentityTokenInfos',
        'testnet'
      );
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token info data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenInfoData = JSON.parse(result.result);
      expect(tokenInfoData).toBeDefined();
      expect(Array.isArray(tokenInfoData) || typeof tokenInfoData === 'object').toBe(true);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('getIdentityTokenInfos split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentityTokenInfos query');
      }
      
    });

    test('should execute getIdentitiesTokenInfos query without proof info', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesTokenInfos');
      
      // Ensure proof info is disabled
      await wasmSdkPage.disableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesTokenInfos', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      validateBasicQueryResult(result);
      
      // Should be in single view (no proof)
      validateSingleView(result);
      
      // Should contain token info data for multiple identities (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenInfoData = JSON.parse(result.result);
      expect(tokenInfoData).toBeDefined();
      expect(Array.isArray(tokenInfoData) || typeof tokenInfoData === 'object').toBe(true);
      
      console.log('getIdentitiesTokenInfos single view without proof confirmed');
    });

    // Skip this test - proof support not yet implemented in WASM SDK for getIdentitiesTokenInfos
    test.skip('should execute getIdentitiesTokenInfos query with proof info', async () => {
      const { result, proofEnabled } = await executeQueryWithProof(
        wasmSdkPage, 
        parameterInjector, 
        'identity', 
        'getIdentitiesTokenInfos',
        'testnet'
      );
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token info data in data section (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenInfoData = JSON.parse(result.result);
      expect(tokenInfoData).toBeDefined();
      expect(Array.isArray(tokenInfoData) || typeof tokenInfoData === 'object').toBe(true);
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('getIdentitiesTokenInfos split view with proof confirmed');
      } else {
        console.log('⚠️ Proof was not enabled for getIdentitiesTokenInfos query');
      }
      
    });
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
      
      // Fill with invalid ID
      await wasmSdkPage.fillQueryParameters({ id: 'invalid_identity_id' });
      
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
});
