import { EvoSDK } from '../../../dist/evo-sdk.module.js';

const isBrowser = typeof window !== 'undefined';

describe('ProtocolFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('versionUpgradeState and versionUpgradeStateWithProof forward', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.protocol.versionUpgradeState();
    await sdk.protocol.versionUpgradeStateWithProof();
    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include('get_protocol_version_upgrade_state');
    expect(names).to.include('get_protocol_version_upgrade_state_with_proof_info');
  });

  it('versionUpgradeVoteStatus and withProof forward with args', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.protocol.versionUpgradeVoteStatus({ startProTxHash: 'h', count: 5 });
    await sdk.protocol.versionUpgradeVoteStatusWithProof({ startProTxHash: 'g', count: 3 });
    const calls = wasmStubModule.__getCalls();
    const a = calls.find(c => c.called === 'get_protocol_version_upgrade_vote_status');
    const b = calls.find(c => c.called === 'get_protocol_version_upgrade_vote_status_with_proof_info');
    expect(a.args).to.deep.equal([raw, 'h', 5]);
    expect(b.args).to.deep.equal([raw, 'g', 3]);
  });
});

