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

  it('epochsInfo and finalizedInfos forward with null handling', async () => {
    await client.epoch.epochsInfo({ startEpoch: 1, count: 2, ascending: true });
    await client.epoch.epochsInfoWithProof({});
    await client.epoch.finalizedInfos({ startEpoch: 3 });
    await client.epoch.finalizedInfosWithProof({ startEpoch: 4, count: 5 });
    expect(wasmSdk.getEpochsInfo).to.be.calledOnceWithExactly({
      startEpoch: 1,
      count: 2,
      ascending: true,
    });
    expect(wasmSdk.getEpochsInfoWithProofInfo).to.be.calledOnceWithExactly({
      startEpoch: undefined,
      count: undefined,
      ascending: undefined,
    });
    expect(wasmSdk.getFinalizedEpochInfos).to.be.calledOnceWithExactly({
      startEpoch: 3,
      count: undefined,
      ascending: undefined,
    });
    expect(wasmSdk.getFinalizedEpochInfosWithProofInfo).to.be.calledOnceWithExactly({
      startEpoch: 4,
      count: 5,
      ascending: undefined,
    });
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
    expect(wasmSdk.getEvonodesProposedEpochBlocksByRange).to.be.calledOnceWithExactly({
      epoch: 12,
      limit: 2,
      startAfter: 's',
      orderAscending: false,
    });
    expect(wasmSdk.getEvonodesProposedEpochBlocksByRangeWithProofInfo)
      .to.be.calledOnceWithExactly({
        epoch: 13,
        limit: undefined,
        startAfter: undefined,
        orderAscending: undefined,
      });
  });
});
