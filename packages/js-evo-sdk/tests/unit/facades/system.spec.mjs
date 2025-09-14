import { EvoSDK } from '../../../dist/evo-sdk.module.js';

const isBrowser = typeof window !== 'undefined';

describe('SystemFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('forwards all methods to free functions', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.system.status();
    await sdk.system.currentQuorumsInfo();
    await sdk.system.totalCreditsInPlatform();
    await sdk.system.totalCreditsInPlatformWithProof();
    await sdk.system.prefundedSpecializedBalance('i');
    await sdk.system.prefundedSpecializedBalanceWithProof('i');
    await sdk.system.waitForStateTransitionResult('h');
    await sdk.system.pathElements(['p'], ['k']);
    await sdk.system.pathElementsWithProof(['p2'], ['k2']);
    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include.members([
      'get_status',
      'get_current_quorums_info',
      'get_total_credits_in_platform',
      'get_total_credits_in_platform_with_proof_info',
      'get_prefunded_specialized_balance',
      'get_prefunded_specialized_balance_with_proof_info',
      'wait_for_state_transition_result',
      'get_path_elements',
      'get_path_elements_with_proof_info',
    ]);
  });
});

