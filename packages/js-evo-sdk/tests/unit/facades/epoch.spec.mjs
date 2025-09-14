import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';

describe('EpochFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function () {
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

  it('epochsInfo and finalizedInfos forward with null handling', async () => {
    await client.epoch.epochsInfo({ startEpoch: 1, count: 2, ascending: true });
    await client.epoch.epochsInfoWithProof({});
    await client.epoch.finalizedInfos({ startEpoch: 3 });
    await client.epoch.finalizedInfosWithProof({ count: 4 });
    expect(wasmSdk.getEpochsInfo).to.be.calledOnceWithExactly(1, 2, true);
    expect(wasmSdk.getEpochsInfoWithProofInfo).to.be.calledOnceWithExactly(null, null, null);
    expect(wasmSdk.getFinalizedEpochInfos).to.be.calledOnceWithExactly(3, null, null);
    expect(wasmSdk.getFinalizedEpochInfosWithProofInfo).to.be.calledOnceWithExactly(null, 4, null);
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
    await client.epoch.evonodesProposedBlocksByRange(12, { limit: 2, startAfter: 's', orderAscending: false });
    await client.epoch.evonodesProposedBlocksByRangeWithProof(13, {});
    expect(wasmSdk.getEvonodesProposedEpochBlocksByIds).to.be.calledOnceWithExactly(10, ['a', 'b']);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByIdsWithProofInfo).to.be.calledOnceWithExactly(11, ['x']);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByRange).to.be.calledOnceWithExactly(12, 2, 's', false);
    expect(wasmSdk.getEvonodesProposedEpochBlocksByRangeWithProofInfo).to.be.calledOnceWithExactly(13, null, null, null);
  });
});
