import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class ProtocolFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async versionUpgradeState(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getProtocolVersionUpgradeState(); }
  async versionUpgradeStateWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getProtocolVersionUpgradeStateWithProofInfo(); }

  async versionUpgradeVoteStatus(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeVoteStatus(startProTxHash, count);
  }

  async versionUpgradeVoteStatusWithProof(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeVoteStatusWithProofInfo(startProTxHash, count);
  }
}
