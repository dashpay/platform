import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('EpochFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  // Stub references for type-safe assertions
  let getEpochsInfoStub: SinonStub;
  let getEpochsInfoWithProofInfoStub: SinonStub;
  let getFinalizedEpochInfosStub: SinonStub;
  let getFinalizedEpochInfosWithProofInfoStub: SinonStub;
  let getCurrentEpochStub: SinonStub;
  let getCurrentEpochWithProofInfoStub: SinonStub;
  let getEvonodesProposedEpochBlocksByIdsStub: SinonStub;
  let getEvonodesProposedEpochBlocksByIdsWithProofInfoStub: SinonStub;
  let getEvonodesProposedEpochBlocksByRangeStub: SinonStub;
  let getEvonodesProposedEpochBlocksByRangeWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getEpochsInfoStub = this.sinon.stub(wasmSdk, 'getEpochsInfo').resolves('ok');
    getEpochsInfoWithProofInfoStub = this.sinon.stub(wasmSdk, 'getEpochsInfoWithProofInfo').resolves('ok');
    getFinalizedEpochInfosStub = this.sinon.stub(wasmSdk, 'getFinalizedEpochInfos').resolves('ok');
    getFinalizedEpochInfosWithProofInfoStub = this.sinon.stub(wasmSdk, 'getFinalizedEpochInfosWithProofInfo').resolves('ok');
    getCurrentEpochStub = this.sinon.stub(wasmSdk, 'getCurrentEpoch').resolves('ok');
    getCurrentEpochWithProofInfoStub = this.sinon.stub(wasmSdk, 'getCurrentEpochWithProofInfo').resolves('ok');
    getEvonodesProposedEpochBlocksByIdsStub = this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByIds').resolves('ok');
    getEvonodesProposedEpochBlocksByIdsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByIdsWithProofInfo').resolves('ok');
    getEvonodesProposedEpochBlocksByRangeStub = this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByRange').resolves('ok');
    getEvonodesProposedEpochBlocksByRangeWithProofInfoStub = this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByRangeWithProofInfo').resolves('ok');
  });

  it('should forward epochsInfo and finalizedInfos queries untouched', async () => {
    const epochsQuery = { startEpoch: 1, count: 2, ascending: true };
    await client.epoch.epochsInfo(epochsQuery);
    await client.epoch.epochsInfoWithProof();
    const finalizedQuery = { startEpoch: 3 };
    await client.epoch.finalizedInfos(finalizedQuery);
    const finalizedProofQuery = { startEpoch: 4, count: 5 };
    await client.epoch.finalizedInfosWithProof(finalizedProofQuery);
    expect(getEpochsInfoStub).to.be.calledOnceWithExactly(epochsQuery);
    expect(getEpochsInfoWithProofInfoStub).to.be.calledOnce();
    expect(getEpochsInfoWithProofInfoStub.firstCall.args[0]).to.deep.equal({});
    expect(getFinalizedEpochInfosStub).to.be.calledOnceWithExactly(finalizedQuery);
    expect(getFinalizedEpochInfosWithProofInfoStub)
      .to.be.calledOnceWithExactly(finalizedProofQuery);
  });

  it('should forward current and currentWithProof', async () => {
    await client.epoch.current();
    await client.epoch.currentWithProof();
    expect(getCurrentEpochStub).to.be.calledOnce();
    expect(getCurrentEpochWithProofInfoStub).to.be.calledOnce();
  });

  it('should forward evonodesProposedBlocks* with args', async () => {
    await client.epoch.evonodesProposedBlocksByIds(10, ['a', 'b']);
    await client.epoch.evonodesProposedBlocksByIdsWithProof(11, ['x']);
    const rangeQuery = {
      epoch: 12, limit: 2, startAfter: 's', orderAscending: false,
    };
    await client.epoch.evonodesProposedBlocksByRange(rangeQuery);
    const rangeProofQuery = { epoch: 13 };
    await client.epoch.evonodesProposedBlocksByRangeWithProof(rangeProofQuery);
    expect(getEvonodesProposedEpochBlocksByIdsStub).to.be.calledOnceWithExactly(10, ['a', 'b']);
    expect(getEvonodesProposedEpochBlocksByIdsWithProofInfoStub).to.be.calledOnceWithExactly(11, ['x']);
    expect(getEvonodesProposedEpochBlocksByRangeStub).to.be.calledOnceWithExactly(rangeQuery);
    expect(getEvonodesProposedEpochBlocksByRangeWithProofInfoStub)
      .to.be.calledOnceWithExactly(rangeProofQuery);
  });
});
