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

  it('getTokenTotalSupply using derived token id', async () => {
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 0);
    const total = await client.getTokenTotalSupply(tokenId);
    // Returns total supply (might be 0 for tokens without minting)
    expect(total).to.exist;
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
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT_3, 0);
    await client.getTokenPerpetualDistributionLastClaim(TEST_IDENTITY, tokenId);
  });
});
