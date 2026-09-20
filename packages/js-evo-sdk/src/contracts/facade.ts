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
   * One identity's status on a moderated contract: whether it is banned, and until when it is
   * suspended. Every list named must be one the contract keeps.
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
   * One page of a moderated contract's banlist or suspension list, in identity id order. Pass
   * the page's `nextStartAfter` as the next query's `startAfter`; a page without one (it holds fewer entries than the limit) is the last.
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
   * What the document action fees of a contract (the `actionFees` keyword, protocol version
   * 14) have collected for its owner and for its moderation team, and the epoch each pot was
   * last paid out in. A contract that charges no fees has two empty pots.
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
   * may claim it, and the `moderators` pot in equal shares to the contract's moderation team,
   * any member of which may claim it for all of them. Signed with a CRITICAL authentication
   * key. A pot is paid out at most once per epoch, and an empty pot refuses the claim.
   */
  async claimFees(options: wasm.ContractClaimFeesOptions): Promise<wasm.ContractClaimFeesResult> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractClaimFees(options);
  }
}
