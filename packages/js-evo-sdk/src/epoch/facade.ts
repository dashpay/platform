import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class EpochFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  epochsInfo(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_epochs_info(this.sdk.wasm, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  epochsInfoWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_epochs_info_with_proof_info(this.sdk.wasm, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  finalizedInfos(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_finalized_epoch_infos(this.sdk.wasm, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  finalizedInfosWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    return wasm.get_finalized_epoch_infos_with_proof_info(this.sdk.wasm, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  current(): Promise<any> { return wasm.get_current_epoch(this.sdk.wasm); }
  currentWithProof(): Promise<any> { return wasm.get_current_epoch_with_proof_info(this.sdk.wasm); }

  evonodesProposedBlocksByIds(epoch: number, ids: string[]): Promise<any> {
    return wasm.get_evonodes_proposed_epoch_blocks_by_ids(this.sdk.wasm, epoch, ids);
  }

  evonodesProposedBlocksByIdsWithProof(epoch: number, ids: string[]): Promise<any> {
    return wasm.get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info(this.sdk.wasm, epoch, ids);
  }

  evonodesProposedBlocksByRange(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    return wasm.get_evonodes_proposed_epoch_blocks_by_range(this.sdk.wasm, epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }

  evonodesProposedBlocksByRangeWithProof(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    return wasm.get_evonodes_proposed_epoch_blocks_by_range_with_proof_info(this.sdk.wasm, epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }
}

