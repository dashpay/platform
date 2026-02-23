import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Utilities', function describeUtilities() {
  this.timeout(60000);

  before(async () => { await init(); });

  describe('WasmTrustedContext.prefetchLocal()', () => {
    it('should prefetch trusted quorums for local', async () => {
      await sdk.WasmTrustedContext.prefetchLocal();
    });
  });

  describe('testSerialization()', () => {
    it('should return object from testSerialization method (if present)', async () => {
      const builder = sdk.WasmSdkBuilder.local();
      const client = await builder.build();
      if (typeof client.testSerialization === 'function') {
        const res = client.testSerialization('simple');
        expect(res).to.be.an('object');
      }
      client.free();
    });
  });
});
