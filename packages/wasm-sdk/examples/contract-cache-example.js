// Example of using the enhanced contract cache in the WASM SDK

import init, {
  // Contract cache
  ContractCacheConfig,
  ContractCache,
  createContractCache,
  
  // General cache manager
  WasmCacheManager,
  integrateContractCache,
  
  // Data contract operations
  create_data_contract,
  fetch_data_contract,
  
  // SDK
  WasmSdk,
} from '../pkg/wasm_sdk.js';

// Initialize WASM
await init();

// Example 1: Basic contract caching
async function basicContractCaching() {
  console.log('=== Basic Contract Caching Example ===');
  
  // Create cache with default config
  const cache = createContractCache();
  
  // Simulate a contract
  const contractDefinition = {
    id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq',
    version: 1,
    ownerId: 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF',
    documentSchemas: {
      profile: {
        type: 'object',
        properties: {
          username: { type: 'string', minLength: 3, maxLength: 20 },
          displayName: { type: 'string' },
          avatar: { type: 'string', contentMediaType: 'image/*' }
        },
        required: ['username'],
        additionalProperties: false
      },
      message: {
        type: 'object',
        properties: {
          content: { type: 'string', maxLength: 280 },
          timestamp: { type: 'integer' },
          author: { type: 'string' }
        },
        required: ['content', 'timestamp', 'author'],
        additionalProperties: false
      }
    }
  };
  
  // Create contract bytes (in real usage, this would come from the network)
  const contractBytes = new TextEncoder().encode(JSON.stringify(contractDefinition));
  
  // Cache the contract
  const contractId = cache.cacheContract(contractBytes);
  console.log('Cached contract:', contractId);
  
  // Check if cached
  console.log('Is cached:', cache.isContractCached(contractId));
  
  // Get from cache
  const cachedBytes = cache.getCachedContract(contractId);
  if (cachedBytes) {
    console.log('Retrieved from cache, size:', cachedBytes.length, 'bytes');
  }
  
  // Get metadata
  const metadata = cache.getContractMetadata(contractId);
  console.log('Contract metadata:', metadata);
  
  return cache;
}

// Example 2: Advanced cache configuration
async function advancedCacheConfig() {
  console.log('\n=== Advanced Cache Configuration Example ===');
  
  // Create custom configuration
  const config = new ContractCacheConfig();
  config.setMaxContracts(50);
  config.setTtl(1800000); // 30 minutes
  config.setCacheHistory(true);
  config.setMaxVersionsPerContract(3);
  config.setEnablePreloading(true);
  
  // Create cache with custom config
  const cache = createContractCache(config);
  
  // Simulate caching multiple contract versions
  const baseContract = {
    id: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S3Qdq',
    ownerId: 'FKEPbQ7HyHiPYmJD4rKugXPvDqUBKcCRZGnkm6mEthQF',
  };
  
  // Cache version 1
  const v1 = { ...baseContract, version: 1, schema: { profile: {} } };
  cache.cacheContract(new TextEncoder().encode(JSON.stringify(v1)));
  
  // Cache version 2 (with updates)
  const v2 = { ...baseContract, version: 2, schema: { profile: {}, message: {} } };
  cache.cacheContract(new TextEncoder().encode(JSON.stringify(v2)));
  
  // Get cache statistics
  const stats = cache.getCacheStats();
  console.log('Cache statistics:', stats);
  
  return cache;
}

// Example 3: Cache management and eviction
async function cacheManagement() {
  console.log('\n=== Cache Management Example ===');
  
  const config = new ContractCacheConfig();
  config.setMaxContracts(5); // Small cache for demo
  config.setTtl(5000); // 5 seconds TTL for demo
  
  const cache = createContractCache(config);
  
  // Fill cache to capacity
  for (let i = 0; i < 7; i++) {
    const contract = {
      id: `contract${i}`,
      version: 1,
      data: `Contract data ${i}`
    };
    cache.cacheContract(new TextEncoder().encode(JSON.stringify(contract)));
    
    // Simulate access patterns
    if (i % 2 === 0) {
      // Access even contracts more frequently
      cache.getCachedContract(`contract${i}`);
      cache.getCachedContract(`contract${i}`);
    }
  }
  
  // Check what's in cache (should be last 5 due to LRU eviction)
  console.log('Cached contracts:', cache.getCachedContractIds());
  
  // Wait for TTL expiration
  console.log('Waiting for TTL expiration...');
  await new Promise(resolve => setTimeout(resolve, 6000));
  
  // Clean up expired entries
  const removed = cache.cleanupExpired();
  console.log('Removed expired entries:', removed);
  
  // Check remaining
  console.log('Remaining contracts:', cache.getCachedContractIds());
  
  return cache;
}

// Example 4: Access patterns and preloading
async function accessPatternsExample() {
  console.log('\n=== Access Patterns and Preloading Example ===');
  
  const cache = createContractCache();
  
  // Simulate realistic access patterns
  const contracts = [
    'dpns-contract',
    'dashpay-contract',
    'feature-flags-contract',
    'masternode-reward-shares-contract'
  ];
  
  // Cache contracts
  for (const contractId of contracts) {
    const contract = {
      id: contractId,
      version: 1,
      schema: {}
    };
    cache.cacheContract(new TextEncoder().encode(JSON.stringify(contract)));
  }
  
  // Simulate access patterns
  // DPNS contract accessed frequently
  for (let i = 0; i < 10; i++) {
    cache.getCachedContract('dpns-contract');
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  
  // DashPay contract accessed moderately
  for (let i = 0; i < 5; i++) {
    cache.getCachedContract('dashpay-contract');
    await new Promise(resolve => setTimeout(resolve, 200));
  }
  
  // Feature flags accessed rarely
  cache.getCachedContract('feature-flags-contract');
  
  // Get preload suggestions based on access patterns
  const suggestions = cache.getPreloadSuggestions();
  console.log('Preload suggestions:', suggestions);
  
  // Get cache stats to see access counts
  const stats = cache.getCacheStats();
  console.log('Most accessed contracts:', stats.mostAccessed);
  
  return cache;
}

// Example 5: Integration with general cache manager
async function integratedCacheExample() {
  console.log('\n=== Integrated Cache Example ===');
  
  // Create both caches
  const generalCache = new WasmCacheManager();
  const contractCache = createContractCache();
  
  // Integrate them
  integrateContractCache(generalCache, contractCache);
  
  // Use contract cache for contracts
  const contract = {
    id: 'test-contract',
    version: 1,
    schema: { document: {} }
  };
  contractCache.cacheContract(new TextEncoder().encode(JSON.stringify(contract)));
  
  // Use general cache for other data
  generalCache.cacheIdentity(
    'identity123',
    new TextEncoder().encode(JSON.stringify({ id: 'identity123', balance: 1000 }))
  );
  
  // Get stats from both
  console.log('Contract cache stats:', contractCache.getCacheStats());
  console.log('General cache stats:', generalCache.getStats());
  
  return { generalCache, contractCache };
}

// Example 6: Performance testing
async function performanceTest() {
  console.log('\n=== Cache Performance Test ===');
  
  const cache = createContractCache();
  const iterations = 1000;
  
  // Create test contract
  const testContract = {
    id: 'perf-test-contract',
    version: 1,
    schema: {
      testDoc: {
        type: 'object',
        properties: {
          field1: { type: 'string' },
          field2: { type: 'integer' },
          field3: { type: 'boolean' }
        }
      }
    }
  };
  const contractBytes = new TextEncoder().encode(JSON.stringify(testContract));
  
  // Test cache write performance
  const writeStart = performance.now();
  for (let i = 0; i < iterations; i++) {
    const contract = { ...testContract, id: `contract-${i}` };
    cache.cacheContract(new TextEncoder().encode(JSON.stringify(contract)));
  }
  const writeEnd = performance.now();
  console.log(`Cache write: ${(writeEnd - writeStart) / iterations}ms per contract`);
  
  // Test cache read performance
  const readStart = performance.now();
  for (let i = 0; i < iterations; i++) {
    cache.getCachedContract(`contract-${i % 100}`); // Read first 100 contracts
  }
  const readEnd = performance.now();
  console.log(`Cache read: ${(readEnd - readStart) / iterations}ms per contract`);
  
  // Test metadata access
  const metaStart = performance.now();
  for (let i = 0; i < iterations; i++) {
    cache.getContractMetadata(`contract-${i % 100}`);
  }
  const metaEnd = performance.now();
  console.log(`Metadata access: ${(metaEnd - metaStart) / iterations}ms per contract`);
  
  // Final stats
  const stats = cache.getCacheStats();
  console.log('Final cache stats:', stats);
}

// Example 7: Real-world usage with SDK
async function realWorldExample() {
  console.log('\n=== Real-World Cache Usage Example ===');
  
  // Initialize SDK
  const sdk = new WasmSdk();
  
  // Create contract cache
  const contractCache = createContractCache();
  
  // Function to fetch contract with caching
  async function fetchContractWithCache(contractId) {
    // Check cache first
    const cachedBytes = contractCache.getCachedContract(contractId);
    if (cachedBytes) {
      console.log(`Contract ${contractId} found in cache`);
      return new TextDecoder().decode(cachedBytes);
    }
    
    console.log(`Contract ${contractId} not in cache, fetching...`);
    
    // Simulate network fetch
    // In real usage, this would call fetch_data_contract
    const contract = {
      id: contractId,
      version: 1,
      schema: { /* ... */ }
    };
    
    const contractBytes = new TextEncoder().encode(JSON.stringify(contract));
    
    // Cache for next time
    contractCache.cacheContract(contractBytes);
    
    return contract;
  }
  
  // Use the cached fetch function
  const contract1 = await fetchContractWithCache('dpns-contract');
  console.log('Fetched contract 1');
  
  // Second fetch should hit cache
  const contract2 = await fetchContractWithCache('dpns-contract');
  console.log('Fetched contract 2 (from cache)');
  
  // Check cache efficiency
  const metadata = contractCache.getContractMetadata('dpns-contract');
  console.log('Contract access count:', metadata.accessCount);
}

// Run all examples
(async () => {
  try {
    await basicContractCaching();
    await advancedCacheConfig();
    await cacheManagement();
    await accessPatternsExample();
    await integratedCacheExample();
    await performanceTest();
    await realWorldExample();
    
    console.log('\n✅ All contract cache examples completed successfully!');
  } catch (error) {
    console.error('❌ Error in contract cache examples:', error);
  }
})();