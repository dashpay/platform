import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Token queries', function describeTokenQueries() {
  this.timeout(60000);

  const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
  const TOKEN_CONTRACT = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
  const TOKEN_CONTRACT_2 = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
  const TOKEN_CONTRACT_3 = 'EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta';

  let client;
  let builder;

  before(async () => {
    await init();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  // TODO: fix this test

  it.skip('getTokenTotalSupply using derived token id', async () => {
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 0);
    const total = await client.getTokenTotalSupply(tokenId);
    // Returns an object with totalSupply as string
    expect(total).to.be.an('object');
    expect(String(total.totalSupply)).to.match(/^\d+$/);
  });

  it('getTokenStatuses for multiple tokens', async () => {
    await client.getTokenStatuses([TOKEN_CONTRACT]);
  });

  it('getTokenDirectPurchasePrices', async () => {
    await client.getTokenDirectPurchasePrices([TOKEN_CONTRACT_2]);
  });

  it('getTokenContractInfo', async () => {
    await client.getTokenContractInfo(TOKEN_CONTRACT_3);
  });

  it('getTokenPerpetualDistributionLastClaim', async () => {
    await client.getTokenPerpetualDistributionLastClaim(TEST_IDENTITY, TOKEN_CONTRACT_3);
  });
});
