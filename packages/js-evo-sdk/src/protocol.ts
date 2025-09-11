import * as wasm from './wasm';

export class ProtocolFacade {
  private _sdk: wasm.WasmSdk;
  constructor(sdk: wasm.WasmSdk) { this._sdk = sdk; }

  versionUpgradeState(): Promise<any> { return wasm.get_protocol_version_upgrade_state(this._sdk); }
  versionUpgradeStateWithProof(): Promise<any> { return wasm.get_protocol_version_upgrade_state_with_proof_info(this._sdk); }

  versionUpgradeVoteStatus(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    return wasm.get_protocol_version_upgrade_vote_status(this._sdk, startProTxHash, count);
  }

  versionUpgradeVoteStatusWithProof(params: { startProTxHash: string; count: number }): Promise<any> {
    const { startProTxHash, count } = params;
    return wasm.get_protocol_version_upgrade_vote_status_with_proof_info(this._sdk, startProTxHash, count);
  }
}
