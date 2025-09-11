import * as wasm from './wasm';

export class EpochFacade {
  private _sdk: wasm.WasmSdk;
  constructor(sdk: wasm.WasmSdk) { this._sdk = sdk; }

  epochsInfo(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_epochs_info(this._sdk, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  epochsInfoWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_epochs_info_with_proof_info(this._sdk, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  finalizedInfos(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_finalized_epoch_infos(this._sdk, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  finalizedInfosWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_finalized_epoch_infos_with_proof_info(this._sdk, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  current(): Promise<any> { return wasm.get_current_epoch(this._sdk); }
  currentWithProof(): Promise<any> { return wasm.get_current_epoch_with_proof_info(this._sdk); }

  evonodesProposedBlocksByIds(epoch: number, ids: string[]): Promise<any> {
    return wasm.get_evonodes_proposed_epoch_blocks_by_ids(this._sdk, epoch, ids);
  }

  evonodesProposedBlocksByIdsWithProof(epoch: number, ids: string[]): Promise<any> {
    return wasm.get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info(this._sdk, epoch, ids);
  }

  evonodesProposedBlocksByRange(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    return wasm.get_evonodes_proposed_epoch_blocks_by_range(this._sdk, epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }

  evonodesProposedBlocksByRangeWithProof(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    return wasm.get_evonodes_proposed_epoch_blocks_by_range_with_proof_info(this._sdk, epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }
}
