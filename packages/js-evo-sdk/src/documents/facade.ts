import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class DocumentsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // Query many documents
  async query(query: wasm.DocumentsQuery): Promise<Map<string, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocuments(query);
  }

  async queryWithProof(
    query: wasm.DocumentsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<string, wasm.Document | undefined>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsWithProofInfo(query);
  }

  async get(contractId: wasm.IdentifierLike, type: string, documentId: wasm.IdentifierLike):
    Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocument(contractId, type, documentId);
  }

  async getWithProof(
    contractId: wasm.IdentifierLike,
    type: string,
    documentId: wasm.IdentifierLike,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentWithProofInfo(contractId, type, documentId);
  }

  async create(options: wasm.DocumentCreateOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentCreate(options);
  }

  async replace(options: wasm.DocumentReplaceOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentReplace(options);
  }

  async delete(options: wasm.DocumentDeleteOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentDelete(options);
  }

  async transfer(options: wasm.DocumentTransferOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentTransfer(options);
  }

  async purchase(options: wasm.DocumentPurchaseOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentPurchase(options);
  }

  async setPrice(options: wasm.DocumentSetPriceOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentSetPrice(options);
  }
}
