/**
 * Bluetooth communication types and interfaces
 */

// Bluetooth Service and Characteristic UUIDs
export const DASH_BLUETOOTH_SERVICE_UUID = '00000000-dash-platform-bluetooth-service';
export const COMMAND_CHARACTERISTIC_UUID = '00000001-dash-platform-command-char';
export const RESPONSE_CHARACTERISTIC_UUID = '00000002-dash-platform-response-char';
export const STATUS_CHARACTERISTIC_UUID = '00000003-dash-platform-status-char';

// Message types for communication
export enum MessageType {
  // Context Provider requests
  GET_PLATFORM_STATUS = 'GET_PLATFORM_STATUS',
  GET_BLOCK_HEIGHT = 'GET_BLOCK_HEIGHT',
  GET_BLOCK_TIME = 'GET_BLOCK_TIME',
  GET_CORE_CHAIN_LOCKED_HEIGHT = 'GET_CORE_CHAIN_LOCKED_HEIGHT',
  GET_PLATFORM_VERSION = 'GET_PLATFORM_VERSION',
  GET_PROPOSER_BLOCK_COUNT = 'GET_PROPOSER_BLOCK_COUNT',
  GET_TIME_PER_BLOCK = 'GET_TIME_PER_BLOCK',
  GET_BLOCK_PROPOSER = 'GET_BLOCK_PROPOSER',
  
  // Wallet requests
  GET_ADDRESSES = 'GET_ADDRESSES',
  GET_IDENTITY_KEYS = 'GET_IDENTITY_KEYS',
  SIGN_STATE_TRANSITION = 'SIGN_STATE_TRANSITION',
  CREATE_ASSET_LOCK_PROOF = 'CREATE_ASSET_LOCK_PROOF',
  DERIVE_KEY = 'DERIVE_KEY',
  
  // Authentication
  AUTH_CHALLENGE = 'AUTH_CHALLENGE',
  AUTH_RESPONSE = 'AUTH_RESPONSE',
  
  // Control
  PING = 'PING',
  PONG = 'PONG',
  ERROR = 'ERROR',
  SUCCESS = 'SUCCESS'
}

export interface BluetoothMessage {
  id: string;
  type: MessageType;
  payload?: any;
  timestamp: number;
  signature?: string;
}

export interface BluetoothResponse {
  id: string;
  type: MessageType;
  success: boolean;
  data?: any;
  error?: {
    code: string;
    message: string;
  };
  timestamp: number;
}

export interface BluetoothDeviceInfo {
  id: string;
  name: string;
  paired: boolean;
  authenticated: boolean;
  rssi?: number;
}

export interface BluetoothConnectionOptions {
  timeout?: number;
  retries?: number;
  requireAuthentication?: boolean;
  encryptionKey?: Uint8Array;
}

export interface BluetoothSecurityOptions {
  requirePairing?: boolean;
  requireEncryption?: boolean;
  pinCode?: string;
  publicKey?: Uint8Array;
}

// Wallet-specific types
export interface BluetoothWalletInfo {
  walletId: string;
  network: 'mainnet' | 'testnet' | 'devnet';
  accounts: Array<{
    index: number;
    address: string;
    balance?: number;
  }>;
  identities: Array<{
    id: string;
    index: number;
  }>;
}

export interface AssetLockRequest {
  amount: number;
  accountIndex?: number;
  oneTimePrivateKey?: Uint8Array;
}

export interface SigningRequest {
  stateTransition: Uint8Array;
  identityId: string;
  keyIndex: number;
  keyType: 'ECDSA' | 'BLS';
}

// Events
export interface BluetoothEvents {
  'connected': (device: BluetoothDeviceInfo) => void;
  'disconnected': (reason?: string) => void;
  'authenticated': (device: BluetoothDeviceInfo) => void;
  'error': (error: Error) => void;
  'message': (message: BluetoothMessage) => void;
  'response': (response: BluetoothResponse) => void;
}