import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('EpochFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getEpochsInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getEpochsInfoWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getFinalizedEpochInfos').resolves('ok');
    this.sinon.stub(wasmSdk, 'getFinalizedEpochInfosWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getCurrentEpoch').resolves('ok');
    this.sinon.stub(wasmSdk, 'getCurrentEpochWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByIds').resolves('ok');
    this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByIdsWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByRange').resolves('ok');
    this.sinon.stub(wasmSdk, 'getEvonodesProposedEpochBlocksByRangeWithProofInfo').resolves('ok');
  });

  it('epochsInfo and finalizedInfos forward queries untouched', async () => {
    const epochsQuery = { startEpoch: 1, count: 2, ascending: true };
    await client.epoch.epochsInfo(epochsQuery);
    await client.epoch.epochsInfoWithProof();
    const finalizedQuery = { startEpoch: 3 };
    await client.epoch.finalizedInfos(finalizedQuery);
    const finalizedProofQuery = { startEpoch: 4, count: 5 };
    await client.epoch.finalizedInfosWithProof(finalizedProofQuery);
    expect(wasmSdk.getEpochsInfo).to.be.calledOnceWithExactly(epochsQuery);
    expect(wasmSdk.getEpochsInfoWithProofInfo).to.be.calledOnce();
    expect(wasmSdk.getEpochsInfoWithProofInfo.firstCall.args[0]).to.deep.equal({});
    expect(wasmSdk.getFinalizedEpochInfos).to.be.calledOnceWithExactly(finalizedQuery);
    expect(wasmSdk.getFinalizedEpochInfosWithProofInfo)
      .to.be.calledOnceWithExactly(finalizedProofQuery);
  });

  it('current and currentWithProof forward', async () => {
    await client.epoch.current();
    await client.epoch.currentWithProof();
    expect(wasmSdk.getCurrentEpoch).to.be.calledOnce();
    expect(wasmSdk.getCurrentEpochWithProofInfo).to.be.calledOnce();
  });

  it('evonodesProposedBlocks* forward with args', async () => {
    await client.epoch.evonodesProposedBlocksByIds(10, ['a', 'b']);
    await client.epoch.evonodesProposedBlocksByIdsWithProof(11, ['x']);
    const rangeQuery = {
      epoch: 12, limit: 2, startAfter: 's', orderAscending: false,
    };
    await client.epoch.evonodesProposedBlocksByRange(rangeQuery);
    const rangeProofQuery = { epoch: 13 };
    await client.epoch.evonodesProposedBlocksByRangeWithProof(rangeProofQuery);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByIds).to.be.calledOnceWithExactly(10, ['a', 'b']);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByIdsWithProofInfo).to.be.calledOnceWithExactly(11, ['x']);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByRange).to.be.calledOnceWithExactly(rangeQuery);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByRangeWithProofInfo)
      .to.be.calledOnceWithExactly(rangeProofQuery);
  });
});
