import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('SystemFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  // Stub references for type-safe assertions
  let getStatusStub: SinonStub;
  let getCurrentQuorumsInfoStub: SinonStub;
  let getTotalCreditsInPlatformStub: SinonStub;
  let getTotalCreditsInPlatformWithProofInfoStub: SinonStub;
  let getPrefundedSpecializedBalanceStub: SinonStub;
  let getPrefundedSpecializedBalanceWithProofInfoStub: SinonStub;
  let waitForStateTransitionResultStub: SinonStub;
  let getPathElementsStub: SinonStub;
  let getPathElementsWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnetTrusted();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getStatusStub = this.sinon.stub(wasmSdk, 'getStatus').resolves('ok');
    getCurrentQuorumsInfoStub = this.sinon.stub(wasmSdk, 'getCurrentQuorumsInfo').resolves('ok');
    getTotalCreditsInPlatformStub = this.sinon.stub(wasmSdk, 'getTotalCreditsInPlatform').resolves('ok');
    getTotalCreditsInPlatformWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTotalCreditsInPlatformWithProofInfo').resolves('ok');
    getPrefundedSpecializedBalanceStub = this.sinon.stub(wasmSdk, 'getPrefundedSpecializedBalance').resolves('ok');
    getPrefundedSpecializedBalanceWithProofInfoStub = this.sinon.stub(wasmSdk, 'getPrefundedSpecializedBalanceWithProofInfo').resolves('ok');
    waitForStateTransitionResultStub = this.sinon.stub(wasmSdk, 'waitForStateTransitionResult').resolves('ok');
    getPathElementsStub = this.sinon.stub(wasmSdk, 'getPathElements').resolves('ok');
    getPathElementsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getPathElementsWithProofInfo').resolves('ok');
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
    expect(getStatusStub).to.be.calledOnce();
    expect(getCurrentQuorumsInfoStub).to.be.calledOnce();
    expect(getTotalCreditsInPlatformStub).to.be.calledOnce();
    expect(getTotalCreditsInPlatformWithProofInfoStub).to.be.calledOnce();
    expect(getPrefundedSpecializedBalanceStub).to.be.calledOnce();
    expect(getPrefundedSpecializedBalanceWithProofInfoStub).to.be.calledOnce();
    expect(waitForStateTransitionResultStub).to.be.calledOnce();
    expect(getPathElementsStub).to.be.calledOnce();
    expect(getPathElementsWithProofInfoStub).to.be.calledOnce();
  });
});
