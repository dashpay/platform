import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenConfiguration', () => {
  describe('serialization / deserialization', () => {
    it('Should allow to create from values', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: {
            shouldCapitalize: true,
            singularForm: 'TOKEN',
            pluralForm: 'TOKENS',
          },
        },
        1,
      );

      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = new wasm.ChangeControlRules(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const keepHistory = new wasm.TokenKeepsHistoryRules(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10000),
          },
        },
      );

      const distributionRules = new wasm.TokenDistributionRules(
        undefined,
        changeRules,
        preProgrammedDistribution,
        undefined,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const marketplaceRules = new wasm.TokenMarketplaceRules(
        tradeMode,
        changeRules,
      );

      const config = new wasm.TokenConfiguration(
        convention,
        changeRules,
        BigInt(999999999),
        undefined,
        keepHistory,
        false,
        false,
        changeRules,
        distributionRules,
        marketplaceRules,
        changeRules,
        changeRules,
        changeRules,
        changeRules,
        changeRules,
        changeRules,
        undefined,
        noOne,
        'note',
      );

      expect(config.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get getters', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: {
            shouldCapitalize: true,
            singularForm: 'TOKEN',
            pluralForm: 'TOKENS',
          },
        },
        1,
      );

      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = new wasm.ChangeControlRules(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const keepHistory = new wasm.TokenKeepsHistoryRules(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistribution(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10000),
          },
        },
      );

      const distributionRules = new wasm.TokenDistributionRules(
        undefined,
        changeRules,
        preProgrammedDistribution,
        undefined,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const marketplaceRules = new wasm.TokenMarketplaceRules(
        tradeMode,
        changeRules,
      );

      const config = new wasm.TokenConfiguration(
        convention,
        changeRules,
        BigInt(999999999),
        undefined,
        keepHistory,
        false,
        false,
        changeRules,
        distributionRules,
        marketplaceRules,
        changeRules,
        changeRules,
        changeRules,
        changeRules,
        changeRules,
        changeRules,
        undefined,
        noOne,
        'note',
      );

      expect(config.conventions.constructor.name).to.equal('TokenConfigurationConvention');
      expect(config.conventionsChangeRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.baseSupply.constructor.name).to.equal('BigInt');
      expect(config.keepsHistory.constructor.name).to.equal('TokenKeepsHistoryRules');
      expect(config.startAsPaused.constructor.name).to.equal('Boolean');
      expect(config.isAllowedTransferToFrozenBalance.constructor.name).to.equal('Boolean');
      expect(config.maxSupply).to.equal(undefined);
      expect(config.maxSupplyChangeRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.distributionRules.constructor.name).to.equal('TokenDistributionRules');
      expect(config.marketplaceRules.constructor.name).to.equal('TokenMarketplaceRules');
      expect(config.manualMintingRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.manualBurningRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.freezeRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.unfreezeRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.destroyFrozenFundsRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.emergencyActionRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.mainControlGroup).to.equal(undefined);
      expect(config.description.constructor.name).to.equal('String');
    });
  });
});
