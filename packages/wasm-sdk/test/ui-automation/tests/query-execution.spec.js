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
  console.log(`Raw ${propertyName} result:`, JSON.stringify(trimmedStr));
  
  // Try to parse as JSON first (in case it's a JSON response)
  let numericValue;
  try {
    const parsed = JSON.parse(trimmedStr);
    
    // Check if it's a JSON object with the expected property
    if (typeof parsed === 'object' && parsed[propertyName] !== undefined) {
      numericValue = Number(parsed[propertyName]);
      console.log(`Parsed as JSON object with ${propertyName} property:`, parsed[propertyName], 'converted to:', numericValue);
    } else if (typeof parsed === 'number') {
      numericValue = parsed;
      console.log(`Parsed as JSON number:`, numericValue);
    } else {
      numericValue = Number(parsed);
      console.log(`Parsed as JSON and converted to number:`, numericValue);
    }
  } catch {
    // If not JSON, try parsing directly as number
    numericValue = Number(trimmedStr);
    console.log(`Parsed as direct number:`, numericValue);
    
    // If Number() fails, log the issue
    if (isNaN(numericValue)) {
      console.error(`Failed to parse ${propertyName}:`, trimmedStr, 'type:', typeof trimmedStr);
    }
  }
  
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
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      expect(result.result.length).toBeGreaterThan(0);
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      expect(result.result).not.toContain('invalid');
      
      // Should be in single view (no proof)
      expect(result.inSplitView).toBe(false);
      expect(result.proofContent).toBeNull();
      
      // Should contain identity data (valid JSON with expected fields)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toHaveProperty('id');
      expect(identityData).toHaveProperty('publicKeys');
      expect(identityData).toHaveProperty('balance');
      
      console.log('✅ getIdentity single view without proof confirmed');
      console.log('Identity query result:', result.result.substring(0, 200) + '...');
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
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      expect(result.result.length).toBeGreaterThan(0);
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      expect(result.result).not.toContain('invalid');
      
      // If proof was enabled, verify split view
      if (proofEnabled) {
        expect(result.inSplitView).toBe(true);
        expect(result.proofContent).toBeDefined();
        expect(result.proofContent).not.toBe('');
        
        // Verify data section still contains identity data (valid JSON with expected fields)
        expect(() => JSON.parse(result.result)).not.toThrow();
        const identityData = JSON.parse(result.result);
        expect(identityData).toHaveProperty('id');
        expect(identityData).toHaveProperty('publicKeys');
        expect(identityData).toHaveProperty('balance');
        
        // Verify proof content contains expected fields
        expect(result.proofContent).toContain('metadata');
        expect(result.proofContent).toContain('proof');
        expect(result.proofContent).toContain('grovedbProof');
        expect(result.proofContent).toContain('quorumHash');
        expect(result.proofContent).toContain('signature');
        
        console.log('✅ getIdentity split view with proof confirmed');
        console.log('Data section length:', result.result.length);
        console.log('Proof section length:', result.proofContent.length);
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

    test('should execute getIdentityBalance query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityBalance');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityBalance', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain balance data (should be a number or numeric string)
      const balance = parseNumericResult(result.result, 'balance');
      
      expect(balance).not.toBeNaN();
      expect(balance).toBeGreaterThanOrEqual(0);
      
      console.log('Identity balance result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityKeys query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityKeys');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityKeys', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain keys data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const keysData = JSON.parse(result.result);
      expect(keysData).toBeDefined();
      
      console.log('Identity keys result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentitiesContractKeys query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesContractKeys');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesContractKeys', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain contract keys data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const contractKeysData = JSON.parse(result.result);
      expect(contractKeysData).toBeDefined();
      
      console.log('Identities contract keys result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityNonce query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityNonce');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityNonce', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain nonce data (should be a number)
      const nonce = parseNumericResult(result.result, 'nonce');
      
      expect(nonce).not.toBeNaN();
      expect(nonce).toBeGreaterThanOrEqual(0);
      
      console.log('Identity nonce result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityContractNonce query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityContractNonce');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityContractNonce', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain contract nonce data (should be a number)
      const contractNonce = parseNumericResult(result.result, 'nonce');
      
      expect(contractNonce).not.toBeNaN();
      expect(contractNonce).toBeGreaterThanOrEqual(0);
      
      console.log('Identity contract nonce result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentitiesBalances query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesBalances');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesBalances', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain balances data (valid JSON with multiple balance entries)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const balancesData = JSON.parse(result.result);
      expect(balancesData).toBeDefined();
      
      // Should be an array or object with balance information
      expect(Array.isArray(balancesData) || typeof balancesData === 'object').toBe(true);
      
      // If it's an array, each entry should have valid balance data
      if (Array.isArray(balancesData)) {
        balancesData.forEach((balanceEntry, index) => {
          expect(balanceEntry).toBeDefined();
          console.log(`Balance entry ${index}:`, JSON.stringify(balanceEntry).substring(0, 100) + '...');
          
          // Each entry should have balance information
          if (typeof balanceEntry === 'object' && balanceEntry.balance !== undefined) {
            const balance = Number(balanceEntry.balance);
            expect(balance).not.toBeNaN();
            expect(balance).toBeGreaterThanOrEqual(0);
          }
        });
      } else if (typeof balancesData === 'object') {
        // If it's an object, it might have balance properties for each identity
        Object.keys(balancesData).forEach(key => {
          const balanceEntry = balancesData[key];
          console.log(`Balance for ${key}:`, JSON.stringify(balanceEntry).substring(0, 100) + '...');
          
          if (typeof balanceEntry === 'object' && balanceEntry.balance !== undefined) {
            const balance = Number(balanceEntry.balance);
            expect(balance).not.toBeNaN();
            expect(balance).toBeGreaterThanOrEqual(0);
          }
        });
      }
      
      console.log('Identities balances result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityBalanceAndRevision query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityBalanceAndRevision');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityBalanceAndRevision', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain balance and revision data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const balanceRevisionData = JSON.parse(result.result);
      expect(balanceRevisionData).toBeDefined();
      
      // Should have both balance and revision properties
      expect(balanceRevisionData).toHaveProperty('balance');
      expect(balanceRevisionData).toHaveProperty('revision');
      
      // Validate balance using helper function
      const balance = parseNumericResult(JSON.stringify(balanceRevisionData), 'balance');
      expect(balance).not.toBeNaN();
      expect(balance).toBeGreaterThanOrEqual(0);
      
      // Validate revision (should be a number >= 0)
      const revision = Number(balanceRevisionData.revision);
      expect(revision).not.toBeNaN();
      expect(revision).toBeGreaterThanOrEqual(0);
      
      console.log('Identity balance and revision result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityByNonUniquePublicKeyHash query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityByNonUniquePublicKeyHash');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityByNonUniquePublicKeyHash', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain identity data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toBeDefined();
      
      // Could be an array of identities (non-unique) or a single identity
      if (Array.isArray(identityData)) {
        // If it's an array, each entry should be a valid identity
        identityData.forEach((identity, index) => {
          expect(identity).toBeDefined();
          expect(identity).toHaveProperty('id');
          console.log(`Identity ${index}:`, identity.id);
        });
      } else if (typeof identityData === 'object') {
        // If it's a single identity object
        expect(identityData).toHaveProperty('id');
        console.log('Found identity:', identityData.id);
      }
      
      console.log('Identity by non-unique public key hash result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityByPublicKeyHash query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityByPublicKeyHash');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityByPublicKeyHash', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain identity data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const identityData = JSON.parse(result.result);
      expect(identityData).toBeDefined();
      
      // Should be a single identity object (unique public key hash)
      expect(identityData).toHaveProperty('id');
      expect(identityData.id).toBeDefined();
      expect(identityData.publicKeys).toBeDefined();
      
      // Log the identity ID for verification
      console.log('Found identity by unique public key hash:', identityData.id);
      console.log('Identity by public key hash result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityTokenBalances query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityTokenBalances');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityTokenBalances', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token balance data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenBalancesData = JSON.parse(result.result);
      expect(tokenBalancesData).toBeDefined();
      
      // Should be an array or object with token balance information
      expect(Array.isArray(tokenBalancesData) || typeof tokenBalancesData === 'object').toBe(true);
      
      console.log('Identity token balances result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentitiesTokenBalances query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesTokenBalances');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesTokenBalances', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token balance data for multiple identities (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenBalancesData = JSON.parse(result.result);
      expect(tokenBalancesData).toBeDefined();
      
      // Should be an array or object with token balance information for multiple identities
      expect(Array.isArray(tokenBalancesData) || typeof tokenBalancesData === 'object').toBe(true);
      
      console.log('Identities token balances result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityTokenInfos query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityTokenInfos');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityTokenInfos', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token info data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenInfoData = JSON.parse(result.result);
      expect(tokenInfoData).toBeDefined();
      
      // Should be an array or object with token information
      expect(Array.isArray(tokenInfoData) || typeof tokenInfoData === 'object').toBe(true);
      
      console.log('Identity token infos result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentitiesTokenInfos query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentitiesTokenInfos');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentitiesTokenInfos', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain token info data for multiple identities (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const tokenInfoData = JSON.parse(result.result);
      expect(tokenInfoData).toBeDefined();
      
      // Should be an array or object with token information for multiple identities
      expect(Array.isArray(tokenInfoData) || typeof tokenInfoData === 'object').toBe(true);
      
      console.log('Identities token infos result:', result.result.substring(0, 200) + '...');
    });
  });

  test.describe('Data Contract Queries', () => {
    test('should execute getDataContract query', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContract');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContract', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain contract data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const contractData = JSON.parse(result.result);
      expect(contractData).toBeDefined();
      
      console.log('Data contract result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getDataContracts query for multiple contracts', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContracts');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContracts', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      console.log('Multiple data contracts result:', result.result.substring(0, 200) + '...');
    });
  });

  test.describe('Document Queries', () => {
    test('should execute getDocuments query', async () => {
      await wasmSdkPage.setupQuery('document', 'getDocuments');
      
      const success = await parameterInjector.injectParameters('document', 'getDocuments', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain documents data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const documentsData = JSON.parse(result.result);
      expect(documentsData).toBeDefined();
      
      console.log('Documents query result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getDocument query for specific document', async () => {
      await wasmSdkPage.setupQuery('document', 'getDocument');
      
      const success = await parameterInjector.injectParameters('document', 'getDocument', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify query executed successfully
      expect(result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      // Verify the result is not an error message
      expect(result.hasError).toBe(false);
      expect(result.result).not.toContain('Error executing query');
      expect(result.result).not.toContain('not found');
      
      // Should contain document data (valid JSON)
      expect(() => JSON.parse(result.result)).not.toThrow();
      const documentData = JSON.parse(result.result);
      expect(documentData).toBeDefined();
      
      console.log('Single document result:', result.result.substring(0, 200) + '...');
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
      
      console.log('Status query result:', result.result.substring(0, 200) + '...');
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
      
      console.log('Current epoch result:', result.result.substring(0, 200) + '...');
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
      
      console.log('Total credits result:', result.result.substring(0, 200) + '...');
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
      
      console.log('Mainnet status result:', result.result.substring(0, 200) + '...');
    });
  });
});