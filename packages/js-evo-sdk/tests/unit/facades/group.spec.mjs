import { EvoSDK } from '../../../dist/sdk.js';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';

describe('GroupFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function () {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getContestedResources').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourcesWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceVotersForIdentity').resolves('ok');
    this.sinon.stub(wasmSdk, 'getContestedResourceVotersForIdentityWithProofInfo').resolves('ok');
  });

  it('forwards contestedResources and voters queries', async () => {
    await client.group.contestedResources({ documentTypeName: 'dt', contractId: 'c', indexName: 'i', startAtValue: new Uint8Array([1]), limit: 2, orderAscending: false });
    await client.group.contestedResourcesWithProof({ documentTypeName: 'dt', contractId: 'c', indexName: 'i' });
    await client.group.contestedResourceVotersForIdentity({ contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v1'], contestantId: 'id', startAtVoterInfo: 's', limit: 3, orderAscending: true });
    await client.group.contestedResourceVotersForIdentityWithProof({ contractId: 'c', documentTypeName: 'dt', indexName: 'i', indexValues: ['v2'], contestantId: 'id' });
    expect(wasmSdk.getContestedResources).to.be.calledOnce();
    expect(wasmSdk.getContestedResourcesWithProofInfo).to.be.calledOnce();
    expect(wasmSdk.getContestedResourceVotersForIdentity).to.be.calledOnce();
    expect(wasmSdk.getContestedResourceVotersForIdentityWithProofInfo).to.be.calledOnce();
  });
});
