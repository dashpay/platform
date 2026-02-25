import type { SinonStub } from 'sinon';
import init, * as wasmSDKPackage from '@dashevo/wasm-sdk';
import { EvoSDK } from '../../../dist/sdk.js';

describe('StateTransitionsFacade', () => {
  let wasmSdk: wasmSDKPackage.WasmSdk;
  let client: EvoSDK;

  // Stub references for type-safe assertions
  let broadcastStateTransitionStub: SinonStub;
  let waitForResponseStub: SinonStub;
  let broadcastAndWaitStub: SinonStub;
  let waitForStateTransitionResultStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    broadcastStateTransitionStub = this.sinon.stub(wasmSdk, 'broadcastStateTransition').resolves();
    waitForResponseStub = this.sinon.stub(wasmSdk, 'waitForResponse').resolves('ok');
    broadcastAndWaitStub = this.sinon.stub(wasmSdk, 'broadcastAndWait').resolves('ok');
    waitForStateTransitionResultStub = this.sinon.stub(wasmSdk, 'waitForStateTransitionResult').resolves('ok');
  });

  describe('broadcastStateTransition()', () => {
    it('should forward to broadcastStateTransition', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      await client.stateTransitions.broadcastStateTransition(st);
      expect(broadcastStateTransitionStub).to.be.calledOnceWithExactly(st, undefined);
    });

    it('should pass settings when provided', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      const settings = { retries: 3 } as wasmSDKPackage.PutSettings;
      await client.stateTransitions.broadcastStateTransition(st, settings);
      expect(broadcastStateTransitionStub).to.be.calledOnceWithExactly(st, settings);
    });
  });

  describe('waitForResponse()', () => {
    it('should forward to waitForResponse', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      await client.stateTransitions.waitForResponse(st);
      expect(waitForResponseStub).to.be.calledOnceWithExactly(st, undefined);
    });

    it('should pass settings when provided', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      const settings = { retries: 5 } as wasmSDKPackage.PutSettings;
      await client.stateTransitions.waitForResponse(st, settings);
      expect(waitForResponseStub).to.be.calledOnceWithExactly(st, settings);
    });
  });

  describe('broadcastAndWait()', () => {
    it('should forward to broadcastAndWait', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      await client.stateTransitions.broadcastAndWait(st);
      expect(broadcastAndWaitStub).to.be.calledOnceWithExactly(st, undefined);
    });

    it('should pass settings when provided', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      const settings = { retries: 2 } as wasmSDKPackage.PutSettings;
      await client.stateTransitions.broadcastAndWait(st, settings);
      expect(broadcastAndWaitStub).to.be.calledOnceWithExactly(st, settings);
    });
  });

  describe('waitForStateTransitionResult()', () => {
    it('should forward to waitForStateTransitionResult', async () => {
      await client.stateTransitions.waitForStateTransitionResult('h');
      expect(waitForStateTransitionResultStub).to.be.calledOnce();
    });
  });
});
