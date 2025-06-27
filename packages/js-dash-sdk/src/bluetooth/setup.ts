/**
 * Bluetooth setup helper for easy SDK configuration
 */

import { SDK, SDKOptions } from '../SDK';
import { BluetoothProvider } from './BluetoothProvider';
import { BluetoothWallet } from './BluetoothWallet';
import { BluetoothConnection } from './BluetoothConnection';

/**
 * Default timeout for Bluetooth operations
 */
export const DEFAULT_BLUETOOTH_TIMEOUT = 30000; // 30 seconds

export interface BluetoothSetupOptions {
  /**
   * Require authentication when connecting to device
   * @default true
   */
  requireAuthentication?: boolean;
  
  /**
   * Automatically reconnect if connection is lost
   * @default true
   */
  autoReconnect?: boolean;
  
  /**
   * Connection timeout in milliseconds
   * @default 30000 (30 seconds)
   */
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
    timeout: options.timeout ?? DEFAULT_BLUETOOTH_TIMEOUT
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