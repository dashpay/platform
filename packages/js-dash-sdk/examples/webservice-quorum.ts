/**
 * Example: Using web service provider for quorum key management
 */

import { createSDK } from '../src';
import { WebServiceProvider } from '../src/providers';

async function quorumKeysExample() {
  console.log('Web Service Quorum Keys Example');
  console.log('===============================\n');

  // Create web service provider
  const provider = new WebServiceProvider({
    network: 'testnet',
    cacheDuration: 60000, // Cache quorum keys for 1 minute
    retryAttempts: 3,
    timeout: 30000
  });

  console.log('Checking web service availability...');
  const isAvailable = await provider.isAvailable();
  console.log(`Web service available: ${isAvailable}`);

  if (!isAvailable) {
    console.error('Web service is not available. The service may not be running yet.');
    console.log('Using testnet endpoint: https://quorum.testnet.networks.dash.org');
    return;
  }

  // Create SDK with web service provider
  const sdk = createSDK({
    network: 'testnet',
    contextProvider: provider
  });

  await sdk.initialize();
  console.log('✓ SDK initialized with web service provider\n');

  // Get platform status
  console.log('Platform Status:');
  const [height, time, coreHeight, version] = await Promise.all([
    provider.getLatestPlatformBlockHeight(),
    provider.getLatestPlatformBlockTime(),
    provider.getLatestPlatformCoreChainLockedHeight(),
    provider.getLatestPlatformVersion()
  ]);

  console.log(`  Block Height: ${height}`);
  console.log(`  Block Time: ${new Date(time).toISOString()}`);
  console.log(`  Core Chain Locked Height: ${coreHeight}`);
  console.log(`  Platform Version: ${version}`);

  // Fetch quorum keys
  console.log('\n\nFetching Quorum Keys...');
  try {
    const quorumKeys = await provider.getQuorumKeys();
    console.log(`Total quorums: ${quorumKeys.size}`);

    // Display quorum information
    console.log('\nQuorum Details:');
    let displayCount = 0;
    for (const [hash, info] of quorumKeys) {
      if (displayCount >= 5) {
        console.log(`  ... and ${quorumKeys.size - 5} more quorums`);
        break;
      }

      console.log(`\n  Quorum Hash: ${hash}`);
      console.log(`  Public Key: ${info.quorumPublicKey.publicKey.substring(0, 32)}...`);
      console.log(`  Type: ${info.quorumPublicKey.type}`);
      console.log(`  Version: ${info.quorumPublicKey.version}`);
      console.log(`  Active: ${info.isActive}`);
      
      displayCount++;
    }

    // Get specific quorum
    const firstQuorumHash = Array.from(quorumKeys.keys())[0];
    if (firstQuorumHash) {
      console.log('\n\nFetching specific quorum...');
      const specificQuorum = await provider.getQuorum(firstQuorumHash);
      if (specificQuorum) {
        console.log(`Found quorum: ${specificQuorum.quorumHash}`);
      }
    }

    // Get active quorums
    console.log('\n\nActive Quorums:');
    const activeQuorums = await provider.getActiveQuorums();
    console.log(`Active quorum count: ${activeQuorums.length}`);

  } catch (error: any) {
    console.error('\nError fetching quorum keys:', error.message);
    console.log('\nNote: The quorum service endpoint may not be active yet.');
    console.log('Expected endpoints:');
    console.log('  - Mainnet: https://quorum.networks.dash.org');
    console.log('  - Testnet: https://quorum.testnet.networks.dash.org');
  }

  // Example: Using quorum keys for verification
  console.log('\n\nExample Use Case: Quorum Key Verification');
  console.log('=========================================');
  
  console.log('\nIn a real application, quorum keys would be used for:');
  console.log('1. Verifying platform state transitions');
  console.log('2. Validating masternode signatures');
  console.log('3. Checking consensus on platform data');
  console.log('4. Verifying instant send locks');
  
  // Demonstrate caching behavior
  console.log('\n\nDemonstrating Cache Behavior:');
  console.log('First call (fetches from network):');
  let start = Date.now();
  await provider.getQuorumKeys();
  console.log(`  Time: ${Date.now() - start}ms`);
  
  console.log('Second call (uses cache):');
  start = Date.now();
  await provider.getQuorumKeys();
  console.log(`  Time: ${Date.now() - start}ms`);
  
  console.log('\nCache significantly improves performance for repeated calls.');
}

// Example: Monitoring quorum changes
async function monitorQuorums() {
  console.log('\n\nQuorum Monitoring Example');
  console.log('=========================\n');

  const provider = new WebServiceProvider({
    network: 'testnet',
    cacheDuration: 5000 // Short cache for monitoring
  });

  console.log('Monitoring quorum changes (press Ctrl+C to stop)...\n');

  let previousQuorumCount = 0;
  let previousHashes = new Set<string>();

  const checkQuorums = async () => {
    try {
      const quorums = await provider.getQuorumKeys();
      const currentHashes = new Set(quorums.keys());
      
      // Check for changes
      if (quorums.size !== previousQuorumCount) {
        console.log(`Quorum count changed: ${previousQuorumCount} -> ${quorums.size}`);
        previousQuorumCount = quorums.size;
      }
      
      // Check for new quorums
      for (const hash of currentHashes) {
        if (!previousHashes.has(hash)) {
          console.log(`New quorum detected: ${hash}`);
        }
      }
      
      // Check for removed quorums
      for (const hash of previousHashes) {
        if (!currentHashes.has(hash)) {
          console.log(`Quorum removed: ${hash}`);
        }
      }
      
      previousHashes = currentHashes;
    } catch (error: any) {
      console.error('Monitor error:', error.message);
    }
  };

  // Initial check
  await checkQuorums();
  
  // Set up monitoring interval
  const interval = setInterval(checkQuorums, 10000); // Check every 10 seconds
  
  // Handle graceful shutdown
  process.on('SIGINT', () => {
    clearInterval(interval);
    console.log('\nMonitoring stopped.');
    process.exit(0);
  });
}

// Run examples
async function main() {
  await quorumKeysExample();
  
  // Uncomment to run monitoring
  // await monitorQuorums();
}

main().catch(console.error);