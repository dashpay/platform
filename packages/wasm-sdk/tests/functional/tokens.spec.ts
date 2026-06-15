import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { prefetchLocalReady } from './helpers/trustedContext.ts';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('Tokens', function describeTokens() {
  this.timeout(60000);

  const req = wasmFunctionalTestRequirements();
  const TEST_IDENTITY = req.identityId;
  const TOKEN_CONTRACT = req.tokenContracts[0].contractId;
  const TOKEN_CONTRACT_2 = TOKEN_CONTRACT;

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
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
    it('should return token contract info for a valid token id', async () => {
      const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 0);
      const info = await client.getTokenContractInfo(tokenId);
      expect(info).to.exist();
      expect(info.contractId.toBase58()).to.equal(TOKEN_CONTRACT);
      expect(info.tokenContractPosition).to.equal(0);
    });
  });

  describe('getTokenPerpetualDistributionLastClaim()', () => {
    it('should return token perpetual distribution last claim', async () => {
      const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 0);
      await client.getTokenPerpetualDistributionLastClaim(TEST_IDENTITY, tokenId);
    });
  });
});
