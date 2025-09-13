import * as wasm from '../wasm.js';
import { asJsonString } from '../util.js';
import type { EvoSDK } from '../sdk.js';

export class ContractsFacade {
  private sdk: EvoSDK;

  constructor(sdk: EvoSDK) {
    this.sdk = sdk;
  }

  fetch(contractId: string): Promise<wasm.DataContractWasm> {
    return wasm.data_contract_fetch(this.sdk.wasm, contractId);
  }

  fetchWithProof(contractId: string): Promise<any> {
    return wasm.data_contract_fetch_with_proof_info(this.sdk.wasm, contractId);
  }

  getHistory(args: { contractId: string; limit?: number; startAtMs?: number | bigint }): Promise<any> {
    const { contractId, limit, startAtMs } = args;
    return wasm.get_data_contract_history(
      this.sdk.wasm,
      contractId,
      limit ?? null,
      null,
      startAtMs != null ? BigInt(startAtMs) : null,
    );
  }

  getHistoryWithProof(args: { contractId: string; limit?: number; startAtMs?: number | bigint }): Promise<any> {
    const { contractId, limit, startAtMs } = args;
    return wasm.get_data_contract_history_with_proof_info(
      this.sdk.wasm,
      contractId,
      limit ?? null,
      null,
      startAtMs != null ? BigInt(startAtMs) : null,
    );
  }

  getMany(contractIds: string[]): Promise<any> {
    return wasm.get_data_contracts(this.sdk.wasm, contractIds);
  }

  getManyWithProof(contractIds: string[]): Promise<any> {
    return wasm.get_data_contracts_with_proof_info(this.sdk.wasm, contractIds);
  }

  create(args: { ownerId: string; definition: unknown; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { ownerId, definition, privateKeyWif, keyId } = args;
    return this.sdk.wasm.contractCreate(ownerId, asJsonString(definition)!, privateKeyWif, keyId ?? null);
  }

  update(args: { contractId: string; ownerId: string; updates: unknown; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { contractId, ownerId, updates, privateKeyWif, keyId } = args;
    return this.sdk.wasm.contractUpdate(contractId, ownerId, asJsonString(updates)!, privateKeyWif, keyId ?? null);
  }
}

