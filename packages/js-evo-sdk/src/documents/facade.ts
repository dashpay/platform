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

  /**
   * Chained document query — a provable semi-join:
   * `SELECT * FROM <outerDocumentType> WHERE $id IN
   *   (SELECT <joinProperty> FROM <innerDocumentType> WHERE ...)`.
   *
   * "Posts I liked" in one verified round trip: inner `like` through
   * its byLiker-style index, join `postId`, outer `post`. Both halves
   * ride ONE merged proof — a single quorum-signed state root by
   * construction — and the outer query is re-derived and checked
   * against the proven inner values, so the responding node cannot
   * steer the join. Paginate on the inner query with a range clause on
   * the join property.
   */
  async chained(query: wasm.ChainedDocumentsQuery): Promise<wasm.ChainedDocumentsResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getChainedDocuments(query);
  }

  async chainedWithProof(
    query: wasm.ChainedDocumentsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ChainedDocumentsResult>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getChainedDocumentsWithProofInfo(query);
  }

  /**
   * Composite document query: a page plus the sub-queries derived from
   * it (by-id joins, indexed lookups, grouped counts, siblings), in ONE
   * verified round trip.
   *
   * A feed page in a single call: the posts, their like counts, the
   * posts they quote, their authors' profiles, and the viewer's own
   * likes on them. Everything rides ONE merged proof under one
   * quorum-signed state root, and every sub-query is re-derived from
   * the proven page, so the responding node cannot substitute, omit,
   * or inject a sub-result. Paginate with a range clause on the page's
   * ordering property.
   */
  async composite(query: wasm.CompositeDocumentsQuery): Promise<wasm.CompositeDocumentsResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getCompositeDocuments(query);
  }

  async compositeWithProof(
    query: wasm.CompositeDocumentsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.CompositeDocumentsResult>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getCompositeDocumentsWithProofInfo(query);
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
   * type requires `$createdAt`. A document of a contested index joins a
   * contest: `options.contestFund` is the most, in credits, it pays into it.
   * For an indexOnly type the proof shows the document's entry at the proof's
   * block, not that this create wrote it: no stronger proof exists for one.
   */
  async create(options: wasm.DocumentCreateOptions): Promise<wasm.Document> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentCreate(options);
  }

  /**
   * The prefunded voting balance a create of `document` states to join the
   * contest it enters (a DPNS name, a moderation charter): the contested index
   * and the fund to join it now, which from protocol version 14 doubles once
   * the contest holds 250 contenders and again for every 50 more. Undefined
   * when the document joins no contest. {@link create} states it itself; a
   * transition built by hand passes it as `prefundedVotingBalance` to
   * `new DocumentCreateTransition`. Its `credits` alone is what the Rust SDK's
   * `contest_fund_to_join` returns; this is its `prefunded_voting_balance_to_join`.
   */
  async contestFundToJoin(
    document: wasm.Document,
  ): Promise<wasm.PrefundedVotingBalance | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContestFundToJoin(document);
  }

  async replace(options: wasm.DocumentReplaceOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.documentReplace(options);
  }

  /**
   * Deletes a document and resolves once the proof shows it gone. For an
   * indexOnly type the proof shows the document's entry gone at the proof's
   * block, not that this delete removed it: no stronger proof exists for one.
   */
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
