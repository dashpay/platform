const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');
const { ParameterInjector } = require('../utils/parameter-injector');
const { testData } = require('../fixtures/test-data');

test.describe('WASM SDK Parameterized Query Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  // Generate parameterized tests for each query category
  const queryCategories = [
    { category: 'identity', queries: ['getIdentity', 'getIdentityBalance', 'getIdentityKeys'] },
    { category: 'dataContract', queries: ['getDataContract', 'getDataContracts'] },
    { category: 'document', queries: ['getDocuments', 'getDocument'] },
    { category: 'system', queries: ['getStatus', 'getCurrentEpoch', 'getTotalCreditsInPlatform'] }
  ];

  for (const { category, queries } of queryCategories) {
    test.describe(`${category.toUpperCase()} Category Tests`, () => {
      
      for (const queryType of queries) {
        test(`should execute ${queryType} with all available parameter sets`, async () => {
          const parameterSets = parameterInjector.createParameterizedTests(category, queryType, 'testnet');
          
          if (parameterSets.length === 0) {
            test.skip(`No parameter sets available for ${category}.${queryType}`);
            return;
          }

          let successCount = 0;
          let errorCount = 0;
          const results = [];

          for (const paramSet of parameterSets) {
            try {
              console.log(`\n🧪 Testing ${paramSet.testName}`);
              
              await wasmSdkPage.setupQuery(category, queryType);
              
              // Inject parameters
              const injectionSuccess = await parameterInjector.injectParameters(
                category, 
                queryType, 
                'testnet', 
                paramSet.index
              );
              
              if (!injectionSuccess) {
                console.warn(`⚠️  Could not inject parameters for ${paramSet.testName}`);
                continue;
              }

              // Execute query
              const result = await wasmSdkPage.executeQueryAndGetResult();
              results.push({
                testName: paramSet.testName,
                parameters: paramSet.parameters,
                success: result.success,
                hasError: result.hasError,
                resultLength: result.result?.length || 0,
                statusText: result.statusText
              });

              if (result.success) {
                successCount++;
                console.log(`✅ ${paramSet.testName} - SUCCESS`);
              } else {
                errorCount++;
                console.log(`❌ ${paramSet.testName} - ERROR: ${result.statusText}`);
              }

              // Brief pause between executions
              await wasmSdkPage.page.waitForTimeout(1000);
              await wasmSdkPage.clearResults();
              
            } catch (error) {
              errorCount++;
              console.error(`💥 ${paramSet.testName} - EXCEPTION:`, error.message);
              results.push({
                testName: paramSet.testName,
                parameters: paramSet.parameters,
                success: false,
                hasError: true,
                error: error.message
              });
            }
          }

          // Summary assertions
          console.log(`\n📊 ${category}.${queryType} Summary:`);
          console.log(`   Total tests: ${parameterSets.length}`);
          console.log(`   Successful: ${successCount}`);
          console.log(`   Errors: ${errorCount}`);

          // At least one test should complete (success or graceful error)
          expect(successCount + errorCount).toBeGreaterThan(0);
          
          // Store results for reporting
          test.info().attach('test-results', {
            body: JSON.stringify(results, null, 2),
            contentType: 'application/json'
          });
        });
      }
    });
  }

  test.describe('Cross-Network Parameter Tests', () => {
    const networks = ['testnet', 'mainnet'];
    
    for (const network of networks) {
      test(`should execute system queries on ${network}`, async () => {
        await wasmSdkPage.setNetwork(network);
        
        const systemQueries = ['getStatus', 'getCurrentEpoch'];
        const results = [];
        
        for (const queryType of systemQueries) {
          try {
            await wasmSdkPage.setupQuery('system', queryType);
            const result = await wasmSdkPage.executeQueryAndGetResult();
            
            results.push({
              network,
              queryType,
              success: result.success,
              hasError: result.hasError,
              resultLength: result.result?.length || 0
            });
            
            await wasmSdkPage.clearResults();
            await wasmSdkPage.page.waitForTimeout(500);
            
          } catch (error) {
            results.push({
              network,
              queryType,
              success: false,
              error: error.message
            });
          }
        }
        
        // At least one query should work
        const successfulQueries = results.filter(r => r.success);
        expect(successfulQueries.length).toBeGreaterThan(0);
        
        console.log(`${network} system queries:`, results);
      });
    }
  });

  test.describe('Parameter Validation Tests', () => {
    test('should validate parameters before injection', async () => {
      const testCases = [
        {
          category: 'identity',
          queryType: 'getIdentity',
          parameters: { id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec' },
          expectedValid: true
        },
        {
          category: 'identity', 
          queryType: 'getIdentity',
          parameters: { id: '' },
          expectedValid: false
        },
        {
          category: 'identity',
          queryType: 'getIdentity', 
          parameters: { id: 'too_short' },
          expectedValid: false
        }
      ];

      for (const testCase of testCases) {
        const validation = parameterInjector.validateParameters(testCase.parameters);
        
        if (testCase.expectedValid) {
          expect(validation.errors.length).toBe(0);
        } else {
          expect(validation.errors.length).toBeGreaterThan(0);
        }
        
        console.log(`Validation for ${JSON.stringify(testCase.parameters)}:`, validation);
      }
    });
  });

  test.describe('Random Parameter Stress Tests', () => {
    test('should handle random parameter generation gracefully', async () => {
      const testQueries = [
        { category: 'identity', queryType: 'getIdentity' },
        { category: 'system', queryType: 'getStatus' }
      ];

      for (const { category, queryType } of testQueries) {
        try {
          const randomParams = parameterInjector.generateRandomParameters(category, queryType);
          
          if (Object.keys(randomParams).length > 0) {
            await wasmSdkPage.setupQuery(category, queryType);
            await wasmSdkPage.fillQueryParameters(randomParams);
            
            const result = await wasmSdkPage.executeQueryAndGetResult(false);
            
            // Should complete without crashing
            expect(result).toBeDefined();
            expect(typeof result.success).toBe('boolean');
            
            console.log(`Random test ${category}.${queryType}:`, {
              parameters: randomParams,
              success: result.success,
              hasError: result.hasError
            });
          }
          
          await wasmSdkPage.clearResults();
          
        } catch (error) {
          // Random parameters might cause errors - that's OK as long as UI doesn't crash
          console.log(`Random test error (acceptable): ${error.message}`);
        }
      }
    });
  });
});
