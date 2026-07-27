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
  let getPathElementsStub: SinonStub;
  let getPathElementsWithProofInfoStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    getStatusStub = this.sinon.stub(wasmSdk, 'getStatus').resolves('ok');
    getCurrentQuorumsInfoStub = this.sinon.stub(wasmSdk, 'getCurrentQuorumsInfo').resolves('ok');
    getTotalCreditsInPlatformStub = this.sinon.stub(wasmSdk, 'getTotalCreditsInPlatform').resolves('ok');
    getTotalCreditsInPlatformWithProofInfoStub = this.sinon.stub(wasmSdk, 'getTotalCreditsInPlatformWithProofInfo').resolves('ok');
    getPrefundedSpecializedBalanceStub = this.sinon.stub(wasmSdk, 'getPrefundedSpecializedBalance').resolves('ok');
    getPrefundedSpecializedBalanceWithProofInfoStub = this.sinon.stub(wasmSdk, 'getPrefundedSpecializedBalanceWithProofInfo').resolves('ok');
    getPathElementsStub = this.sinon.stub(wasmSdk, 'getPathElements').resolves('ok');
    getPathElementsWithProofInfoStub = this.sinon.stub(wasmSdk, 'getPathElementsWithProofInfo').resolves('ok');
  });

  describe('status()', () => {
    it('should forward to getStatus', async () => {
      await client.system.status();
      expect(getStatusStub).to.be.calledOnce();
    });
  });

  describe('currentQuorumsInfo()', () => {
    it('should forward to getCurrentQuorumsInfo', async () => {
      await client.system.currentQuorumsInfo();
      expect(getCurrentQuorumsInfoStub).to.be.calledOnce();
    });
  });

  describe('totalCreditsInPlatform()', () => {
    it('should forward to getTotalCreditsInPlatform', async () => {
      await client.system.totalCreditsInPlatform();
      expect(getTotalCreditsInPlatformStub).to.be.calledOnce();
    });
  });

  describe('totalCreditsInPlatformWithProof()', () => {
    it('should forward to getTotalCreditsInPlatformWithProofInfo', async () => {
      await client.system.totalCreditsInPlatformWithProof();
      expect(getTotalCreditsInPlatformWithProofInfoStub).to.be.calledOnce();
    });
  });

  describe('prefundedSpecializedBalance()', () => {
    it('should forward to getPrefundedSpecializedBalance', async () => {
      await client.system.prefundedSpecializedBalance('i');
      expect(getPrefundedSpecializedBalanceStub).to.be.calledOnce();
    });
  });

  describe('prefundedSpecializedBalanceWithProof()', () => {
    it('should forward to getPrefundedSpecializedBalanceWithProofInfo', async () => {
      await client.system.prefundedSpecializedBalanceWithProof('i');
      expect(getPrefundedSpecializedBalanceWithProofInfoStub).to.be.calledOnce();
    });
  });

  describe('pathElements()', () => {
    it('should forward to getPathElements', async () => {
      const path = ['p', new Uint8Array([0x80, 0xff])];
      const keys = [new Uint8Array([0x01, 0x02])];

      await client.system.pathElements(path, keys);

      expect(getPathElementsStub).to.be.calledOnce();
      expect(getPathElementsStub).to.be.calledWithExactly(path, keys);
    });
  });

  describe('pathElementsWithProof()', () => {
    it('should forward to getPathElementsWithProofInfo', async () => {
      const path = ['p2', new Uint8Array([0x80, 0xff])];
      const keys = [new Uint8Array([0x03, 0x04])];

      await client.system.pathElementsWithProof(path, keys);

      expect(getPathElementsWithProofInfoStub).to.be.calledOnce();
      expect(getPathElementsWithProofInfoStub).to.be.calledWithExactly(path, keys);
    });
  });
});
