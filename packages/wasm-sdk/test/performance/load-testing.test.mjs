/**
 * Performance and Load Testing Suite for WASM SDK
 * Tests performance benchmarks, memory usage, and scalability
 */

import { jest } from '@jest/globals';

describe('WASM SDK Performance Tests', () => {
  let sdk;

  beforeAll(async () => {
    const wasmInitialized = await global.initializeWasm();
    if (!wasmInitialized) {
      throw new Error('Failed to initialize WASM - tests cannot proceed');
    }
  });

  beforeEach(async () => {
    sdk = await global.createTestSDK({
      network: 'testnet',
      proofs: false // Disable proofs for faster performance testing
    });
  });

  afterEach(async () => {
    if (sdk && sdk.destroy) {
      await sdk.destroy();
    }
    
    // Force garbage collection if available
    if (global.gc) {
      global.gc();
    }
  });

  describe('Operation Performance Benchmarks', () => {
    test('should meet mnemonic generation performance requirements', async () => {
      const iterations = 100;
      const maxTimePerOperation = 50; // 50ms max per operation
      
      const results = [];
      
      for (let i = 0; i < iterations; i++) {
        const performance = await global.measurePerformance(
          () => sdk.generateMnemonic(12),
          `mnemonic-generation-${i}`
        );
        
        results.push(performance.duration);
        expect(performance.duration).toCompleteWithinTime(maxTimePerOperation);
      }
      
      // Calculate statistics
      const avgTime = results.reduce((sum, time) => sum + time, 0) / results.length;
      const maxTime = Math.max(...results);
      const minTime = Math.min(...results);
      
      console.log(`Mnemonic Generation Performance:
        Average: ${avgTime.toFixed(2)}ms
        Min: ${minTime.toFixed(2)}ms
        Max: ${maxTime.toFixed(2)}ms
        Operations: ${iterations}`);
      
      expect(avgTime).toBeLessThan(maxTimePerOperation / 2); // Average should be well under max
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should meet key derivation performance requirements', async () => {
      const iterations = 50;
      const maxTimePerOperation = 100; // 100ms max per operation
      
      const mnemonic = await sdk.generateMnemonic(12);
      const seed = await sdk.generateSeedFromMnemonic(mnemonic);
      
      const results = [];
      
      for (let i = 0; i < iterations; i++) {
        const performance = await global.measurePerformance(
          () => sdk.deriveKeyFromSeed(seed, 'identity', i),
          `key-derivation-${i}`
        );
        
        results.push(performance.duration);
        expect(performance.duration).toCompleteWithinTime(maxTimePerOperation);
      }
      
      const avgTime = results.reduce((sum, time) => sum + time, 0) / results.length;
      
      console.log(`Key Derivation Performance:
        Average: ${avgTime.toFixed(2)}ms
        Operations: ${iterations}`);
      
      expect(avgTime).toBeLessThan(maxTimePerOperation);
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should meet network query performance requirements', async () => {
      const maxTimePerQuery = 5000; // 5 seconds max per network query
      
      const networkOperations = [
        {
          name: 'getStatus',
          fn: () => sdk.getStatus(),
          maxTime: 3000 // Network status should be faster
        },
        {
          name: 'getIdentity',
          fn: () => sdk.getIdentity(TEST_CONFIG.SAMPLE_IDENTITY_ID),
          maxTime: maxTimePerQuery
        }
      ];
      
      for (const operation of networkOperations) {
        const performance = await global.measurePerformance(
          operation.fn,
          operation.name
        );
        
        console.log(`${operation.name} took ${performance.duration.toFixed(2)}ms`);
        expect(performance.duration).toCompleteWithinTime(operation.maxTime);
      }
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle batch operations efficiently', async () => {
      const batchSize = 10;
      const maxTotalTime = 3000; // 3 seconds for batch of 10
      
      const startTime = performance.now();
      
      const mnemonics = [];
      for (let i = 0; i < batchSize; i++) {
        mnemonics.push(await sdk.generateMnemonic(12));
      }
      
      const totalTime = performance.now() - startTime;
      const avgTimePerOperation = totalTime / batchSize;
      
      console.log(`Batch Operations Performance:
        Total Time: ${totalTime.toFixed(2)}ms
        Average per Operation: ${avgTimePerOperation.toFixed(2)}ms
        Batch Size: ${batchSize}`);
      
      expect(totalTime).toBeLessThan(maxTotalTime);
      expect(avgTimePerOperation).toBeLessThan(maxTotalTime / batchSize);
      expect(mnemonics).toHaveLength(batchSize);
      
      // All mnemonics should be unique
      const uniqueMnemonics = new Set(mnemonics);
      expect(uniqueMnemonics.size).toBe(batchSize);
    }, TEST_CONFIG.SLOW_TIMEOUT);
  });

  describe('Memory Usage and Leaks', () => {
    test('should not leak memory during repeated operations', async () => {
      const iterations = 100;
      const memoryMeasurements = [];
      
      // Initial memory measurement
      if (global.gc) global.gc();
      const initialMemory = process.memoryUsage();
      
      // Perform repeated operations
      for (let i = 0; i < iterations; i++) {
        await sdk.generateMnemonic(12);
        
        // Measure memory every 20 iterations
        if (i % 20 === 0) {
          if (global.gc) global.gc();
          const currentMemory = process.memoryUsage();
          memoryMeasurements.push({
            iteration: i,
            heapUsed: currentMemory.heapUsed,
            heapTotal: currentMemory.heapTotal,
            rss: currentMemory.rss
          });
        }
      }
      
      // Final memory measurement
      if (global.gc) global.gc();
      const finalMemory = process.memoryUsage();
      
      console.log(`Memory Usage Test:
        Initial Heap: ${(initialMemory.heapUsed / 1024 / 1024).toFixed(2)}MB
        Final Heap: ${(finalMemory.heapUsed / 1024 / 1024).toFixed(2)}MB
        Difference: ${((finalMemory.heapUsed - initialMemory.heapUsed) / 1024 / 1024).toFixed(2)}MB
        Iterations: ${iterations}`);
      
      // Memory growth should be reasonable (less than 50MB for this test)
      const memoryGrowth = finalMemory.heapUsed - initialMemory.heapUsed;
      expect(memoryGrowth).toBeLessThan(50 * 1024 * 1024); // 50MB limit
      
      // Check for memory leak patterns
      if (memoryMeasurements.length >= 3) {
        const firstMeasurement = memoryMeasurements[0];
        const lastMeasurement = memoryMeasurements[memoryMeasurements.length - 1];
        const growthRate = (lastMeasurement.heapUsed - firstMeasurement.heapUsed) / firstMeasurement.iteration;
        
        // Growth rate should be reasonable (less than 1MB per 20 iterations)
        expect(growthRate).toBeLessThan(1024 * 1024);
      }
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle multiple SDK instances efficiently', async () => {
      const instanceCount = 5;
      const instances = [];
      
      const startTime = performance.now();
      if (global.gc) global.gc();
      const initialMemory = process.memoryUsage();
      
      // Create multiple instances
      for (let i = 0; i < instanceCount; i++) {
        const instance = await global.createTestSDK({
          network: 'testnet',
          proofs: false
        });
        instances.push(instance);
      }
      
      const creationTime = performance.now() - startTime;
      if (global.gc) global.gc();
      const peakMemory = process.memoryUsage();
      
      // Test operations with all instances
      const operationPromises = instances.map(async (instance, index) => {
        return await instance.generateMnemonic(12);
      });
      
      const results = await Promise.all(operationPromises);
      expect(results).toHaveLength(instanceCount);
      
      // Cleanup all instances
      await Promise.all(instances.map(instance => instance.destroy()));
      
      if (global.gc) global.gc();
      const finalMemory = process.memoryUsage();
      
      console.log(`Multiple Instances Test:
        Instance Count: ${instanceCount}
        Creation Time: ${creationTime.toFixed(2)}ms
        Initial Memory: ${(initialMemory.heapUsed / 1024 / 1024).toFixed(2)}MB
        Peak Memory: ${(peakMemory.heapUsed / 1024 / 1024).toFixed(2)}MB
        Final Memory: ${(finalMemory.heapUsed / 1024 / 1024).toFixed(2)}MB`);
      
      // Memory should return close to initial after cleanup
      const memoryIncrease = finalMemory.heapUsed - initialMemory.heapUsed;
      expect(memoryIncrease).toBeLessThan(20 * 1024 * 1024); // 20MB tolerance
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle large data structures efficiently', async () => {
      // Test handling of large arrays/objects
      const largeArraySize = 1000;
      
      const startTime = performance.now();
      if (global.gc) global.gc();
      const initialMemory = process.memoryUsage();
      
      // Generate large amount of data
      const mnemonics = [];
      for (let i = 0; i < largeArraySize; i++) {
        mnemonics.push(await sdk.generateMnemonic(24)); // Use 24-word mnemonics for more data
      }
      
      const generationTime = performance.now() - startTime;
      if (global.gc) global.gc();
      const peakMemory = process.memoryUsage();
      
      // Verify data integrity
      expect(mnemonics).toHaveLength(largeArraySize);
      mnemonics.forEach(mnemonic => {
        expect(mnemonic.split(' ')).toHaveLength(24);
      });
      
      // Clear references and measure cleanup
      mnemonics.length = 0;
      if (global.gc) global.gc();
      const finalMemory = process.memoryUsage();
      
      console.log(`Large Data Structure Test:
        Array Size: ${largeArraySize}
        Generation Time: ${generationTime.toFixed(2)}ms
        Average per Item: ${(generationTime / largeArraySize).toFixed(2)}ms
        Initial Memory: ${(initialMemory.heapUsed / 1024 / 1024).toFixed(2)}MB
        Peak Memory: ${(peakMemory.heapUsed / 1024 / 1024).toFixed(2)}MB
        Final Memory: ${(finalMemory.heapUsed / 1024 / 1024).toFixed(2)}MB`);
      
      // Generation time should be reasonable
      expect(generationTime).toBeLessThan(largeArraySize * 100); // 100ms per item max
      
      // Memory should be efficiently managed
      const peakIncrease = peakMemory.heapUsed - initialMemory.heapUsed;
      expect(peakIncrease).toBeLessThan(200 * 1024 * 1024); // 200MB max for this test
    }, TEST_CONFIG.SLOW_TIMEOUT);
  });

  describe('Concurrent Operations Stress Test', () => {
    test('should handle high concurrency without degradation', async () => {
      const concurrentOperations = 20;
      const operationsPerConcurrent = 5;
      
      const startTime = performance.now();
      
      // Create multiple concurrent operation chains
      const concurrentPromises = [];
      
      for (let i = 0; i < concurrentOperations; i++) {
        const operationChain = async (chainId) => {
          const results = [];
          
          for (let j = 0; j < operationsPerConcurrent; j++) {
            const operationStart = performance.now();
            const mnemonic = await sdk.generateMnemonic(12);
            const operationTime = performance.now() - operationStart;
            
            results.push({
              chainId,
              operationId: j,
              mnemonic,
              duration: operationTime
            });
          }
          
          return results;
        };
        
        concurrentPromises.push(operationChain(i));
      }
      
      const allResults = await Promise.all(concurrentPromises);
      const totalTime = performance.now() - startTime;
      
      // Flatten results for analysis
      const flatResults = allResults.flat();
      const totalOperations = concurrentOperations * operationsPerConcurrent;
      
      expect(flatResults).toHaveLength(totalOperations);
      
      // Calculate performance statistics
      const operationTimes = flatResults.map(r => r.duration);
      const avgOperationTime = operationTimes.reduce((sum, time) => sum + time, 0) / operationTimes.length;
      const maxOperationTime = Math.max(...operationTimes);
      const minOperationTime = Math.min(...operationTimes);
      
      console.log(`Concurrency Stress Test:
        Concurrent Chains: ${concurrentOperations}
        Operations per Chain: ${operationsPerConcurrent}
        Total Operations: ${totalOperations}
        Total Time: ${totalTime.toFixed(2)}ms
        Average Operation Time: ${avgOperationTime.toFixed(2)}ms
        Min Operation Time: ${minOperationTime.toFixed(2)}ms
        Max Operation Time: ${maxOperationTime.toFixed(2)}ms
        Operations per Second: ${(totalOperations / (totalTime / 1000)).toFixed(2)}`);
      
      // Performance requirements
      expect(avgOperationTime).toBeLessThan(100); // Average should be under 100ms
      expect(maxOperationTime).toBeLessThan(500); // Max should be under 500ms
      
      // Verify all operations succeeded
      flatResults.forEach(result => {
        expect(typeof result.mnemonic).toBe('string');
        expect(result.mnemonic.split(' ')).toHaveLength(12);
      });
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle mixed operation types concurrently', async () => {
      const operationTypes = [
        {
          name: 'generateMnemonic',
          fn: () => sdk.generateMnemonic(12),
          weight: 0.4 // 40% of operations
        },
        {
          name: 'validateMnemonic',
          fn: async () => {
            const mnemonic = await sdk.generateMnemonic(12);
            return await sdk.validateMnemonic(mnemonic);
          },
          weight: 0.3 // 30% of operations
        },
        {
          name: 'getStatus',
          fn: () => sdk.getStatus(),
          weight: 0.3 // 30% of operations
        }
      ];
      
      const totalOperations = 50;
      const operations = [];
      
      // Create mixed operations based on weights
      for (let i = 0; i < totalOperations; i++) {
        const random = Math.random();
        let cumulativeWeight = 0;
        
        for (const opType of operationTypes) {
          cumulativeWeight += opType.weight;
          if (random <= cumulativeWeight) {
            operations.push({
              id: i,
              type: opType.name,
              fn: opType.fn
            });
            break;
          }
        }
      }
      
      const startTime = performance.now();
      
      // Execute all operations concurrently
      const results = await Promise.allSettled(
        operations.map(async (op) => {
          const opStart = performance.now();
          const result = await op.fn();
          const duration = performance.now() - opStart;
          
          return {
            id: op.id,
            type: op.type,
            result,
            duration,
            success: true
          };
        })
      );
      
      const totalTime = performance.now() - startTime;
      
      // Analyze results by operation type
      const typeStats = {};
      results.forEach(result => {
        if (result.status === 'fulfilled') {
          const { type, duration } = result.value;
          if (!typeStats[type]) {
            typeStats[type] = { count: 0, totalTime: 0, times: [] };
          }
          typeStats[type].count++;
          typeStats[type].totalTime += duration;
          typeStats[type].times.push(duration);
        }
      });
      
      console.log(`Mixed Operations Concurrency Test:
        Total Operations: ${totalOperations}
        Total Time: ${totalTime.toFixed(2)}ms
        Successful Operations: ${results.filter(r => r.status === 'fulfilled').length}`);
      
      Object.entries(typeStats).forEach(([type, stats]) => {
        const avgTime = stats.totalTime / stats.count;
        console.log(`  ${type}: ${stats.count} ops, avg ${avgTime.toFixed(2)}ms`);
      });
      
      // Most operations should succeed
      const successCount = results.filter(r => r.status === 'fulfilled').length;
      expect(successCount).toBeGreaterThan(totalOperations * 0.8); // At least 80% success rate
    }, TEST_CONFIG.SLOW_TIMEOUT);
  });

  describe('Resource Cleanup and Recovery', () => {
    test('should recover from rapid create/destroy cycles', async () => {
      const cycles = 10;
      const maxCycleTime = 2000; // 2 seconds per cycle
      
      const cycleTimes = [];
      
      for (let i = 0; i < cycles; i++) {
        const cycleStart = performance.now();
        
        // Create SDK instance
        const testSdk = await global.createTestSDK({
          network: 'testnet',
          proofs: false
        });
        
        // Perform some operations
        await testSdk.generateMnemonic(12);
        const status = await testSdk.getStatus();
        expect(status).toBeDefined();
        
        // Destroy instance
        await testSdk.destroy();
        
        const cycleTime = performance.now() - cycleStart;
        cycleTimes.push(cycleTime);
        
        expect(cycleTime).toCompleteWithinTime(maxCycleTime);
        
        console.log(`Cycle ${i + 1}: ${cycleTime.toFixed(2)}ms`);
      }
      
      const avgCycleTime = cycleTimes.reduce((sum, time) => sum + time, 0) / cycleTimes.length;
      
      console.log(`Rapid Create/Destroy Test:
        Cycles: ${cycles}
        Average Cycle Time: ${avgCycleTime.toFixed(2)}ms`);
      
      expect(avgCycleTime).toBeLessThan(maxCycleTime);
    }, TEST_CONFIG.SLOW_TIMEOUT);

    test('should handle resource exhaustion gracefully', async () => {
      const maxInstances = 10;
      const instances = [];
      
      try {
        // Create many instances to test resource limits
        for (let i = 0; i < maxInstances; i++) {
          const instance = await global.createTestSDK({
            network: 'testnet',
            proofs: false
          });
          instances.push(instance);
          
          // Test instance is working
          const mnemonic = await instance.generateMnemonic(12);
          expect(typeof mnemonic).toBe('string');
        }
        
        console.log(`Successfully created ${instances.length} SDK instances`);
        
        // All instances should be functional
        const testPromises = instances.map(async (instance, index) => {
          const mnemonic = await instance.generateMnemonic(12);
          return { index, success: true, mnemonic };
        });
        
        const testResults = await Promise.allSettled(testPromises);
        const successCount = testResults.filter(r => r.status === 'fulfilled').length;
        
        console.log(`${successCount}/${instances.length} instances are functional`);
        
        // Most instances should work (allow for some resource pressure failures)
        expect(successCount).toBeGreaterThan(instances.length * 0.7); // 70% success rate minimum
        
      } finally {
        // Cleanup all instances
        const cleanupPromises = instances.map(instance => 
          instance.destroy().catch(error => {
            console.warn('Failed to destroy instance:', error.message);
          })
        );
        
        await Promise.allSettled(cleanupPromises);
        console.log('All instances cleaned up');
      }
    }, TEST_CONFIG.SLOW_TIMEOUT);
  });

  describe('Performance Regression Tests', () => {
    test('should maintain baseline performance over time', async () => {
      // Define performance baselines (these could be loaded from a file)
      const performanceBaselines = {
        mnemonicGeneration: 50, // 50ms baseline
        keyDerivation: 100,     // 100ms baseline
        statusQuery: 3000,      // 3s baseline
        identityQuery: 5000     // 5s baseline
      };
      
      const tests = [
        {
          name: 'mnemonicGeneration',
          fn: () => sdk.generateMnemonic(12),
          baseline: performanceBaselines.mnemonicGeneration
        },
        {
          name: 'keyDerivation',
          fn: async () => {
            const mnemonic = await sdk.generateMnemonic(12);
            const seed = await sdk.generateSeedFromMnemonic(mnemonic);
            return await sdk.deriveKeyFromSeed(seed, 'identity', 0);
          },
          baseline: performanceBaselines.keyDerivation
        },
        {
          name: 'statusQuery',
          fn: () => sdk.getStatus(),
          baseline: performanceBaselines.statusQuery
        }
      ];
      
      const results = [];
      
      for (const test of tests) {
        const iterations = test.name.includes('Query') ? 3 : 10; // Fewer iterations for network calls
        const times = [];
        
        for (let i = 0; i < iterations; i++) {
          const performance = await global.measurePerformance(test.fn, test.name);
          times.push(performance.duration);
        }
        
        const avgTime = times.reduce((sum, time) => sum + time, 0) / times.length;
        const maxTime = Math.max(...times);
        
        results.push({
          name: test.name,
          avgTime,
          maxTime,
          baseline: test.baseline,
          withinBaseline: avgTime <= test.baseline,
          regressionRatio: avgTime / test.baseline
        });
        
        console.log(`${test.name}:
          Average: ${avgTime.toFixed(2)}ms
          Max: ${maxTime.toFixed(2)}ms
          Baseline: ${test.baseline}ms
          Regression Ratio: ${(avgTime / test.baseline).toFixed(2)}x`);
      }
      
      // Check for performance regressions
      const regressions = results.filter(r => !r.withinBaseline);
      
      if (regressions.length > 0) {
        console.warn('Performance regressions detected:');
        regressions.forEach(r => {
          console.warn(`  ${r.name}: ${r.avgTime.toFixed(2)}ms (${r.regressionRatio.toFixed(2)}x baseline)`);
        });
      }
      
      // Allow up to 2x baseline for now (adjust as needed)
      results.forEach(result => {
        expect(result.regressionRatio).toBeLessThan(2.0);
      });
      
    }, TEST_CONFIG.SLOW_TIMEOUT);
  });
});