import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.ts';

describe('TokenPricing', function describeTokenPricing() {
  this.timeout(60000);

  let client: sdk.WasmSdk;
  let builder: sdk.WasmSdkBuilder;

  before(async () => {
    await init();
    const context = await sdk.WasmTrustedContext.prefetchLocal();
    builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('calculateTokenIdFromContract()', () => {
    it('should calculate token id from contract', async () => {
      const CONTRACT_ID = wasmFunctionalTestRequirements().tokenContracts[0].contractId;
      const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(CONTRACT_ID, 0);
      expect(tokenId).to.be.a('string');
    });
  });

  describe('getTokenPriceByContract()', () => {
    it('should fetch price by contract', async () => {
      const CONTRACT_ID = wasmFunctionalTestRequirements().tokenContracts[0].contractId;
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
});
