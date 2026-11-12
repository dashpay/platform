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
  let waitForAffectedStateStub: SinonStub;
  let broadcastAndWaitForAffectedStateStub: SinonStub;
  let waitForStateTransitionResultStub: SinonStub;

  beforeEach(async function setup() {
    await init();
    const builder = wasmSDKPackage.WasmSdkBuilder.testnet();
    wasmSdk = await builder.build();
    client = EvoSDK.fromWasm(wasmSdk);

    broadcastStateTransitionStub = this.sinon.stub(wasmSdk, 'broadcastStateTransition').resolves();
    waitForResponseStub = this.sinon.stub(wasmSdk, 'waitForResponse').resolves('ok');
    broadcastAndWaitStub = this.sinon.stub(wasmSdk, 'broadcastAndWait').resolves('ok');
    waitForAffectedStateStub = this.sinon.stub(wasmSdk, 'waitForAffectedState').resolves('snapshot');
    broadcastAndWaitForAffectedStateStub = this.sinon.stub(wasmSdk, 'broadcastAndWaitForAffectedState').resolves('snapshot');
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

    it('should propagate a strict refusal without falling back to a snapshot wait', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      const refusal = {
        kind: wasmSDKPackage.WasmSdkErrorKind.ExecutionNotProved,
        code: -1,
        isRetriable: false,
        message: 'verified snapshot does not prove execution',
      };
      broadcastAndWaitStub.rejects(refusal);
      let caught: unknown;
      try {
        await client.stateTransitions.broadcastAndWait(st);
      } catch (error) {
        caught = error;
      }
      expect(caught).to.equal(refusal);
      expect(broadcastAndWaitStub).to.be.calledOnceWithExactly(st, undefined);
      expect(waitForAffectedStateStub).not.to.be.called();
      expect(broadcastAndWaitForAffectedStateStub).not.to.be.called();
    });
  });

  describe('waitForAffectedState()', () => {
    it('should forward an explicit snapshot wait and its settings', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      const settings = { retries: 2 } as wasmSDKPackage.PutSettings;
      const result = await client.stateTransitions.waitForAffectedState(st, settings);
      expect(result).to.equal('snapshot');
      expect(waitForAffectedStateStub).to.be.calledOnceWithExactly(st, settings);
      expect(broadcastStateTransitionStub).not.to.be.called();
    });
  });

  describe('broadcastAndWaitForAffectedState()', () => {
    it('should forward the explicit combined snapshot policy and its settings', async () => {
      const st = {} as wasmSDKPackage.StateTransition;
      const settings = { retries: 2 } as wasmSDKPackage.PutSettings;
      const result = await client.stateTransitions.broadcastAndWaitForAffectedState(st, settings);
      expect(result).to.equal('snapshot');
      expect(broadcastAndWaitForAffectedStateStub).to.be.calledOnceWithExactly(st, settings);
      expect(broadcastAndWaitStub).not.to.be.called();
    });
  });

  describe('waitForStateTransitionResult()', () => {
    it('should forward to waitForStateTransitionResult', async () => {
      await client.stateTransitions.waitForStateTransitionResult('h');
      expect(waitForStateTransitionResultStub).to.be.calledOnce();
    });
  });
});
