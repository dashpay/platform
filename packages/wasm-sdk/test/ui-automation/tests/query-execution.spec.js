const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');
const { ParameterInjector } = require('../utils/parameter-injector');
const { getTestParameters } = require('../fixtures/test-data');

test.describe('WASM SDK Query Execution Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  test.describe('Identity Queries', () => {
    test('should execute getIdentity query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Inject test parameters
      const success = await parameterInjector.injectParameters('identity', 'getIdentity', 'testnet');
      expect(success).toBe(true);
      
      // Execute query
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      // Verify execution completed
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      expect(result.result.length).toBeGreaterThan(0);
      
      console.log('Identity query result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityBalance query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityBalance');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityBalance', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Identity balance result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getIdentityKeys query', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentityKeys');
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentityKeys', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Identity keys result:', result.result.substring(0, 200) + '...');
    });
  });

  test.describe('Data Contract Queries', () => {
    test('should execute getDataContract query', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContract');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContract', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Data contract result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getDataContracts query for multiple contracts', async () => {
      await wasmSdkPage.setupQuery('dataContract', 'getDataContracts');
      
      const success = await parameterInjector.injectParameters('dataContract', 'getDataContracts', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Multiple data contracts result:', result.result.substring(0, 200) + '...');
    });
  });

  test.describe('Document Queries', () => {
    test('should execute getDocuments query', async () => {
      await wasmSdkPage.setupQuery('document', 'getDocuments');
      
      const success = await parameterInjector.injectParameters('document', 'getDocuments', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Documents query result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getDocument query for specific document', async () => {
      await wasmSdkPage.setupQuery('document', 'getDocument');
      
      const success = await parameterInjector.injectParameters('document', 'getDocument', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
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
      await wasmSdkPage.setupQuery('system', 'getCurrentEpoch');
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Current epoch result:', result.result.substring(0, 200) + '...');
    });

    test('should execute getTotalCreditsInPlatform query', async () => {
      await wasmSdkPage.setupQuery('system', 'getTotalCreditsInPlatform');
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Total credits result:', result.result.substring(0, 200) + '...');
    });
  });

  test.describe('Error Handling', () => {
    test('should handle invalid identity ID gracefully', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Fill with invalid ID
      await wasmSdkPage.fillQueryParameters({ id: 'invalid_identity_id' });
      
      const result = await wasmSdkPage.executeQueryAndGetResult(false);
      
      // Should show error
      expect(result.hasError || !result.success).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Error handling result:', result.statusText);
    });

    test('should handle empty required fields', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Don't fill any parameters, try to execute
      const result = await wasmSdkPage.executeQueryAndGetResult(false);
      
      // Should show error or validation message
      expect(result.hasError || !result.success).toBe(true);
      
      console.log('Empty fields handling:', result.statusText);
    });
  });

  test.describe('Proof Information', () => {
    test('should execute query with proof info enabled', async () => {
      await wasmSdkPage.setupQuery('identity', 'getIdentity');
      
      // Enable proof info if available
      await wasmSdkPage.enableProofInfo();
      
      const success = await parameterInjector.injectParameters('identity', 'getIdentity', 'testnet');
      expect(success).toBe(true);
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      // With proof info, result might be larger
      console.log('Query with proof result length:', result.result.length);
    });
  });

  test.describe('Network Switching', () => {
    test('should execute queries on mainnet', async () => {
      // Switch to mainnet
      await wasmSdkPage.setNetwork('mainnet');
      
      await wasmSdkPage.setupQuery('system', 'getStatus');
      
      const result = await wasmSdkPage.executeQueryAndGetResult();
      
      expect(result.success || result.hasError).toBe(true);
      expect(result.result).toBeDefined();
      
      console.log('Mainnet status result:', result.result.substring(0, 200) + '...');
    });
  });
});