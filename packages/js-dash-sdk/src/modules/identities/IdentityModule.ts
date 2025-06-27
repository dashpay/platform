import { SDK } from '../../SDK';
import { getWasmSdk } from '../../core/WasmLoader';
import { StateTransitionResult, BroadcastOptions, ProofOptions } from '../../core/types';
import {
  Identity,
  IdentityCreateOptions,
  IdentityTopUpOptions,
  IdentityUpdateOptions,
  AssetLockProof,
  CreditTransferOptions,
  CreditWithdrawalOptions
} from './types';

export class IdentityModule {
  constructor(private sdk: SDK) {}

  private ensureInitialized(): void {
    if (!this.sdk.isInitialized()) {
      throw new Error('SDK not initialized. Call SDK.initialize() first.');
    }
  }

  async register(options: IdentityCreateOptions = {}): Promise<Identity> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Create asset lock proof (this would come from wallet integration)
    // For now, we'll throw an error indicating wallet is needed
    if (!this.sdk.getOptions().wallet) {
      throw new Error('Wallet required for identity registration. Configure SDK with wallet options.');
    }
    
    // TODO: Implement asset lock creation through wallet
    // const assetLockProof = await this.createAssetLockProof(options.fundingAmount || 100000);
    
    // Create identity create transition
    // const transition = await wasm.createIdentityCreateTransition(
    //   wasmSdk,
    //   assetLockProof,
    //   options.keys
    // );
    
    // Broadcast and wait for confirmation
    // const result = await this.broadcast(transition);
    
    // Return the created identity
    // return this.get(result.identityId);
    
    throw new Error('Identity registration requires wallet integration (not yet implemented)');
  }

  async get(identityId: string, options: ProofOptions = {}): Promise<Identity | null> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    try {
      const identityResult = await wasm.fetchIdentity(wasmSdk, identityId, options.verify);
      
      if (!identityResult) {
        return null;
      }
      
      // Parse the identity from WASM result
      return this.parseIdentity(identityResult);
    } catch (error: any) {
      if (error.message?.includes('not found')) {
        return null;
      }
      throw error;
    }
  }

  async topUp(identityId: string, options: IdentityTopUpOptions): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Verify identity exists
    const identity = await this.get(identityId);
    if (!identity) {
      throw new Error(`Identity ${identityId} not found`);
    }
    
    // Create asset lock for top up amount
    if (!this.sdk.getOptions().wallet) {
      throw new Error('Wallet required for identity top up. Configure SDK with wallet options.');
    }
    
    // TODO: Implement asset lock creation and top up transition
    throw new Error('Identity top up requires wallet integration (not yet implemented)');
  }

  async update(identityId: string, options: IdentityUpdateOptions): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Fetch current identity
    const identity = await this.get(identityId);
    if (!identity) {
      throw new Error(`Identity ${identityId} not found`);
    }
    
    // Create identity update transition
    const updateData: any = {};
    
    if (options.addKeys && options.addKeys.length > 0) {
      updateData.addPublicKeys = options.addKeys;
    }
    
    if (options.disableKeys && options.disableKeys.length > 0) {
      updateData.disablePublicKeys = options.disableKeys;
    }
    
    const transition = await wasm.createIdentityUpdateTransition(
      wasmSdk,
      identityId,
      identity.revision,
      updateData
    );
    
    // Sign and broadcast
    return this.broadcast(transition);
  }

  async getBalance(identityId: string): Promise<number> {
    const identity = await this.get(identityId);
    if (!identity) {
      throw new Error(`Identity ${identityId} not found`);
    }
    return identity.balance;
  }

  async creditTransfer(identityId: string, options: CreditTransferOptions): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Verify identities exist
    const [sender, recipient] = await Promise.all([
      this.get(identityId),
      this.get(options.recipientId)
    ]);
    
    if (!sender) {
      throw new Error(`Sender identity ${identityId} not found`);
    }
    
    if (!recipient) {
      throw new Error(`Recipient identity ${options.recipientId} not found`);
    }
    
    if (sender.balance < options.amount) {
      throw new Error(`Insufficient balance. Required: ${options.amount}, available: ${sender.balance}`);
    }
    
    const transition = await wasm.createIdentityCreditTransferTransition(
      wasmSdk,
      identityId,
      options.recipientId,
      options.amount,
      sender.revision
    );
    
    return this.broadcast(transition);
  }

  async creditWithdrawal(identityId: string, options: CreditWithdrawalOptions): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Verify identity exists and has sufficient balance
    const identity = await this.get(identityId);
    if (!identity) {
      throw new Error(`Identity ${identityId} not found`);
    }
    
    if (identity.balance < options.amount) {
      throw new Error(`Insufficient balance. Required: ${options.amount}, available: ${identity.balance}`);
    }
    
    const transition = await wasm.createIdentityCreditWithdrawalTransition(
      wasmSdk,
      identityId,
      options.amount,
      options.coreFeePerByte,
      identity.revision,
      options.pooling || 'if-needed',
      options.outputScript
    );
    
    return this.broadcast(transition);
  }

  async getByPublicKeyHash(publicKeyHash: Uint8Array | string): Promise<Identity[]> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    const hash = typeof publicKeyHash === 'string' 
      ? Buffer.from(publicKeyHash, 'hex')
      : publicKeyHash;
    
    const results = await wasm.fetchIdentitiesByPublicKeyHash(wasmSdk, hash);
    
    return results.map((result: any) => this.parseIdentity(result));
  }

  async waitForConfirmation(identityId: string, timeout: number = 60000): Promise<Identity> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeout) {
      const identity = await this.get(identityId);
      if (identity) {
        return identity;
      }
      
      // Wait 2 seconds before retry
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    
    throw new Error(`Identity ${identityId} not confirmed within ${timeout}ms`);
  }

  private async broadcast(transition: any, options: BroadcastOptions = {}): Promise<StateTransitionResult> {
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Sign the transition if wallet is available
    if (this.sdk.getOptions().wallet) {
      // TODO: Sign transition with wallet
    }
    
    // Broadcast
    const result = await wasm.broadcastStateTransition(
      wasmSdk,
      transition,
      options.skipValidation
    );
    
    return {
      stateTransition: transition,
      metadata: result.metadata
    };
  }

  private parseIdentity(wasmIdentity: any): Identity {
    return {
      id: wasmIdentity.id,
      balance: wasmIdentity.balance,
      revision: wasmIdentity.revision,
      publicKeys: wasmIdentity.publicKeys?.map((key: any) => ({
        id: key.id,
        type: key.type,
        purpose: key.purpose,
        securityLevel: key.securityLevel,
        data: key.data,
        readOnly: key.readOnly || false,
        disabledAt: key.disabledAt
      })) || []
    };
  }
}