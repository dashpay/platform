import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class DocumentsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  // Query many documents
  query(params: {
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
    return wasm.get_documents(
      this.sdk.wasm,
      contractId,
      type,
      whereJson ?? null,
      orderJson ?? null,
      limit ?? null,
      startAfter ?? null,
      startAt ?? null,
    );
  }

  queryWithProof(params: {
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
    return wasm.get_documents_with_proof_info(
      this.sdk.wasm,
      contractId,
      type,
      whereJson ?? null,
      orderJson ?? null,
      limit ?? null,
      startAfter ?? null,
      startAt ?? null,
    );
  }

  get(contractId: string, type: string, documentId: string): Promise<any> {
    return wasm.get_document(this.sdk.wasm, contractId, type, documentId);
  }

  getWithProof(contractId: string, type: string, documentId: string): Promise<any> {
    return wasm.get_document_with_proof_info(this.sdk.wasm, contractId, type, documentId);
  }

  create(args: {
    contractId: string;
    type: string;
    ownerId: string;
    data: unknown;
    entropyHex: string;
    privateKeyWif: string;
  }): Promise<any> {
    const { contractId, type, ownerId, data, entropyHex, privateKeyWif } = args;
    return this.sdk.wasm.documentCreate(
      contractId,
      type,
      ownerId,
      asJsonString(data)!,
      entropyHex,
      privateKeyWif,
    );
  }

  replace(args: {
    contractId: string;
    type: string;
    documentId: string;
    ownerId: string;
    data: unknown;
    revision: number | bigint;
    privateKeyWif: string;
  }): Promise<any> {
    const { contractId, type, documentId, ownerId, data, revision, privateKeyWif } = args;
    return this.sdk.wasm.documentReplace(
      contractId,
      type,
      documentId,
      ownerId,
      asJsonString(data)!,
      BigInt(revision),
      privateKeyWif,
    );
  }

  delete(args: { contractId: string; type: string; documentId: string; ownerId: string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, privateKeyWif } = args;
    return this.sdk.wasm.documentDelete(contractId, type, documentId, ownerId, privateKeyWif);
  }

  transfer(args: { contractId: string; type: string; documentId: string; ownerId: string; recipientId: string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, recipientId, privateKeyWif } = args;
    return this.sdk.wasm.documentTransfer(contractId, type, documentId, ownerId, recipientId, privateKeyWif);
  }

  purchase(args: { contractId: string; type: string; documentId: string; buyerId: string; price: number | bigint | string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, buyerId, price, privateKeyWif } = args;
    return this.sdk.wasm.documentPurchase(contractId, type, documentId, buyerId, BigInt(price), privateKeyWif);
  }

  setPrice(args: { contractId: string; type: string; documentId: string; ownerId: string; price: number | bigint | string; privateKeyWif: string }): Promise<any> {
    const { contractId, type, documentId, ownerId, price, privateKeyWif } = args;
    return this.sdk.wasm.documentSetPrice(contractId, type, documentId, ownerId, BigInt(price), privateKeyWif);
  }
}

