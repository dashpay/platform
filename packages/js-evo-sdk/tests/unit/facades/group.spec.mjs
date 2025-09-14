import { EvoSDK } from '../../../dist/evo-sdk.module.js';

const isBrowser = typeof window !== 'undefined';

describe('GroupFacade', () => {
  if (!isBrowser) {
    it('skips in Node environment (browser-only)', function () { this.skip(); });
    return;
  }

  let wasmStubModule;
  before(async () => { wasmStubModule = await import('@dashevo/wasm-sdk'); });
  beforeEach(() => { wasmStubModule.__clearCalls(); });

  it('forwards contestedResources and voters queries', async () => {
    const raw = {};
    const sdk = EvoSDK.fromWasm(raw);
    await sdk.group.contestedResources({ documentTypeName: 'dt', contractId: 'c', indexName: 'i', startAtValue: new Uint8Array([1]), limit: 2, orderAscending: false });
    await sdk.group.contestedResourcesWithProof({ documentTypeName: 'dt', contractId: 'c', indexName: 'i' });
    await sdk.group.contestedResourceVotersForIdentity({ contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], contestantId: 'id', startAtVoterInfo: 's', limit: 3, orderAscending: true });
    await sdk.group.contestedResourceVotersForIdentityWithProof({ contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v2'], contestantId: 'id' });
    const names = wasmStubModule.__getCalls().map(c => c.called);
    expect(names).to.include.members([
      'get_contested_resources',
      'get_contested_resources_with_proof_info',
      'get_contested_resource_voters_for_identity',
      'get_contested_resource_voters_for_identity_with_proof_info',
    ]);
  });
});

