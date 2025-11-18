import type { EvoSDK } from '../sdk.js';
import type {
  EpochsQuery,
  FinalizedEpochsQuery,
  EvonodeProposedBlocksRangeQuery,
} from '../wasm.js';

export class EpochFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async epochsInfo(query: EpochsQuery = {}): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEpochsInfo(query);
  }

  async epochsInfoWithProof(query: EpochsQuery = {}): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEpochsInfoWithProofInfo(query);
  }

  async finalizedInfos(query: FinalizedEpochsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getFinalizedEpochInfos(query);
  }

  async finalizedInfosWithProof(query: FinalizedEpochsQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getFinalizedEpochInfosWithProofInfo(query);
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

  async evonodesProposedBlocksByRange(query: EvonodeProposedBlocksRangeQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByRange(query);
  }

  async evonodesProposedBlocksByRangeWithProof(query: EvonodeProposedBlocksRangeQuery): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByRangeWithProofInfo(query);
  }
}
