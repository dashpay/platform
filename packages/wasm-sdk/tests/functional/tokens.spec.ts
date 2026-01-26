import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Token queries', function describeTokenQueries() {
  this.timeout(60000);

  const req = wasmFunctionalTestRequirements();
  const TEST_IDENTITY = req.identityId;
  const TOKEN_CONTRACT = req.tokenContracts[0].contractId;
  const TOKEN_CONTRACT_2 = TOKEN_CONTRACT;
  const TOKEN_CONTRACT_3 = TOKEN_CONTRACT;

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

  it('should get token total supply using derived token id', async () => {
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 0);
    const total = await client.getTokenTotalSupply(tokenId);
    // Returns total supply (might be 0 for tokens without minting)
    expect(total).to.exist();
  });

  it('should get token statuses for multiple tokens', async () => {
    await client.getTokenStatuses([TOKEN_CONTRACT]);
  });

  it('should get token direct purchase prices', async () => {
    await client.getTokenDirectPurchasePrices([TOKEN_CONTRACT_2]);
  });

  it('should get token contract info', async () => {
    await client.getTokenContractInfo(TOKEN_CONTRACT_3);
  });

  it('should get token perpetual distribution last claim', async () => {
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT_3, 0);
    await client.getTokenPerpetualDistributionLastClaim(TEST_IDENTITY, tokenId);
  });
});
