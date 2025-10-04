import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenConfiguration', () => {
  describe('serialization / deserialization', () => {
    it('Should allow to create from values', () => {
      const convention = new wasm.TokenConfigurationConventionWASM(
        {
          ru: {
            shouldCapitalize: true,
            singularForm: 'TOKEN',
            pluralForm: 'TOKENS',
          },
        },
        1,
      );

      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const keepHistory = new wasm.TokenKeepsHistoryRulesWASM(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistributionWASM(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10000),
          },
        },
      );

      const distributionRules = new wasm.TokenDistributionRulesWASM(
        undefined,
        changeRules,
        preProgrammedDistribution,
        undefined,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      const tradeMode = wasm.TokenTradeModeWASM.NotTradeable();

      const marketplaceRules = new wasm.TokenMarketplaceRulesWASM(
        tradeMode,
        changeRules,
      );

      const config = new wasm.TokenConfigurationWASM(
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
      const convention = new wasm.TokenConfigurationConventionWASM(
        {
          ru: {
            shouldCapitalize: true,
            singularForm: 'TOKEN',
            pluralForm: 'TOKENS',
          },
        },
        1,
      );

      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const keepHistory = new wasm.TokenKeepsHistoryRulesWASM(
        true,
        true,
        true,
        true,
        true,
        true,
      );

      const preProgrammedDistribution = new wasm.TokenPreProgrammedDistributionWASM(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10000),
          },
        },
      );

      const distributionRules = new wasm.TokenDistributionRulesWASM(
        undefined,
        changeRules,
        preProgrammedDistribution,
        undefined,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      const tradeMode = wasm.TokenTradeModeWASM.NotTradeable();

      const marketplaceRules = new wasm.TokenMarketplaceRulesWASM(
        tradeMode,
        changeRules,
      );

      const config = new wasm.TokenConfigurationWASM(
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

      expect(config.conventions.constructor.name).to.equal('TokenConfigurationConventionWASM');
      expect(config.conventionsChangeRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.baseSupply.constructor.name).to.equal('BigInt');
      expect(config.keepsHistory.constructor.name).to.equal('TokenKeepsHistoryRulesWASM');
      expect(config.startAsPaused.constructor.name).to.equal('Boolean');
      expect(config.isAllowedTransferToFrozenBalance.constructor.name).to.equal('Boolean');
      expect(config.maxSupply).to.equal(undefined);
      expect(config.maxSupplyChangeRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.distributionRules.constructor.name).to.equal('TokenDistributionRulesWASM');
      expect(config.marketplaceRules.constructor.name).to.equal('TokenMarketplaceRulesWASM');
      expect(config.manualMintingRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.manualBurningRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.freezeRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.unfreezeRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.destroyFrozenFundsRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.emergencyActionRules.constructor.name).to.equal('ChangeControlRulesWASM');
      expect(config.mainControlGroup).to.equal(undefined);
      expect(config.description.constructor.name).to.equal('String');
    });
  });
});
