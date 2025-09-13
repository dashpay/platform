import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class ProtocolFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  versionUpgradeState(): Promise<any> { return wasm.get_protocol_version_upgrade_state(this.sdk.wasm); }
  versionUpgradeStateWithProof(): Promise<any> { return wasm.get_protocol_version_upgrade_state_with_proof_info(this.sdk.wasm); }

  versionUpgradeVoteStatus(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    return wasm.get_protocol_version_upgrade_vote_status(this.sdk.wasm, startProTxHash, count);
  }

  versionUpgradeVoteStatusWithProof(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    return wasm.get_protocol_version_upgrade_vote_status_with_proof_info(this.sdk.wasm, startProTxHash, count);
  }
}

