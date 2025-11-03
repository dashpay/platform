import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenKeepsHistoryRules', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create TokenKeepsHistoryRules from values', () => {
      const keepHistory = new wasm.TokenKeepsHistoryRules(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      expect(keepHistory.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get values', () => {
      const keepHistory = new wasm.TokenKeepsHistoryRules(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      expect(keepHistory.keepsTransferHistory).to.equal(true);
      expect(keepHistory.keepsFreezingHistory).to.equal(true);
      expect(keepHistory.keepsMintingHistory).to.equal(true);
      expect(keepHistory.keepsBurningHistory).to.equal(true);
      expect(keepHistory.keepsDirectPricingHistory).to.equal(true);
      expect(keepHistory.keepsDirectPurchaseHistory).to.equal(true);
    });
  });

  describe('setters', () => {
    it('should allow to set values', () => {
      const keepHistory = new wasm.TokenKeepsHistoryRules(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      keepHistory.keepsTransferHistory = false;
      keepHistory.keepsFreezingHistory = false;
      keepHistory.keepsMintingHistory = false;
      keepHistory.keepsBurningHistory = false;
      keepHistory.keepsDirectPricingHistory = false;
      keepHistory.keepsDirectPurchaseHistory = false;

      expect(keepHistory.keepsTransferHistory).to.equal(false);
      expect(keepHistory.keepsFreezingHistory).to.equal(false);
      expect(keepHistory.keepsMintingHistory).to.equal(false);
      expect(keepHistory.keepsBurningHistory).to.equal(false);
      expect(keepHistory.keepsDirectPricingHistory).to.equal(false);
      expect(keepHistory.keepsDirectPurchaseHistory).to.equal(false);
    });
  });
});
