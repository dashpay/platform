import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Utilities', function describeUtilities() {
  before(async () => { await init(); });
  this.timeout(60000);

  it('prefetches trusted quorums (mainnet/testnet) or tolerates network errors', async () => {
    await sdk.WasmSdk.prefetchTrustedQuorumsMainnet();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
  });

  it('testSerialization method (if present) returns object', async () => {
    const builder = sdk.WasmSdkBuilder.testnet();
    const client = await builder.build();
    if (typeof client.testSerialization === 'function') {
      const res = client.testSerialization('simple');
      expect(res).to.be.an('object');
    }
    client.free();
  });
});
