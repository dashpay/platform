import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class EpochFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async epochsInfo(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_epochs_info(w, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async epochsInfoWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_epochs_info_with_proof_info(w, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async finalizedInfos(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_finalized_epoch_infos(w, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async finalizedInfosWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_finalized_epoch_infos_with_proof_info(w, startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async current(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_current_epoch(w); }
  async currentWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_current_epoch_with_proof_info(w); }

  async evonodesProposedBlocksByIds(epoch: number, ids: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_evonodes_proposed_epoch_blocks_by_ids(w, epoch, ids);
  }

  async evonodesProposedBlocksByIdsWithProof(epoch: number, ids: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info(w, epoch, ids);
  }

  async evonodesProposedBlocksByRange(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_evonodes_proposed_epoch_blocks_by_range(w, epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }

  async evonodesProposedBlocksByRangeWithProof(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_evonodes_proposed_epoch_blocks_by_range_with_proof_info(w, epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }
}
