import type { EvoSDK } from '../sdk.js';

export class EpochFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async epochsInfo(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEpochsInfo(startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async epochsInfoWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEpochsInfoWithProofInfo(startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async finalizedInfos(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getFinalizedEpochInfos(startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async finalizedInfosWithProof(params: { startEpoch?: number; count?: number; ascending?: boolean } = {}): Promise<any> {
    const { startEpoch, count, ascending } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getFinalizedEpochInfosWithProofInfo(startEpoch ?? null, count ?? null, ascending ?? null);
  }

  async current(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getCurrentEpoch(); }
  async currentWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getCurrentEpochWithProofInfo(); }

  async evonodesProposedBlocksByIds(epoch: number, ids: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByIds(epoch, ids);
  }

  async evonodesProposedBlocksByIdsWithProof(epoch: number, ids: string[]): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByIdsWithProofInfo(epoch, ids);
  }

  async evonodesProposedBlocksByRange(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByRange(epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }

  async evonodesProposedBlocksByRangeWithProof(epoch: number, opts: { limit?: number; startAfter?: string; orderAscending?: boolean } = {}): Promise<any> {
    const { limit, startAfter, orderAscending } = opts;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByRangeWithProofInfo(epoch, limit ?? null, startAfter ?? null, orderAscending ?? null);
  }
}
