import { SDK } from '../../SDK';
import { getWasmSdk } from '../../core/WasmLoader';
import { StateTransitionResult, ProofOptions } from '../../core/types';
import {
  Document,
  DocumentCreateOptions,
  DocumentReplaceOptions,
  DocumentDeleteOptions,
  DocumentsBatchOptions,
  DocumentQuery
} from './types';

export class DocumentModule {
  constructor(private sdk: SDK) {}

  private ensureInitialized(): void {
    if (!this.sdk.isInitialized()) {
      throw new Error('SDK not initialized. Call SDK.initialize() first.');
    }
  }

  async create(
    dataContractId: string,
    ownerId: string,
    type: string,
    data: Record<string, any>
  ): Promise<Document> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Validate contract exists using shared validator
    const { validateContract, validateDocumentType } = await import('../shared/ContractValidator');
    const contract = await validateContract(this.sdk, dataContractId);
    
    // Validate document type exists in contract
    validateDocumentType(contract, type);
    
    // Create document
    const document = await wasm.createDocument(
      wasmSdk,
      dataContractId,
      ownerId,
      type,
      data
    );
    
    return this.parseDocument(document);
  }

  async get(
    dataContractId: string,
    type: string,
    documentId: string,
    options: ProofOptions = {}
  ): Promise<Document | null> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    try {
      const document = await wasm.fetchDocument(
        wasmSdk,
        dataContractId,
        type,
        documentId,
        options.verify
      );
      
      if (!document) {
        return null;
      }
      
      return this.parseDocument(document);
    } catch (error: any) {
      if (error.message?.includes('not found')) {
        return null;
      }
      throw error;
    }
  }

  async query(query: DocumentQuery, options: ProofOptions = {}): Promise<Document[]> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Build query object for WASM
    const wasmQuery = {
      dataContractId: query.dataContractId,
      documentType: query.type,
      where: query.where || [],
      orderBy: query.orderBy || [],
      limit: query.limit || 100,
      startAt: query.startAt,
      startAfter: query.startAfter
    };
    
    const results = await wasm.fetchDocuments(
      wasmSdk,
      wasmQuery,
      options.verify
    );
    
    return results.map((doc: any) => this.parseDocument(doc));
  }

  async broadcast(
    dataContractId: string,
    ownerId: string,
    options: DocumentsBatchOptions
  ): Promise<StateTransitionResult> {
    this.ensureInitialized();
    
    const wasm = getWasmSdk();
    const wasmSdk = this.sdk.getWasmSdk();
    
    // Validate identity exists
    const identityModule = await import('../identities/IdentityModule');
    const identity = await new identityModule.IdentityModule(this.sdk).get(ownerId);
    
    if (!identity) {
      throw new Error(`Owner identity ${ownerId} not found`);
    }
    
    // Build transitions array
    const transitions = [];
    
    if ('create' in options) {
      for (const item of options.create) {
        const doc = await this.create(dataContractId, ownerId, item.type, item.data);
        transitions.push({
          action: 'create',
          document: doc
        });
      }
    }
    
    if ('replace' in options) {
      for (const item of options.replace) {
        const existing = await this.get(dataContractId, item.type, item.id);
        if (!existing) {
          throw new Error(`Document ${item.id} not found for replacement`);
        }
        
        transitions.push({
          action: 'replace',
          document: {
            ...existing,
            data: item.data,
            revision: item.revision
          }
        });
      }
    }
    
    if ('delete' in options) {
      for (const item of options.delete) {
        const existing = await this.get(dataContractId, item.type, item.id);
        if (!existing) {
          throw new Error(`Document ${item.id} not found for deletion`);
        }
        
        transitions.push({
          action: 'delete',
          document: existing
        });
      }
    }
    
    // Create batch transition
    const transition = await wasm.createDocumentsBatchTransition(
      wasmSdk,
      ownerId,
      transitions,
      identity.revision
    );
    
    // Sign and broadcast
    return this.broadcastTransition(transition);
  }

  async waitForConfirmation(
    dataContractId: string,
    type: string,
    documentId: string,
    timeout: number = 60000
  ): Promise<Document> {
    const startTime = Date.now();
    
    while (Date.now() - startTime < timeout) {
      const document = await this.get(dataContractId, type, documentId);
      if (document) {
        return document;
      }
      
      // Wait 2 seconds before retry
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    
    throw new Error(`Document ${documentId} not confirmed within ${timeout}ms`);
  }

  private async broadcastTransition(transition: any): Promise<StateTransitionResult> {
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

  private parseDocument(wasmDocument: any): Document {
    return {
      id: wasmDocument.id,
      dataContractId: wasmDocument.dataContractId,
      type: wasmDocument.type,
      ownerId: wasmDocument.ownerId,
      revision: wasmDocument.revision || 0,
      data: wasmDocument.data || {},
      createdAt: wasmDocument.createdAt,
      updatedAt: wasmDocument.updatedAt
    };
  }
}