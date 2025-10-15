import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class ContractsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  async fetch(contractId: string): Promise<wasm.DataContract> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContract(contractId);
  }

  async fetchWithProof(contractId: string): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractWithProofInfo(contractId);
  }

  async getHistory(args: { contractId: string; limit?: number; startAtMs?: number | bigint }): Promise<any> {
    const { contractId, limit, startAtMs } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractHistory(
      contractId,
      limit ?? null,
      null,
      startAtMs != null ? BigInt(startAtMs) : null,
    );
  }

  async getHistoryWithProof(args: { contractId: string; limit?: number; startAtMs?: number | bigint }): Promise<any> {
    const { contractId, limit, startAtMs } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractHistoryWithProofInfo(
      contractId,
      limit ?? null,
      null,
      startAtMs != null ? BigInt(startAtMs) : null,
    );
  }

  async getMany(contractIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContracts(contractIds);
  }

  async getManyWithProof(contractIds: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getDataContractsWithProofInfo(contractIds);
  }

  async create(args: { ownerId: string; definition: unknown; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { ownerId, definition, privateKeyWif, keyId } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractCreate(ownerId, asJsonString(definition)!, privateKeyWif, keyId ?? null);
  }

  async update(args: { contractId: string; ownerId: string; updates: unknown; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { contractId, ownerId, updates, privateKeyWif, keyId } = args;
    const w = await this.sdk.getWasmSdkConnected();
    return w.contractUpdate(contractId, ownerId, asJsonString(updates)!, privateKeyWif, keyId ?? null);
  }
}
