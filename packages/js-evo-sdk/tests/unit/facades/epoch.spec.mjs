import { EvoSDK } from '../../../dist/evo-sdk.module.js';

const isBrowser = typeof window !== 'undefined';

describe('EpochFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('epochsInfo and finalizedInfos forward with null handling', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.epoch.epochsInfo({ startEpoch: 1, count: 2, ascending: true });
    await sdk.epoch.epochsInfoWithProof({});
    await sdk.epoch.finalizedInfos({ startEpoch: 3 });
    await sdk.epoch.finalizedInfosWithProof({ count: 4 });

    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include.members([
      'get_epochs_info',
      'get_epochs_info_with_proof_info',
      'get_finalized_epoch_infos',
      'get_finalized_epoch_infos_with_proof_info',
    ]);
  });

  it('current and currentWithProof forward', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.epoch.current();
    await sdk.epoch.currentWithProof();
    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include('get_current_epoch');
    expect(names).to.include('get_current_epoch_with_proof_info');
  });

  it('evonodesProposedBlocks* forward with args', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.epoch.evonodesProposedBlocksByIds(10, ['a', 'b']);
    await sdk.epoch.evonodesProposedBlocksByIdsWithProof(11, ['x']);
    await sdk.epoch.evonodesProposedBlocksByRange(12, { limit: 2, startAfter: 's', orderAscending: false });
    await sdk.epoch.evonodesProposedBlocksByRangeWithProof(13, {});
    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include.members([
      'get_evonodes_proposed_epoch_blocks_by_ids',
      'get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info',
      'get_evonodes_proposed_epoch_blocks_by_range',
      'get_evonodes_proposed_epoch_blocks_by_range_with_proof_info',
    ]);
  });
});

