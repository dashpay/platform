import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

interface KeepsHistoryRulesOptions {
  isKeepingTransferHistory?: boolean;
  isKeepingFreezingHistory?: boolean;
  isKeepingMintingHistory?: boolean;
  isKeepingBurningHistory?: boolean;
  isKeepingDirectPricingHistory?: boolean;
  isKeepingDirectPurchaseHistory?: boolean;
}

describe('TokenKeepsHistoryRules', () => {
  // Helper function to create TokenKeepsHistoryRules with default options
  function createKeepsHistoryRules(options: KeepsHistoryRulesOptions = {}) {
    return new wasm.TokenKeepsHistoryRules({
      isKeepingTransferHistory: options.isKeepingTransferHistory ?? true,
      isKeepingFreezingHistory: options.isKeepingFreezingHistory ?? true,
      isKeepingMintingHistory: options.isKeepingMintingHistory ?? true,
      isKeepingBurningHistory: options.isKeepingBurningHistory ?? true,
      isKeepingDirectPricingHistory: options.isKeepingDirectPricingHistory ?? true,
      isKeepingDirectPurchaseHistory: options.isKeepingDirectPurchaseHistory ?? true,
    });
  }

  describe('constructor', () => {
    it('should create instance from values', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory).to.be.an.instanceof(wasm.TokenKeepsHistoryRules);
    });
  });

  describe('isKeepingTransferHistory', () => {
    it('should return isKeepingTransferHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingTransferHistory).to.equal(true);
    });

    it('should set isKeepingTransferHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingTransferHistory = false;

      expect(keepHistory.isKeepingTransferHistory).to.equal(false);
    });
  });

  describe('isKeepingFreezingHistory', () => {
    it('should return isKeepingFreezingHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingFreezingHistory).to.equal(true);
    });

    it('should set isKeepingFreezingHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingFreezingHistory = false;

      expect(keepHistory.isKeepingFreezingHistory).to.equal(false);
    });
  });

  describe('isKeepingMintingHistory', () => {
    it('should return isKeepingMintingHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingMintingHistory).to.equal(true);
    });

    it('should set isKeepingMintingHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingMintingHistory = false;

      expect(keepHistory.isKeepingMintingHistory).to.equal(false);
    });
  });

  describe('isKeepingBurningHistory', () => {
    it('should return isKeepingBurningHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingBurningHistory).to.equal(true);
    });

    it('should set isKeepingBurningHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingBurningHistory = false;

      expect(keepHistory.isKeepingBurningHistory).to.equal(false);
    });
  });

  describe('isKeepingDirectPricingHistory', () => {
    it('should return isKeepingDirectPricingHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingDirectPricingHistory).to.equal(true);
    });

    it('should set isKeepingDirectPricingHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingDirectPricingHistory = false;

      expect(keepHistory.isKeepingDirectPricingHistory).to.equal(false);
    });
  });

  describe('isKeepingDirectPurchaseHistory', () => {
    it('should return isKeepingDirectPurchaseHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      expect(keepHistory.isKeepingDirectPurchaseHistory).to.equal(true);
    });

    it('should set isKeepingDirectPurchaseHistory', () => {
      const keepHistory = createKeepsHistoryRules();

      keepHistory.isKeepingDirectPurchaseHistory = false;

      expect(keepHistory.isKeepingDirectPurchaseHistory).to.equal(false);
    });
  });
});
