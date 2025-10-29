import * as wasm from '../wasm.js';
import { asJsonString, generateEntropy } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class DocumentsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // Query many documents
  async query(query: wasm.DocumentsQuery): Promise<Map<wasm.Identifier, wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocuments(query);
  }

  async queryWithProof(query: wasm.DocumentsQuery): Promise<wasm.ProofMetadataResponseTyped<Map<wasm.Identifier, wasm.Document | undefined>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsWithProofInfo(query);
  }

  async get(contractId: wasm.IdentifierLike, type: string, documentId: wasm.IdentifierLike): Promise<wasm.Document | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocument(contractId, type, documentId);
  }

  async getWithProof(contractId: wasm.IdentifierLike, type: string, documentId: wasm.IdentifierLike): Promise<wasm.ProofMetadataResponseTyped<wasm.Document | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentWithProofInfo(contractId, type, documentId);
  }

  async create(args: {
    contractId: wasm.IdentifierLike;
    type: string;
    ownerId: wasm.IdentifierLike;
    data: unknown;
    entropyHex?: string; // Now optional - will auto-generate if not provided
    privateKeyWif: string;
  }): Promise<any> {
    const { contractId, type, ownerId, data, privateKeyWif } = args;
    // Auto-generate entropy if not provided
    const entropyHex = args.entropyHex ?? generateEntropy();
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentCreate(
      contractId,
      type,
      ownerId,
      asJsonString(data)!,
      entropyHex,
      privateKeyWif,
    );
  }

  async replace(args: {
    contractId: wasm.IdentifierLike;
    type: string;
    documentId: wasm.IdentifierLike;
    ownerId: wasm.IdentifierLike;
    data: unknown;
    revision: number | bigint;
    privateKeyWif: string;
  }): Promise<any> {
    const { contractId, type, documentId, ownerId, data, revision, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentReplace(
      contractId,
      type,
      documentId,
      ownerId,
      asJsonString(data)!,
      BigInt(revision),
      privateKeyWif,
    );
  }

  async delete(args: { contractId: wasm.IdentifierLike; type: string; documentId: wasm.IdentifierLike; ownerId: wasm.IdentifierLike; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentDelete(contractId, type, documentId, ownerId, privateKeyWif);
  }

  async transfer(args: { contractId: wasm.IdentifierLike; type: string; documentId: wasm.IdentifierLike; ownerId: wasm.IdentifierLike; recipientId: wasm.IdentifierLike; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, recipientId, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentTransfer(contractId, type, documentId, ownerId, recipientId, privateKeyWif);
  }

  async purchase(args: { contractId: wasm.IdentifierLike; type: string; documentId: wasm.IdentifierLike; buyerId: wasm.IdentifierLike; price: number | bigint | string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, buyerId, price, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentPurchase(contractId, type, documentId, buyerId, BigInt(price), privateKeyWif);
  }

  async setPrice(args: { contractId: wasm.IdentifierLike; type: string; documentId: wasm.IdentifierLike; ownerId: wasm.IdentifierLike; price: number | bigint | string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, price, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentSetPrice(contractId, type, documentId, ownerId, BigInt(price), privateKeyWif);
  }
}
