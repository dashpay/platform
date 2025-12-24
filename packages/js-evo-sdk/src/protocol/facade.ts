import type { EvoSDK } from '../sdk.js';
import * as wasm from '../wasm.js';

export class ProtocolFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async versionUpgradeState(): Promise<wasm.ProtocolVersionUpgradeState> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeState();
  }
  async versionUpgradeStateWithProof(): Promise<wasm.ProofMetadataResponseTyped<wasm.ProtocolVersionUpgradeState>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeStateWithProofInfo();
  }

  async versionUpgradeVoteStatus(startProTxHash: string | Uint8Array, count: number): Promise<Map<string, wasm.ProtocolVersionUpgradeVoteStatus>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeVoteStatus(startProTxHash, count);
  }

  async versionUpgradeVoteStatusWithProof(startProTxHash: string | Uint8Array, count: number): Promise<wasm.ProofMetadataResponseTyped<unknown>> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeVoteStatusWithProofInfo(startProTxHash, count);
  }
}
