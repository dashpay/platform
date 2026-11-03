import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class ContractsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async fetch(contractId: wasm.IdentifierLike): Promise<wasm.DataContract | undefined> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContract(contractId);
  }

  async fetchWithProof(contractId: wasm.IdentifierLike):
    Promise<wasm.ProofMetadataResponseTyped<wasm.DataContract>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractWithProofInfo(contractId);
  }

  async getHistory(query: wasm.DataContractHistoryQuery): Promise<Map<bigint, wasm.DataContract>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractHistory(query);
  }

  async getHistoryWithProof(
    query: wasm.DataContractHistoryQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<Map<bigint, wasm.DataContract>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractHistoryWithProofInfo(query);
  }

  async getMany(contractIds: wasm.IdentifierLikeArray): Promise<Map<string, wasm.DataContract | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContracts(contractIds);
  }

  async getManyWithProof(
    contractIds: wasm.IdentifierLikeArray,
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<string, wasm.DataContract | undefined>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractsWithProofInfo(contractIds);
  }

  /**
   * One page of every contract on Platform, ordered by ascending contract id.
   * Pass `{}` for the first page and the last key of a page as `startAfter` for the next.
   */
  async getByRange(
    query: wasm.DataContractsByRangeQuery,
  ): Promise<Map<string, wasm.DataContract | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractsByRange(query);
  }

  async getByRangeWithProof(
    query: wasm.DataContractsByRangeQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<string, wasm.DataContract | undefined>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractsByRangeWithProofInfo(query);
  }

  /**
   * The current versions of contracts: the cheap check that contracts held locally are still
   * current. One entry per requested id, `undefined` for an id no contract has; the contracts
   * themselves come back only with `includeContracts`.
   */
  async getLatestVersions(
    query: wasm.DataContractsLatestVersionsQuery,
  ): Promise<Map<string, wasm.DataContractLatestVersion | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractsLatestVersions(query);
  }

  /**
   * Seed the contract cache with a contract the app already holds (a bundled snapshot, a
   * contract it just published), so queries against it need no fetch. Persisted like a
   * fetched contract. Returns false when the SDK runs without a trusted context. Pair with
   * {@link getLatestVersions}, off the critical path, to learn whether the held contract is
   * still current.
   */
  async addKnown(contract: wasm.DataContract): Promise<boolean> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.addKnownContract(contract);
  }

  async getLatestVersionsWithProof(
    query: wasm.DataContractsLatestVersionsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<string, wasm.DataContractLatestVersion | undefined>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractsLatestVersionsWithProofInfo(query);
  }

  async publish(options: wasm.ContractPublishOptions): Promise<wasm.DataContract> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractPublish(options);
  }

  async update(options: wasm.ContractUpdateOptions): Promise<void> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractUpdate(options);
  }

  /**
   * Puts an identity on a moderated contract's banlist (protocol version 14). Signed by the
   * contract owner or a moderator the contract's config names, with a CRITICAL authentication
   * key. A banned identity cannot act on the contract at the document level. `options.reason`
   * is required and stored with the entry: a free text of at most 1024 bytes, and an optional
   * `code` nothing checks, reserved for ban codes contracts may declare later.
   */
  async banUser(options: wasm.ContractBanOptions): Promise<wasm.ContractModerationResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractBanUser(options);
  }

  /** Takes an identity off a moderated contract's banlist. */
  async unbanUser(options: wasm.ContractModerationOptions): Promise<wasm.ContractModerationResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractUnbanUser(options);
  }

  /**
   * Suspends an identity on a moderated contract until the block time `until` (milliseconds),
   * replacing a suspension it already carries. The suspension is swept by the identity's first
   * document transition after it lapses. `options.until` and `options.reason` are required.
   */
  async suspendUser(options: wasm.ContractSuspendOptions): Promise<wasm.ContractModerationResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractSuspendUser(options);
  }

  /** Takes an identity off a moderated contract's suspension list, lapsed or not. */
  async unsuspendUser(options: wasm.ContractModerationOptions): Promise<wasm.ContractModerationResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractUnsuspendUser(options);
  }

  /**
   * Warns an identity on a moderated contract: adds a warning, stamped with the block time,
   * to the warnings it carries on the contract's warning list. A warning bars nothing; the
   * warnings accumulate, at most 16 at a time, until they are cleared, and anyone can read
   * them. `options.reason` is required, as for a ban.
   */
  async warnUser(options: wasm.ContractWarnOptions): Promise<wasm.ContractModerationResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractWarnUser(options);
  }

  /** Takes an identity off a moderated contract's warning list: every warning it carries goes. */
  async clearUserWarnings(options: wasm.ContractModerationOptions): Promise<wasm.ContractModerationResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractClearUserWarnings(options);
  }

  /**
   * Deletes one document on a moderated contract as a moderator, whoever owns it, except the
   * contract owner and the moderators. The document type must set `canBeDeletedByModerators`;
   * when it also sets `canBeDeletedByModeratorsFor`, the deletion passes up to and including
   * that many seconds after the document's last modification, and is refused (41116) once
   * block time is later than that.
   * Signed like the other moderations. `options.reason` is optional here: left out, no code and
   * an empty text are stored. Resolves with the record the deletion left under the contract;
   * the document's owner gets no storage refund.
   */
  async moderatorDeleteDocument(
    options: wasm.ContractDeleteDocumentOptions,
  ): Promise<wasm.ContractDocumentRemovalResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractDeleteDocument(options);
  }

  /**
   * Restores, as a moderator, one document a moderator deleted: `options.document` is the
   * document as it was (as fetched before the deletion), which must hash to what its removal
   * record holds, and the restore must come within a week of the deletion (41120). Any current
   * moderator or the contract owner may restore, whoever deleted. The document goes back
   * through an ordinary insert, so a unique index value another document took meanwhile
   * refuses it (40105). Signed like the other moderations. Resolves with the record of the
   * deletion, now marked restored (`restoredBy`, `restoredAt`); the signer paid for the
   * document's storage, whose refund stays its owner's.
   */
  async moderatorRestoreDocument(
    options: wasm.ContractRestoreDocumentOptions,
  ): Promise<wasm.ContractDocumentRemovalResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractRestoreDocument(options);
  }

  /**
   * One identity's status on a moderated contract: whether it is banned, until when it is
   * suspended, and the warnings it carries. Every list named must be one the contract keeps.
   */
  async moderationStatus(query: wasm.ContractModerationStatusQuery): Promise<wasm.ContractModerationStatus> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractModerationStatus(query);
  }

  async moderationStatusWithProof(
    query: wasm.ContractModerationStatusQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ContractModerationStatus>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractModerationStatusWithProofInfo(query);
  }

  /**
   * One page of a moderated contract's banlist, suspension list or warning list, in identity
   * id order. Pass the page's `nextStartAfter` as the next query's `startAfter`; a page
   * without one (it holds fewer entries than the limit) is the last.
   */
  async moderationEntries(query: wasm.ContractModerationEntriesQuery): Promise<wasm.ContractModerationEntriesPage> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractModerationEntries(query);
  }

  async moderationEntriesWithProof(
    query: wasm.ContractModerationEntriesQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ContractModerationEntriesPage>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractModerationEntriesWithProofInfo(query);
  }

  /**
   * The records of the documents a contract's moderators deleted within one document type, in
   * document id order: the records of the `documentIds` named, where a document with no record
   * is left out, or else one page of them all. Pass a page's `nextStartAfter` as the next
   * query's `startAfter`; a page without one (it holds fewer records than the limit) is the last.
   */
  async documentRemovals(query: wasm.ContractDocumentRemovalsQuery): Promise<wasm.ContractDocumentRemovalsPage> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractDocumentRemovals(query);
  }

  async documentRemovalsWithProof(
    query: wasm.ContractDocumentRemovalsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ContractDocumentRemovalsPage>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractDocumentRemovalsWithProofInfo(query);
  }

  /**
   * What the document action fees of a contract (the `actionFees` keyword, protocol version
   * 14) have collected for its owner and for its moderation team, and the last claim of each
   * pot: the epoch and the block time it was paid out in, and the identity that claimed it. A
   * contract that charges no fees has two empty pots.
   */
  async feePots(contractId: wasm.IdentifierLike): Promise<wasm.ContractFeePots> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractFeePots(contractId);
  }

  async feePotsWithProof(
    contractId: wasm.IdentifierLike,
  ): Promise<wasm.ProofMetadataResponseTyped<wasm.ContractFeePots>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getContractFeePotsWithProofInfo(contractId);
  }

  /**
   * Pays out a fee pot of a contract: the `owner` pot whole to the contract owner, who alone
   * may claim it, and the `moderators` pot in equal shares to the contract's moderation team
   * (by the proposal's reward split for an elected contract's seated team), any member of
   * which may claim it for all of them. Signed with a CRITICAL authentication key. A pot is
   * paid out at most once per epoch, and an empty pot refuses the claim. The result proves
   * the balance of every recipient the contract names, or of the claimant alone for a seated
   * elected team, which the contract does not name.
   */
  async claimFees(options: wasm.ContractClaimFeesOptions): Promise<wasm.ContractClaimFeesResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractClaimFees(options);
  }
}
