import init, * as sdk from '../../dist/sdk.js';

describe('Utilities', function () {
  before(async () => { await init(); });
  this.timeout(60000);

  it('prefetches trusted quorums (mainnet/testnet) or tolerates network errors', async function () {
    const enabled = typeof process !== 'undefined' && process.env && (process.env.SDK_FUNCTIONAL === '1' || process.env.SDK_FUNCTIONAL === 'true');
    if (!enabled) this.skip();
    try { await sdk.WasmSdk.prefetchTrustedQuorumsMainnet(); } catch (_) {}
    try { await sdk.WasmSdk.prefetchTrustedQuorumsTestnet(); } catch (_) {}
  });

  it('testSerialization method (if present) returns object', async function () {
    const enabled = typeof process !== 'undefined' && process.env && (process.env.SDK_FUNCTIONAL === '1' || process.env.SDK_FUNCTIONAL === 'true');
    if (!enabled) this.skip();
    const builder = sdk.WasmSdkBuilder.testnet();
    const client = await builder.build();
    if (typeof client.testSerialization === 'function') {
      const res = client.testSerialization('simple');
      expect(res).to.be.an('object');
    }
    client.free();
    builder.free();
  });
});
