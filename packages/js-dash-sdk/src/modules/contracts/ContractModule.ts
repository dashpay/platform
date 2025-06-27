import { SDK } from '../../SDK';
import { getWasmSdk } from '../../core/WasmLoader';
import { StateTransitionResult, ProofOptions } from '../../core/types';
import {
  DataContract,
  ContractCreateOptions,
  ContractUpdateOptions,
  ContractHistoryEntry,
  ContractVersion
} from './types';

export class ContractModule {
  constructor(private sdk: SDK) {}

  private ensureInitialized(): void {
    if (!this.sdk.isInitialized()) {
      throw new Error('SDK not initialized. Call SDK.initialize() first.');
    }
  }

  async create(options: ContractCreateOptions): Promise<DataContract> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Validate owner identity exists
    const identityModule = await import('../identities/IdentityModule');
    const identity = await new identityModule.IdentityModule(this.sdk).get(options.ownerId);
    
    if (!identity) {
      throw new Error(`Owner identity ${options.ownerId} not found`);
    }
    
    // Create the contract
    const contractData = {
      ownerId: options.ownerId,
      schema: options.schema || {},
      documents: options.documentSchemas
    };
    
    const contract = await wasm.createDataContract(wasmSdk, contractData);
    
    return this.parseContract(contract);
  }

  async get(contractId: string, options: ProofOptions = {}): Promise<DataContract | null> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    try {
      const contractResult = await wasm.fetchDataContract(wasmSdk, contractId, options.verify);
      
      if (!contractResult) {
        return null;
      }
      
      return this.parseContract(contractResult);
    } catch (error: any) {
      if (error.message?.includes('not found')) {
        return null;
      }
      throw error;
    }
  }

  async publish(contract: DataContract): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Create contract create transition
    const transition = await wasm.createDataContractCreateTransition(
      wasmSdk,
      contract
    );
    
    // Sign and broadcast
    return this.broadcast(transition);
  }

  async update(contractId: string, options: ContractUpdateOptions): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Fetch current contract
    const contract = await this.get(contractId);
    if (!contract) {
      throw new Error(`Contract ${contractId} not found`);
    }
    
    // Apply updates
    const updatedContract = {
      ...contract,
      schema: options.schema || contract.schema,
      documentSchemas: options.documentSchemas || contract.documentSchemas,
      version: contract.version + 1
    };
    
    // Create update transition
    const transition = await wasm.createDataContractUpdateTransition(
      wasmSdk,
      updatedContract
    );
    
    return this.broadcast(transition);
  }

  async getHistory(contractId: string, limit?: number, offset?: number): Promise<ContractHistoryEntry[]> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    const history = await wasm.fetchContractHistory(
      wasmSdk,
      contractId,
      undefined, // startAt
      limit,
      offset
    );
    
    return history.map((entry: any) => ({
      contractId: entry.contractId,
      version: entry.version,
      operation: entry.operation,
      timestamp: entry.timestamp,
      changes: entry.changes,
      transactionHash: entry.transactionHash
    }));
  }

  async getVersions(contractId: string): Promise<ContractVersion[]> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    const versions = await wasm.fetchContractVersions(wasmSdk, contractId);
    
    return versions.map((version: any) => ({
      version: version.version,
      schemaHash: version.schemaHash,
      ownerId: version.ownerId,
      createdAt: version.createdAt,
      documentTypesCount: version.documentTypesCount,
      totalDocuments: version.totalDocuments
    }));
  }

  async waitForConfirmation(contractId: string, timeout: number = 60000): Promise<DataContract> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeout) {
      const contract = await this.get(contractId);
      if (contract) {
        return contract;
      }
      
      // Wait 2 seconds before retry
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    
    throw new Error(`Contract ${contractId} not confirmed within ${timeout}ms`);
  }

  private async broadcast(transition: any): Promise<StateTransitionResult> {
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
      false // skipValidation
    );
    
    return {
      stateTransition: transition,
      metadata: result.metadata
    };
  }

  private parseContract(wasmContract: any): DataContract {
    return {
      id: wasmContract.id,
      ownerId: wasmContract.ownerId,
      schema: wasmContract.schema || {},
      version: wasmContract.version,
      documentSchemas: wasmContract.documents || {}
    };
  }
}