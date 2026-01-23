import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenKeepsHistoryRules', () => {
  // Helper function to create TokenKeepsHistoryRules with default options
  function createKeepsHistoryRules(options = {}) {
    return new wasm.TokenKeepsHistoryRules({
      isKeepingTransferHistory: options.isKeepingTransferHistory ?? true,
      isKeepingFreezingHistory: options.isKeepingFreezingHistory ?? true,
      isKeepingMintingHistory: options.isKeepingMintingHistory ?? true,
      isKeepingBurningHistory: options.isKeepingBurningHistory ?? true,
      isKeepingDirectPricingHistory: options.isKeepingDirectPricingHistory ?? true,
      isKeepingDirectPurchaseHistory: options.isKeepingDirectPurchaseHistory ?? true,
    });
  }

  describe('serialization / deserialization', () => {
    it('should allow to create TokenKeepsHistoryRules from values', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory).to.be.an.instanceof(wasm.TokenKeepsHistoryRules);
    });
  });

  describe('getters', () => {
    it('should allow to get values', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingTransferHistory).to.equal(true);
      expect(keepHistory.isKeepingFreezingHistory).to.equal(true);
      expect(keepHistory.isKeepingMintingHistory).to.equal(true);
      expect(keepHistory.isKeepingBurningHistory).to.equal(true);
      expect(keepHistory.isKeepingDirectPricingHistory).to.equal(true);
      expect(keepHistory.isKeepingDirectPurchaseHistory).to.equal(true);
    });
  });

  describe('setters', () => {
    it('should allow to set values', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingTransferHistory = false;
      keepHistory.isKeepingFreezingHistory = false;
      keepHistory.isKeepingMintingHistory = false;
      keepHistory.isKeepingBurningHistory = false;
      keepHistory.isKeepingDirectPricingHistory = false;
      keepHistory.isKeepingDirectPurchaseHistory = false;

      expect(keepHistory.isKeepingTransferHistory).to.equal(false);
      expect(keepHistory.isKeepingFreezingHistory).to.equal(false);
      expect(keepHistory.isKeepingMintingHistory).to.equal(false);
      expect(keepHistory.isKeepingBurningHistory).to.equal(false);
      expect(keepHistory.isKeepingDirectPricingHistory).to.equal(false);
      expect(keepHistory.isKeepingDirectPurchaseHistory).to.equal(false);
    });
  });
});
