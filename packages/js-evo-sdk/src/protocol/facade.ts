import type { EvoSDK } from '../sdk.js';

export class ProtocolFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async versionUpgradeState(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getProtocolVersionUpgradeState(); }
  async versionUpgradeStateWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return w.getProtocolVersionUpgradeStateWithProofInfo(); }

  async versionUpgradeVoteStatus(startProTxHash: string | Uint8Array, count: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeVoteStatus(startProTxHash as any, count);
  }

  async versionUpgradeVoteStatusWithProof(startProTxHash: string | Uint8Array, count: number): Promise<any> {
    const w = await this.sdk.getWasmSdkConnected();
    return w.getProtocolVersionUpgradeVoteStatusWithProofInfo(startProTxHash as any, count);
  }
}
