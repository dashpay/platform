import * as wasm from './wasm';
import { asJsonString } from './util';

export class ContractsFacade {
  private _sdk: wasm.WasmSdk;

  constructor(sdk: wasm.WasmSdk) {
    this._sdk = sdk;
  }

  fetch(contractId: string): Promise<wasm.DataContractWasm> {
    return wasm.data_contract_fetch(this._sdk, contractId);
  }

  fetchWithProof(contractId: string): Promise<any> {
    return wasm.data_contract_fetch_with_proof_info(this._sdk, contractId);
  }

  getHistory(args: { contractId: string; limit?: number; startAtMs?: number | bigint }): Promise<any> {
    const { contractId, limit, startAtMs } = args;
    return wasm.get_data_contract_history(this._sdk, contractId, limit ?? null, null, startAtMs ?? null);
  }

  getHistoryWithProof(args: { contractId: string; limit?: number; startAtMs?: number | bigint }): Promise<any> {
    const { contractId, limit, startAtMs } = args;
    return wasm.get_data_contract_history_with_proof_info(this._sdk, contractId, limit ?? null, null, startAtMs ?? null);
  }

  getMany(contractIds: string[]): Promise<any> {
    return wasm.get_data_contracts(this._sdk, contractIds);
  }

  getManyWithProof(contractIds: string[]): Promise<any> {
    return wasm.get_data_contracts_with_proof_info(this._sdk, contractIds);
  }

  create(args: { ownerId: string; definition: unknown; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { ownerId, definition, privateKeyWif, keyId } = args;
    return this._sdk.contractCreate(ownerId, asJsonString(definition)!, privateKeyWif, keyId ?? null);
  }

  update(args: { contractId: string; ownerId: string; updates: unknown; privateKeyWif: string; keyId?: number }): Promise<any> {
    const { contractId, ownerId, updates, privateKeyWif, keyId } = args;
    return this._sdk.contractUpdate(contractId, ownerId, asJsonString(updates)!, privateKeyWif, keyId ?? null);
  }
}
