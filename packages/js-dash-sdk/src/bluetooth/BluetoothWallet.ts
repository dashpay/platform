/**
 * Bluetooth-based wallet that delegates signing and key management to a mobile device
 */

import { BluetoothConnection } from './BluetoothConnection';
import { BluetoothProtocol } from './protocol';
import { 
  MessageType,
  BluetoothWalletInfo,
  AssetLockRequest,
  SigningRequest
} from './types';
import { WalletAdapter } from '../modules/wallet/types';

export class BluetoothWallet implements WalletAdapter {
  private walletInfo: BluetoothWalletInfo | null = null;

  constructor(private connection: BluetoothConnection) {}

  /**
   * Initialize wallet and get info
   */
  async initialize(): Promise<void> {
    if (!this.connection.isConnected()) {
      throw new Error('Not connected to device');
    }

    // Get wallet info
    const request = BluetoothProtocol.createRequest(MessageType.GET_ADDRESSES);
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get wallet info: ${response.error?.message}`);
    }

    this.walletInfo = response.data as BluetoothWalletInfo;
  }

  /**
   * Get wallet information
   */
  getWalletInfo(): BluetoothWalletInfo {
    if (!this.walletInfo) {
      throw new Error('Wallet not initialized');
    }
    return this.walletInfo;
  }

  /**
   * Get addresses for the wallet
   */
  async getAddresses(accountIndex?: number): Promise<string[]> {
    const request = BluetoothProtocol.createRequest(
      MessageType.GET_ADDRESSES,
      { accountIndex }
    );
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get addresses: ${response.error?.message}`);
    }

    return response.data.addresses;
  }

  /**
   * Get identity keys
   */
  async getIdentityKeys(identityId: string): Promise<Array<{
    id: number;
    type: string;
    purpose: string;
    securityLevel: string;
    data: Uint8Array;
  }>> {
    const request = BluetoothProtocol.createRequest(
      MessageType.GET_IDENTITY_KEYS,
      { identityId }
    );
    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to get identity keys: ${response.error?.message}`);
    }

    // Convert base64 keys back to Uint8Array
    return response.data.keys.map((key: any) => ({
      ...key,
      data: new Uint8Array(Buffer.from(key.data, 'base64'))
    }));
  }

  /**
   * Sign a state transition
   */
  async signStateTransition(
    stateTransition: Uint8Array,
    identityId: string,
    keyIndex: number,
    keyType: 'ECDSA' | 'BLS' = 'ECDSA'
  ): Promise<Uint8Array> {
    const signingRequest: SigningRequest = {
      stateTransition,
      identityId,
      keyIndex,
      keyType
    };

    const request = BluetoothProtocol.createRequest(
      MessageType.SIGN_STATE_TRANSITION,
      {
        stateTransition: Buffer.from(stateTransition).toString('base64'),
        identityId: signingRequest.identityId,
        keyIndex: signingRequest.keyIndex,
        keyType: signingRequest.keyType
      }
    );

    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to sign state transition: ${response.error?.message}`);
    }

    // Convert base64 signature back to Uint8Array
    return new Uint8Array(Buffer.from(response.data.signature, 'base64'));
  }

  /**
   * Create an asset lock proof for identity funding
   */
  async createAssetLockProof(request: AssetLockRequest): Promise<{
    type: 'instant' | 'chain';
    instantLock?: Uint8Array;
    transaction?: Uint8Array;
    outputIndex?: number;
  }> {
    const bluetoothRequest = BluetoothProtocol.createRequest(
      MessageType.CREATE_ASSET_LOCK_PROOF,
      {
        amount: request.amount,
        accountIndex: request.accountIndex,
        oneTimePrivateKey: request.oneTimePrivateKey 
          ? Buffer.from(request.oneTimePrivateKey).toString('base64')
          : undefined
      }
    );

    const response = await this.connection.sendRequest(bluetoothRequest);

    if (!response.success) {
      throw new Error(`Failed to create asset lock proof: ${response.error?.message}`);
    }

    const proof = response.data;
    
    // Convert base64 data back to Uint8Array
    return {
      type: proof.type,
      instantLock: proof.instantLock 
        ? new Uint8Array(Buffer.from(proof.instantLock, 'base64'))
        : undefined,
      transaction: proof.transaction
        ? new Uint8Array(Buffer.from(proof.transaction, 'base64'))
        : undefined,
      outputIndex: proof.outputIndex
    };
  }

  /**
   * Derive a new key
   */
  async deriveKey(
    derivationPath: string,
    keyType: 'ECDSA' | 'BLS' = 'ECDSA'
  ): Promise<{
    publicKey: Uint8Array;
    chainCode?: Uint8Array;
  }> {
    const request = BluetoothProtocol.createRequest(
      MessageType.DERIVE_KEY,
      {
        derivationPath,
        keyType
      }
    );

    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to derive key: ${response.error?.message}`);
    }

    return {
      publicKey: new Uint8Array(Buffer.from(response.data.publicKey, 'base64')),
      chainCode: response.data.chainCode 
        ? new Uint8Array(Buffer.from(response.data.chainCode, 'base64'))
        : undefined
    };
  }

  /**
   * Get balance for an account
   */
  async getBalance(accountIndex: number = 0): Promise<number> {
    if (!this.walletInfo) {
      throw new Error('Wallet not initialized');
    }

    const account = this.walletInfo.accounts.find(a => a.index === accountIndex);
    if (!account) {
      throw new Error(`Account ${accountIndex} not found`);
    }

    return account.balance || 0;
  }

  /**
   * Check if wallet is ready
   */
  isReady(): boolean {
    return this.connection.isConnected() && 
           this.connection.isAuthenticated() && 
           this.walletInfo !== null;
  }

  /**
   * Get network type
   */
  getNetwork(): 'mainnet' | 'testnet' | 'devnet' {
    if (!this.walletInfo) {
      throw new Error('Wallet not initialized');
    }
    return this.walletInfo.network;
  }

  /**
   * Export wallet info for backup/display
   */
  exportWalletInfo(): BluetoothWalletInfo | null {
    return this.walletInfo ? { ...this.walletInfo } : null;
  }

  /**
   * Sign arbitrary data (for authentication/verification)
   */
  async signData(
    data: Uint8Array,
    accountIndex: number = 0
  ): Promise<Uint8Array> {
    const request = BluetoothProtocol.createRequest(
      MessageType.SIGN_STATE_TRANSITION, // Reuse signing endpoint
      {
        stateTransition: Buffer.from(data).toString('base64'),
        identityId: 'data-signing', // Special identifier for raw data signing
        keyIndex: accountIndex,
        keyType: 'ECDSA'
      }
    );

    const response = await this.connection.sendRequest(request);

    if (!response.success) {
      throw new Error(`Failed to sign data: ${response.error?.message}`);
    }

    return new Uint8Array(Buffer.from(response.data.signature, 'base64'));
  }
}