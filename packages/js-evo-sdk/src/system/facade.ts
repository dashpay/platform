import * as wasm from '../wasm.js';
import type { EvoSDK } from '../sdk.js';

export class SystemFacade {
  private sdk: EvoSDK;
  constructor(sdk: EvoSDK) { this.sdk = sdk; }

  status(): Promise<any> { return wasm.get_status(this.sdk.wasm); }
  currentQuorumsInfo(): Promise<any> { return wasm.get_current_quorums_info(this.sdk.wasm); }
  totalCreditsInPlatform(): Promise<any> { return wasm.get_total_credits_in_platform(this.sdk.wasm); }
  totalCreditsInPlatformWithProof(): Promise<any> { return wasm.get_total_credits_in_platform_with_proof_info(this.sdk.wasm); }
  prefundedSpecializedBalance(identityId: string): Promise<any> { return wasm.get_prefunded_specialized_balance(this.sdk.wasm, identityId); }
  prefundedSpecializedBalanceWithProof(identityId: string): Promise<any> { return wasm.get_prefunded_specialized_balance_with_proof_info(this.sdk.wasm, identityId); }
  waitForStateTransitionResult(stateTransitionHash: string): Promise<any> { return wasm.wait_for_state_transition_result(this.sdk.wasm, stateTransitionHash); }
  pathElements(path: string[], keys: string[]): Promise<any> { return wasm.get_path_elements(this.sdk.wasm, path, keys); }
  pathElementsWithProof(path: string[], keys: string[]): Promise<any> { return wasm.get_path_elements_with_proof_info(this.sdk.wasm, path, keys); }
}

