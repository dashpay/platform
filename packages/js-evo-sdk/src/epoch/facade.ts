import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';
import type {
  EpochsQuery,
  FinalizedEpochsQuery,
  EvonodeProposedBlocksRangeQuery,
} from '../wasm.js';

export class EpochFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async epochsInfo(query: EpochsQuery = {}): Promise<Map<number, wasm.ExtendedEpochInfo | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEpochsInfo(query);
  }

  async epochsInfoWithProof(
    query: EpochsQuery = {},
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<number, wasm.ExtendedEpochInfo | undefined>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEpochsInfoWithProofInfo(query);
  }

  async finalizedInfos(query: FinalizedEpochsQuery): Promise<Map<number, wasm.FinalizedEpochInfo | undefined>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getFinalizedEpochInfos(query);
  }

  async finalizedInfosWithProof(
    query: FinalizedEpochsQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<
    Map<number, wasm.FinalizedEpochInfo | undefined>
  >> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getFinalizedEpochInfosWithProofInfo(query);
  }

  async current(): Promise<wasm.ExtendedEpochInfo> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getCurrentEpoch();
  }
  async currentWithProof(): Promise<wasm.ProofMetadataResponseTyped<wasm.ExtendedEpochInfo>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getCurrentEpochWithProofInfo();
  }

  async evonodesProposedBlocksByIds(epoch: number, ids: wasm.ProTxHashLikeArray):
    Promise<Map<string, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByIds(epoch, ids);
  }

  async evonodesProposedBlocksByIdsWithProof(epoch: number, ids: wasm.ProTxHashLikeArray):
    Promise<wasm.ProofMetadataResponseTyped<Map<string, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByIdsWithProofInfo(epoch, ids);
  }

  async evonodesProposedBlocksByRange(query: EvonodeProposedBlocksRangeQuery): Promise<Map<string, bigint>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByRange(query);
  }

  async evonodesProposedBlocksByRangeWithProof(
    query: EvonodeProposedBlocksRangeQuery,
  ): Promise<wasm.ProofMetadataResponseTyped<Map<string, bigint>>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getEvonodesProposedEpochBlocksByRangeWithProofInfo(query);
  }
}
