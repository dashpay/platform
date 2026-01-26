import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Utilities', function describeUtilities() {
  before(async () => { await init(); });
  this.timeout(60000);

  it('should prefetch trusted quorums for local', async () => {
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
  });

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
