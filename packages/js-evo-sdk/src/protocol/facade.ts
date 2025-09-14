import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class ProtocolFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async versionUpgradeState(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_protocol_version_upgrade_state(w); }
  async versionUpgradeStateWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_protocol_version_upgrade_state_with_proof_info(w); }

  async versionUpgradeVoteStatus(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_protocol_version_upgrade_vote_status(w, startProTxHash, count);
  }

  async versionUpgradeVoteStatusWithProof(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    const w = await this.sdk.getWasmSdkConnected();
    return wasm.get_protocol_version_upgrade_vote_status_with_proof_info(w, startProTxHash, count);
  }
}
