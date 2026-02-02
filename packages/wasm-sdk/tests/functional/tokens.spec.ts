import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Tokens', function describeTokens() {
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

  describe('getTokenTotalSupply()', () => {
    it('should return token total supply', async () => {
      const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 0);
      const total = await client.getTokenTotalSupply(tokenId);
      // Returns total supply (might be 0 for tokens without minting)
      expect(total).to.exist();
    });
  });

  describe('getTokenStatuses()', () => {
    it('should return token statuses for multiple tokens', async () => {
      await client.getTokenStatuses([TOKEN_CONTRACT]);
    });
  });

  describe('getTokenDirectPurchasePrices()', () => {
    it('should return token direct purchase prices', async () => {
      await client.getTokenDirectPurchasePrices([TOKEN_CONTRACT_2]);
    });
  });

  describe('getTokenContractInfo()', () => {
    it('should return token contract info', async () => {
      await client.getTokenContractInfo(TOKEN_CONTRACT_3);
    });
  });

  describe('getTokenPerpetualDistributionLastClaim()', () => {
    it('should return token perpetual distribution last claim', async () => {
      const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT_3, 0);
      await client.getTokenPerpetualDistributionLastClaim(TEST_IDENTITY, tokenId);
    });
  });
});
