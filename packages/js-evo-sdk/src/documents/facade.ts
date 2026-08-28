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

  async history(query: wasm.DocumentHistoryQuery): Promise<Map<bigint, wasm.Document>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentHistory(query);
  }

  async historyWithProof(
    query: wasm.DocumentHistoryQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<Map<bigint, wasm.Document>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentHistoryWithProofInfo(query);
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

  /**
   * Creates a document and resolves to the confirmed Document as Platform
   * committed it, consensus-populated system fields included — keep this
   * instance when you later intend to delete an indexOnly document whose
   * type requires `$createdAt`.
   */
  async create(options: wasm.DocumentCreateOptions): Promise<wasm.Document> {
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

  async count(query: wasm.DocumentsQuery): Promise<Map<string, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsCount(query);
  }

  async countWithProof(
    query: wasm.DocumentsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<Map<string, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsCountWithProofInfo(query);
  }

  async sum(
    query: wasm.DocumentsQuery,
    sumProperty: string,
  ): Promise<Map<string, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsSum(query, sumProperty);
  }

  async sumWithProof(
    query: wasm.DocumentsQuery,
    sumProperty: string,
  ): Promise<wasm.ProofMetadataResponseTyped<Map<string, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsSumWithProofInfo(query, sumProperty);
  }

  async average(
    query: wasm.DocumentsQuery,
    averageProperty: string,
  ): Promise<Map<string, { count: bigint; sum: bigint }>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsAverage(query, averageProperty);
  }

  async averageWithProof(
    query: wasm.DocumentsQuery,
    averageProperty: string,
  ): Promise<wasm.ProofMetadataResponseTyped<Map<string, { count: bigint; sum: bigint }>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsAverageWithProofInfo(query, averageProperty);
  }

  /**
   * Rank groups by an aggregate and return the top (or bottom) `limit` of
   * them. Requires protocol version 14 and a contract index declaring the
   * matching ranked keyword.
   */
  async ranked(query: wasm.DocumentsRankedQuery): Promise<wasm.DocumentsRankedResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsRanked(query);
  }

  async rankedWithProof(
    query: wasm.DocumentsRankedQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.DocumentsRankedResult>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsRankedWithProofInfo(query);
  }

  /**
   * Return the groups whose aggregate falls inside a bound. Same ranked
   * indexes as {@link ranked}, bounded by value rather than by position.
   */
  async having(query: wasm.DocumentsHavingQuery): Promise<wasm.DocumentsHavingResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsHaving(query);
  }

  async havingWithProof(
    query: wasm.DocumentsHavingQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.DocumentsHavingResult>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDocumentsHavingWithProofInfo(query);
  }
}
