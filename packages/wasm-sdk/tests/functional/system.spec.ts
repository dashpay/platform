import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('System', function describeSystem() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('getCurrentQuorumsInfo()', () => {
    it('should get current quorums info', async () => {
      const r = await client.getCurrentQuorumsInfo();
      expect(r).to.be.ok();
    });
  });

  describe('getTotalCreditsInPlatform()', () => {
    it('should get total credits in platform', async () => {
      const r = await client.getTotalCreditsInPlatform();
      expect(typeof r).to.equal('bigint');
      expect(String(r)).to.match(/^\d+$/);
    });
  });
});
