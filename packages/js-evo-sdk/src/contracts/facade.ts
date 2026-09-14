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
}
