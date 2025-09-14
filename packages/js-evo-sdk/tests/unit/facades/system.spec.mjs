import { EvoSDK } from '../../../dist/evo-sdk.module.js';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';

describe('SystemFacade', () => {
  let wasmSdk;
  let client;

  beforeEach(async function () {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    this.sinon.stub(wasmSdk, 'getStatus').resolves('ok');
    this.sinon.stub(wasmSdk, 'getCurrentQuorumsInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTotalCreditsInPlatform').resolves('ok');
    this.sinon.stub(wasmSdk, 'getTotalCreditsInPlatformWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'getPrefundedSpecializedBalance').resolves('ok');
    this.sinon.stub(wasmSdk, 'getPrefundedSpecializedBalanceWithProofInfo').resolves('ok');
    this.sinon.stub(wasmSdk, 'waitForStateTransitionResult').resolves('ok');
    this.sinon.stub(wasmSdk, 'getPathElements').resolves('ok');
    this.sinon.stub(wasmSdk, 'getPathElementsWithProofInfo').resolves('ok');
  });

  it('forwards all methods to instance methods', async () => {
    await client.system.status();
    await client.system.currentQuorumsInfo();
    await client.system.totalCreditsInPlatform();
    await client.system.totalCreditsInPlatformWithProof();
    await client.system.prefundedSpecializedBalance('i');
    await client.system.prefundedSpecializedBalanceWithProof('i');
    await client.system.waitForStateTransitionResult('h');
    await client.system.pathElements(['p'], ['k']);
    await client.system.pathElementsWithProof(['p2'], ['k2']);
    expect(wasmSdk.getStatus).to.be.calledOnce();
    expect(wasmSdk.getCurrentQuorumsInfo).to.be.calledOnce();
    expect(wasmSdk.getTotalCreditsInPlatform).to.be.calledOnce();
    expect(wasmSdk.getTotalCreditsInPlatformWithProofInfo).to.be.calledOnce();
    expect(wasmSdk.getPrefundedSpecializedBalance).to.be.calledOnce();
    expect(wasmSdk.getPrefundedSpecializedBalanceWithProofInfo).to.be.calledOnce();
    expect(wasmSdk.waitForStateTransitionResult).to.be.calledOnce();
    expect(wasmSdk.getPathElements).to.be.calledOnce();
    expect(wasmSdk.getPathElementsWithProofInfo).to.be.calledOnce();
  });
});
