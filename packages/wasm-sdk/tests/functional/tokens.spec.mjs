import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Token queries', function describeTokenQueries() {
  this.timeout(60000);

  const req = wasmFunctionalTestRequirements();
  const TEST_IDENTITY = req.identityId;
  const TOKEN_CONTRACT = req.tokenContracts[0].contractId;
  const TOKEN_CONTRACT_2 = TOKEN_CONTRACT;
  const TOKEN_CONTRACT_3 = TOKEN_CONTRACT;

  let client;
  let builder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
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
