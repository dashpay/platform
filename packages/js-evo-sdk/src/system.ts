import * as wasm from './wasm';

export class SystemFacade {
  private _sdk: wasm.WasmSdk;
  constructor(sdk: wasm.WasmSdk) { this._sdk = sdk; }

  status(): Promise<any> { return wasm.get_status(this._sdk); }
  currentQuorumsInfo(): Promise<any> { return wasm.get_current_quorums_info(this._sdk); }
  totalCreditsInPlatform(): Promise<any> { return wasm.get_total_credits_in_platform(this._sdk); }
  totalCreditsInPlatformWithProof(): Promise<any> { return wasm.get_total_credits_in_platform_with_proof_info(this._sdk); }
  prefundedSpecializedBalance(identityId: string): Promise<any> { return wasm.get_prefunded_specialized_balance(this._sdk, identityId); }
  prefundedSpecializedBalanceWithProof(identityId: string): Promise<any> { return wasm.get_prefunded_specialized_balance_with_proof_info(this._sdk, identityId); }
  waitForStateTransitionResult(stateTransitionHash: string): Promise<any> { return wasm.wait_for_state_transition_result(this._sdk, stateTransitionHash); }
  pathElements(path: string[], keys: string[]): Promise<any> { return wasm.get_path_elements(this._sdk, path, keys); }
  pathElementsWithProof(path: string[], keys: string[]): Promise<any> { return wasm.get_path_elements_with_proof_info(this._sdk, path, keys); }
}
