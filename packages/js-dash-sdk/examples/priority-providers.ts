/**
 * Example: Using priority-based context providers with fallback
 */

import { createSDK } from '../src';
import { 
  ProviderFactory,
  PriorityContextProvider,
  WebServiceProvider,
  ProviderCapability
} from '../src/providers';
import { BluetoothProvider } from '../src/bluetooth';

async function priorityProviderExample() {
  console.log('Priority Provider Example');
  console.log('========================\n');

  // Example 1: Using factory with Bluetooth priority
  console.log('Example 1: Bluetooth priority (if available)');
  try {
    const provider = await ProviderFactory.createWithBluetooth({
      network: 'testnet',
      bluetooth: {
        requireAuthentication: true,
        autoReconnect: true
      },
      webservice: {
        cacheDuration: 60000 // 1 minute cache
      },
      fallbackEnabled: true,
      logErrors: true
    });

    const sdk = createSDK({
      network: 'testnet',
      contextProvider: provider
    });

    await sdk.initialize();
    console.log('✓ SDK initialized with priority provider');

    // This will try Bluetooth first, then web service, then centralized
    const height = await sdk.getContextProvider().getLatestPlatformBlockHeight();
    console.log(`Platform height: ${height}`);

    // Check which provider was used
    if (provider instanceof PriorityContextProvider) {
      const activeProvider = await provider.getActiveProvider();
      console.log(`Active provider: ${activeProvider?.name}`);
    }
  } catch (error: any) {
    console.log('Bluetooth not available, using web service fallback');
  }

  // Example 2: Web service with custom configuration
  console.log('\n\nExample 2: Web service priority');
  const webProvider = await ProviderFactory.createWithWebService({
    network: 'testnet',
    webservice: {
      cacheDuration: 30000,
      // url: 'https://custom-quorum-service.com' // Custom URL if needed
    }
  });

  const sdk2 = createSDK({
    network: 'testnet',
    contextProvider: webProvider
  });

  await sdk2.initialize();

  // Fetch quorum keys (web service specific feature)
  if (webProvider instanceof PriorityContextProvider) {
    const activeProvider = await webProvider.getActiveProvider();
    if (activeProvider?.provider instanceof WebServiceProvider) {
      console.log('\nFetching quorum keys from web service...');
      const quorumKeys = await activeProvider.provider.getQuorumKeys();
      console.log(`Found ${quorumKeys.size} quorum keys`);
      
      // Display first few quorum hashes
      let count = 0;
      for (const [hash, info] of quorumKeys) {
        if (count++ >= 3) break;
        console.log(`  ${hash.substring(0, 16)}... - Type: ${info.quorumPublicKey.type}`);
      }
    }
  }

  // Example 3: Custom priority configuration
  console.log('\n\nExample 3: Custom priority configuration');
  
  const customProvider = new PriorityContextProvider({
    providers: [
      {
        provider: new WebServiceProvider({ network: 'testnet' }),
        priority: 150, // Highest priority
        name: 'Primary Web Service',
        capabilities: [
          ProviderCapability.PLATFORM_STATE,
          ProviderCapability.QUORUM_KEYS
        ]
      },
      {
        provider: new WebServiceProvider({ 
          network: 'testnet',
          url: 'https://backup-quorum.example.com' // Backup service
        }),
        priority: 100,
        name: 'Backup Web Service'
      }
    ],
    fallbackEnabled: true,
    cacheResults: true,
    logErrors: false
  });

  // Monitor provider events
  customProvider.on('provider:used', (name, method) => {
    console.log(`Provider ${name} used for ${method}`);
  });

  customProvider.on('provider:error', (name, error) => {
    console.log(`Provider ${name} failed: ${error.message}`);
  });

  customProvider.on('provider:fallback', (from, to) => {
    console.log(`Fallback from ${from} to ${to}`);
  });

  const sdk3 = createSDK({
    network: 'testnet',
    contextProvider: customProvider
  });

  await sdk3.initialize();
  
  // Test fallback behavior
  console.log('\nTesting provider fallback...');
  const version = await customProvider.getLatestPlatformVersion();
  console.log(`Platform version: ${version}`);

  // Get metrics
  console.log('\nProvider metrics:');
  const metrics = customProvider.getMetrics();
  for (const [name, stats] of metrics) {
    console.log(`  ${name}:`);
    console.log(`    Success: ${stats.successCount}`);
    console.log(`    Errors: ${stats.errorCount}`);
    console.log(`    Avg Response: ${stats.averageResponseTime.toFixed(2)}ms`);
  }
}

// Example 4: Bluetooth + Web Service for different operations
async function hybridExample() {
  console.log('\n\nExample 4: Hybrid Setup (Bluetooth for signing, Web for state)');
  console.log('============================================================\n');

  try {
    // Create Bluetooth provider for wallet operations
    const bluetoothProvider = new BluetoothProvider({
      requireAuthentication: true
    });
    
    // Try to connect to Bluetooth
    console.log('Attempting Bluetooth connection...');
    await bluetoothProvider.connect();
    console.log('✓ Bluetooth connected');

    // Create priority provider for context
    const contextProvider = await ProviderFactory.create({
      providers: ['webservice', 'centralized'],
      network: 'testnet',
      usePriority: true
    });

    // Create SDK with hybrid setup
    const sdk = createSDK({
      network: 'testnet',
      contextProvider: contextProvider, // Web service for platform state
      wallet: {
        bluetooth: true // Bluetooth for signing
      }
    });

    await sdk.initialize();
    console.log('✓ Hybrid SDK initialized');

    // Platform state comes from web service
    console.log('\nGetting platform state from web service...');
    const blockHeight = await sdk.getContextProvider().getLatestPlatformBlockHeight();
    console.log(`Block height: ${blockHeight}`);

    // Signing would use Bluetooth wallet
    console.log('\nBluetooth wallet ready for signing operations');
    
  } catch (error: any) {
    if (error.message.includes('Bluetooth')) {
      console.log('Bluetooth not available - web service only mode');
    } else {
      console.error('Setup error:', error.message);
    }
  }
}

// Run examples
async function main() {
  try {
    await priorityProviderExample();
    await hybridExample();
  } catch (error) {
    console.error('Example error:', error);
  }
}

main().catch(console.error);