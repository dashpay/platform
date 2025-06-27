/**
 * Bluetooth setup helper for easy SDK configuration
 */

import { SDK, SDKOptions } from '../SDK';
import { BluetoothProvider } from './BluetoothProvider';
import { BluetoothWallet } from './BluetoothWallet';
import { BluetoothConnection } from './BluetoothConnection';

export interface BluetoothSetupOptions {
  requireAuthentication?: boolean;
  autoReconnect?: boolean;
  timeout?: number;
}

/**
 * Setup SDK with Bluetooth provider and wallet
 */
export async function setupBluetoothSDK(
  options: BluetoothSetupOptions = {}
): Promise<{
  sdk: SDK;
  provider: BluetoothProvider;
  wallet: BluetoothWallet;
  connection: BluetoothConnection;
}> {
  // Check Bluetooth availability
  if (!BluetoothConnection.isAvailable()) {
    throw new Error(
      'Web Bluetooth is not available. Please use a compatible browser (Chrome, Edge) over HTTPS.'
    );
  }

  // Create Bluetooth provider
  const provider = new BluetoothProvider({
    requireAuthentication: options.requireAuthentication ?? true,
    autoReconnect: options.autoReconnect ?? true,
    timeout: options.timeout ?? 30000
  });

  // Connect to device
  await provider.connect();
  
  // Get connection and create wallet
  const connection = provider.getConnection();
  const wallet = new BluetoothWallet(connection);
  
  // Initialize wallet
  await wallet.initialize();
  
  // Get wallet info to determine network
  const walletInfo = wallet.getWalletInfo();
  
  // Create SDK with Bluetooth components
  const sdkOptions: SDKOptions = {
    network: walletInfo.network,
    contextProvider: provider,
    wallet: {
      adapter: wallet
    }
  };
  
  const sdk = new SDK(sdkOptions);
  await sdk.initialize();
  
  return {
    sdk,
    provider,
    wallet,
    connection
  };
}

/**
 * Quick setup for Bluetooth SDK with defaults
 */
export async function createBluetoothSDK(): Promise<SDK> {
  const { sdk } = await setupBluetoothSDK();
  return sdk;
}