import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class SystemFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  async status(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_status(w); }
  async currentQuorumsInfo(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_current_quorums_info(w); }
  async totalCreditsInPlatform(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_total_credits_in_platform(w); }
  async totalCreditsInPlatformWithProof(): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_total_credits_in_platform_with_proof_info(w); }
  async prefundedSpecializedBalance(identityId: string): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_prefunded_specialized_balance(w, identityId); }
  async prefundedSpecializedBalanceWithProof(identityId: string): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_prefunded_specialized_balance_with_proof_info(w, identityId); }
  async waitForStateTransitionResult(stateTransitionHash: string): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.wait_for_state_transition_result(w, stateTransitionHash); }
  async pathElements(path: string[], keys: string[]): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_path_elements(w, path, keys); }
  async pathElementsWithProof(path: string[], keys: string[]): Promise<any> { const w = await this.sdk.getWasmSdkConnected(); return wasm.get_path_elements_with_proof_info(w, path, keys); }
}
