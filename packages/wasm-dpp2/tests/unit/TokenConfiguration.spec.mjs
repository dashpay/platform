import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenConfiguration', () => {
  function createChangeControlRules(options = {}) {
    // AuthorizedActionTakers defaults to NoOne if not specified
    const noOne = wasm.AuthorizedActionTakers.NoOne();
    return new wasm.ChangeControlRules({
      authorizedToMakeChange: options.authorizedToMakeChange ?? noOne,
      adminActionTakers: options.adminActionTakers ?? noOne,
      isChangingAuthorizedActionTakersToNoOneAllowed: options.isChangingAuthorizedActionTakersToNoOneAllowed ?? true,
      isChangingAdminActionTakersToNoOneAllowed: options.isChangingAdminActionTakersToNoOneAllowed ?? true,
      isSelfChangingAdminActionTakersAllowed: options.isSelfChangingAdminActionTakersAllowed ?? true,
    });
  }

  function createKeepsHistoryRules(options = {}) {
    return new wasm.TokenKeepsHistoryRules({
      isKeepingTransferHistory: options.isKeepingTransferHistory ?? true,
      isKeepingFreezingHistory: options.isKeepingFreezingHistory ?? true,
      isKeepingMintingHistory: options.isKeepingMintingHistory ?? true,
      isKeepingBurningHistory: options.isKeepingBurningHistory ?? true,
      isKeepingDestroyedFrozenFundsHistory: options.isKeepingDestroyedFrozenFundsHistory ?? true,
      isKeepingEmergencyActionHistory: options.isKeepingEmergencyActionHistory ?? true,
    });
  }

  function createPreProgrammedDistribution(timestamp, identifierBase58, amount) {
    const identifier = new wasm.Identifier(identifierBase58);
    const innerMap = new Map();
    innerMap.set(identifier, amount);
    const outerMap = new Map();
    outerMap.set(timestamp.toString(), innerMap);
    return new wasm.TokenPreProgrammedDistribution(outerMap);
  }

  describe('serialization / deserialization', () => {
    it('Should allow to create from values', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: {
            $format_version: '0',
            shouldCapitalize: true,
            singularForm: 'TOKEN',
            pluralForm: 'TOKENS',
          },
        },
        1,
      );

      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules();
      const keepHistory = createKeepsHistoryRules();

      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );

      const distributionRules = new wasm.TokenDistributionRules({
        perpetualDistribution: undefined,
        perpetualDistributionRules: changeRules,
        preProgrammedDistribution,
        newTokensDestinationIdentityRules: changeRules,
        mintingAllowChoosingDestination: true,
        mintingAllowChoosingDestinationRules: changeRules,
        changeDirectPurchasePricingRules: changeRules,
      });

      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const marketplaceRules = new wasm.TokenMarketplaceRules(
        tradeMode,
        changeRules,
      );

      const config = new wasm.TokenConfiguration({
        conventions: convention,
        conventionsChangeRules: changeRules,
        baseSupply: BigInt(999999999),
        maxSupply: undefined,
        keepsHistory: keepHistory,
        isStartedAsPaused: false,
        isAllowedTransferToFrozenBalance: false,
        maxSupplyChangeRules: changeRules,
        distributionRules,
        marketplaceRules,
        manualMintingRules: changeRules,
        manualBurningRules: changeRules,
        freezeRules: changeRules,
        unfreezeRules: changeRules,
        destroyFrozenFundsRules: changeRules,
        emergencyActionRules: changeRules,
        mainControlGroup: undefined,
        mainControlGroupCanBeModified: noOne,
        description: 'note',
      });

      expect(config.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get getters', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: {
            $format_version: '0',
            shouldCapitalize: true,
            singularForm: 'TOKEN',
            pluralForm: 'TOKENS',
          },
        },
        1,
      );

      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules();
      const keepHistory = createKeepsHistoryRules();

      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );

      const distributionRules = new wasm.TokenDistributionRules({
        perpetualDistribution: undefined,
        perpetualDistributionRules: changeRules,
        preProgrammedDistribution,
        newTokensDestinationIdentityRules: changeRules,
        mintingAllowChoosingDestination: true,
        mintingAllowChoosingDestinationRules: changeRules,
        changeDirectPurchasePricingRules: changeRules,
      });

      const tradeMode = wasm.TokenTradeMode.NotTradeable();

      const marketplaceRules = new wasm.TokenMarketplaceRules(
        tradeMode,
        changeRules,
      );

      const config = new wasm.TokenConfiguration({
        conventions: convention,
        conventionsChangeRules: changeRules,
        baseSupply: BigInt(999999999),
        maxSupply: undefined,
        keepsHistory: keepHistory,
        isStartedAsPaused: false,
        isAllowedTransferToFrozenBalance: false,
        maxSupplyChangeRules: changeRules,
        distributionRules,
        marketplaceRules,
        manualMintingRules: changeRules,
        manualBurningRules: changeRules,
        freezeRules: changeRules,
        unfreezeRules: changeRules,
        destroyFrozenFundsRules: changeRules,
        emergencyActionRules: changeRules,
        mainControlGroup: undefined,
        mainControlGroupCanBeModified: noOne,
        description: 'note',
      });

      expect(config.conventions.constructor.name).to.equal('TokenConfigurationConvention');
      expect(config.conventionsChangeRules.constructor.name).to.equal('ChangeControlRules');
      expect(config.baseSupply.constructor.name).to.equal('BigInt');
      expect(config.keepsHistory.constructor.name).to.equal('TokenKeepsHistoryRules');
      expect(config.isStartedAsPaused.constructor.name).to.equal('Boolean');
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
