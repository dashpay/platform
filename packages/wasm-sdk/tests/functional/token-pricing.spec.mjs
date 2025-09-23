import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Token pricing', function describeTokenPricing() {
  this.timeout(60000);

  let client;
  let builder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('calculates token id and fetches price by contract', async () => {
    const CONTRACT_ID = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(CONTRACT_ID, 0);
    expect(tokenId).to.be.a('string');
    try {
      const info = await client.getTokenPriceByContract(CONTRACT_ID, 0);
      expect(info).to.be.ok();
    } catch (e) {
      if (!(e.message.includes('No pricing schedule'))) {
        throw e;
      }
    }
  });
});
