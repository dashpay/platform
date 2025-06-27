/**
 * Example: Using Bluetooth mobile device as context provider and wallet
 */

import { createSDK } from '../src';
import { BluetoothProvider, BluetoothWallet } from '../src/bluetooth';

async function main() {
  // Check if Bluetooth is available
  if (!('bluetooth' in navigator)) {
    console.error('Web Bluetooth is not supported in this browser');
    console.log('Please use Chrome, Edge, or another browser with Web Bluetooth support');
    return;
  }

  console.log('Dash Bluetooth Wallet Example');
  console.log('=============================\n');

  try {
    // Create Bluetooth provider
    console.log('Creating Bluetooth provider...');
    const bluetoothProvider = new BluetoothProvider({
      requireAuthentication: true,
      timeout: 30000
    });

    // Connect to mobile device
    console.log('\nSearching for Dash wallet devices...');
    console.log('Make sure your mobile wallet is in pairing mode');
    
    await bluetoothProvider.connect();
    console.log('✓ Connected to mobile device');

    // Get wallet instance
    const connection = bluetoothProvider.getConnection();
    const bluetoothWallet = new BluetoothWallet(connection);
    
    // Initialize wallet
    console.log('\nInitializing wallet...');
    await bluetoothWallet.initialize();
    console.log('✓ Wallet initialized');

    // Display wallet info
    const walletInfo = bluetoothWallet.getWalletInfo();
    console.log('\nWallet Information:');
    console.log(`  Network: ${walletInfo.network}`);
    console.log(`  Accounts: ${walletInfo.accounts.length}`);
    console.log(`  Identities: ${walletInfo.identities.length}`);

    // Create SDK with Bluetooth provider and wallet
    console.log('\nInitializing SDK with Bluetooth...');
    const sdk = createSDK({
      network: walletInfo.network,
      contextProvider: bluetoothProvider,
      wallet: {
        adapter: bluetoothWallet
      }
    });

    await sdk.initialize();
    console.log('✓ SDK initialized');

    // Get platform status from mobile device
    console.log('\nFetching platform status from mobile device...');
    const status = await bluetoothProvider.getPlatformStatus();
    console.log('Platform Status:');
    console.log(`  Block Height: ${status.blockHeight}`);
    console.log(`  Block Time: ${new Date(status.blockTime).toISOString()}`);
    console.log(`  Core Chain Locked Height: ${status.coreChainLockedHeight}`);
    console.log(`  Platform Version: ${status.version}`);
    console.log(`  Time Per Block: ${status.timePerBlock}ms`);

    // Example: Get identity balance
    if (walletInfo.identities.length > 0) {
      const identityId = walletInfo.identities[0].id;
      console.log(`\nFetching identity ${identityId}...`);
      
      const identity = await sdk.identities.get(identityId);
      if (identity) {
        console.log(`  Balance: ${identity.balance} credits`);
        console.log(`  Revision: ${identity.revision}`);
        console.log(`  Keys: ${identity.publicKeys.length}`);
      }
    }

    // Example: Create and sign a document
    console.log('\nExample: Creating a document (requires signing)...');
    
    // This would normally require a contract ID and proper document data
    // For demo purposes, we'll show the flow
    /*
    const document = await sdk.documents.create(
      'contractId',
      walletInfo.identities[0].id,
      'profile',
      {
        name: 'Alice',
        bio: 'Created from Bluetooth wallet'
      }
    );
    
    // The wallet will be called automatically to sign the state transition
    const result = await sdk.documents.broadcast(
      'contractId',
      walletInfo.identities[0].id,
      {
        create: [{
          type: 'profile',
          data: document.data
        }]
      }
    );
    
    console.log('Document created and signed via Bluetooth wallet!');
    */

    // Example: Monitor connection status
    connection.on('disconnected', () => {
      console.log('\n⚠️  Bluetooth connection lost');
    });

    connection.on('authenticated', () => {
      console.log('\n✓ Bluetooth authentication successful');
    });

    // Keep connection alive for demo
    console.log('\nBluetooth wallet is ready for operations.');
    console.log('The connection will remain active for signing requests.');
    console.log('Press Ctrl+C to disconnect and exit.\n');

    // Graceful shutdown
    process.on('SIGINT', async () => {
      console.log('\nDisconnecting...');
      await bluetoothProvider.disconnect();
      sdk.destroy();
      console.log('Goodbye!');
      process.exit(0);
    });

    // Keep the process running
    await new Promise(() => {});

  } catch (error: any) {
    console.error('\nError:', error.message);
    
    if (error.message.includes('not available')) {
      console.log('\nTroubleshooting:');
      console.log('1. Make sure you are using a compatible browser (Chrome, Edge)');
      console.log('2. Enable Bluetooth on your computer');
      console.log('3. Visit this page over HTTPS (required for Web Bluetooth)');
    } else if (error.message.includes('User cancelled')) {
      console.log('\nDevice selection was cancelled.');
    } else if (error.message.includes('Authentication failed')) {
      console.log('\nAuthentication failed. Make sure to approve the connection on your mobile device.');
    }
  }
}

// Run the example
main().catch(console.error);