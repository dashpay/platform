import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Unsupported proof modes', () => {
  before(async () => {
    await init();
  });

  for (const method of ['getIdentity', 'getIdentityWithProofInfo'] as const) {
    it(`should reject ${method} without proofs before transport and remain usable`, async () => {
      const originalFetch = globalThis.fetch;
      let fetchCalls = 0;
      globalThis.fetch = async () => {
        fetchCalls += 1;
        throw new Error('Unsupported proof modes must fail before transport');
      };
      let instance: sdk.WasmSdk | undefined;
      try {
        instance = sdk.WasmSdkBuilder
          .withAddresses(['https://node-1.test:1443'], 'local')
          .withVersion(1)
          .withProofs(false)
          .build();
        const identityId = new Uint8Array(32).fill(1);
        for (let attempt = 0; attempt < 2; attempt += 1) {
          let refusal: unknown;
          try {
            await instance[method](identityId);
          } catch (error) {
            refusal = error;
          }
          expect(refusal).to.be.instanceOf(sdk.WasmSdkError);
          const error = refusal as sdk.WasmSdkError;
          expect(error.kind).to.equal(sdk.WasmSdkErrorKind.Config);
          expect(error.name).to.equal('Config');
          expect(error.message).to.include('unproved queries are not supported for this query; enable proofs');
          expect(error.code).to.equal(-1);
          expect(error.isRetriable).to.equal(false);
          expect(instance.version()).to.equal(1);
        }
        expect(fetchCalls).to.equal(0);
      } finally {
        globalThis.fetch = originalFetch;
        instance?.free();
      }
    });
  }
});
