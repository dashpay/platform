import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class DocumentsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // Query many documents
  async query(params: {
    contractId: string;
    type: string;
    where?: unknown;
    orderBy?: unknown;
    limit?: number;
    startAfter?: string;
    startAt?: string;
  }): Promise<any> {
    const { contractId, type, where, orderBy, limit, startAfter, startAt } = params;
    const whereJson = asJsonString(where);
    const orderJson = asJsonString(orderBy);
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_documents(
      w,
      contractId,
      type,
      whereJson ?? null,
      orderJson ?? null,
      limit ?? null,
      startAfter ?? null,
      startAt ?? null,
    );
  }

  async queryWithProof(params: {
    contractId: string;
    type: string;
    where?: unknown;
    orderBy?: unknown;
    limit?: number;
    startAfter?: string;
    startAt?: string;
  }): Promise<any> {
    const { contractId, type, where, orderBy, limit, startAfter, startAt } = params;
    const whereJson = asJsonString(where);
    const orderJson = asJsonString(orderBy);
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_documents_with_proof_info(
      w,
      contractId,
      type,
      whereJson ?? null,
      orderJson ?? null,
      limit ?? null,
      startAfter ?? null,
      startAt ?? null,
    );
  }

  async get(contractId: string, type: string, documentId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_document(w, contractId, type, documentId);
  }

  async getWithProof(contractId: string, type: string, documentId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_document_with_proof_info(w, contractId, type, documentId);
  }

  async create(args: {
    contractId: string;
    type: string;
    ownerId: string;
    data: unknown;
    entropyHex: string;
    privateKeyWif: string;
  }): Promise<any> {
    const { contractId, type, ownerId, data, entropyHex, privateKeyWif } = args;
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
    contractId: string;
    type: string;
    documentId: string;
    ownerId: string;
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

  async delete(args: { contractId: string; type: string; documentId: string; ownerId: string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentDelete(contractId, type, documentId, ownerId, privateKeyWif);
  }

  async transfer(args: { contractId: string; type: string; documentId: string; ownerId: string; recipientId: string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, recipientId, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentTransfer(contractId, type, documentId, ownerId, recipientId, privateKeyWif);
  }

  async purchase(args: { contractId: string; type: string; documentId: string; buyerId: string; price: number | bigint | string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, buyerId, price, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentPurchase(contractId, type, documentId, buyerId, BigInt(price), privateKeyWif);
  }

  async setPrice(args: { contractId: string; type: string; documentId: string; ownerId: string; price: number | bigint | string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, price, privateKeyWif } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentSetPrice(contractId, type, documentId, ownerId, BigInt(price), privateKeyWif);
  }
}
