import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

interface ChangeControlRulesOptions {
  authorizedToMakeChange?: unknown;
  adminActionTakers?: unknown;
  isChangingAuthorizedActionTakersToNoOneAllowed?: boolean;
  isChangingAdminActionTakersToNoOneAllowed?: boolean;
  isSelfChangingAdminActionTakersAllowed?: boolean;
}

interface KeepsHistoryRulesOptions {
  isKeepingTransferHistory?: boolean;
  isKeepingFreezingHistory?: boolean;
  isKeepingMintingHistory?: boolean;
  isKeepingBurningHistory?: boolean;
  isKeepingDestroyedFrozenFundsHistory?: boolean;
  isKeepingEmergencyActionHistory?: boolean;
}

describe('TokenConfiguration', () => {
  function createChangeControlRules(options: ChangeControlRulesOptions = {}) {
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
  function createKeepsHistoryRules(options: KeepsHistoryRulesOptions = {}) {
    return new wasm.TokenKeepsHistoryRules({
      isKeepingTransferHistory: options.isKeepingTransferHistory ?? true,
      isKeepingFreezingHistory: options.isKeepingFreezingHistory ?? true,
      isKeepingMintingHistory: options.isKeepingMintingHistory ?? true,
      isKeepingBurningHistory: options.isKeepingBurningHistory ?? true,
      isKeepingDestroyedFrozenFundsHistory: options.isKeepingDestroyedFrozenFundsHistory ?? true,
      isKeepingEmergencyActionHistory: options.isKeepingEmergencyActionHistory ?? true,
    });
  }
  function createPreProgrammedDistribution(timestamp: number, identifierBase58: string, amount: bigint) {
    const innerMap = new Map();
    innerMap.set(identifierBase58, amount);
    const outerMap = new Map();
    outerMap.set(timestamp.toString(), innerMap);
    return new wasm.TokenPreProgrammedDistribution(outerMap);
  }

  describe('constructor', () => {
    it('should create instance from values', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: {
            $formatVersion: '0',
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

      expect(config).to.be.an.instanceof(wasm.TokenConfiguration);
    });
  });

  describe('getters (value verification)', () => {
    it('should return correct values for all getters', () => {
      const convention = new wasm.TokenConfigurationConvention(
        {
          ru: {
            $formatVersion: '0',
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

      // Verify actual values, not just constructor names
      expect(config.conventions).to.be.an.instanceof(wasm.TokenConfigurationConvention);
      expect(config.conventions.decimals).to.equal(1);

      expect(config.conventionsChangeRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.conventionsChangeRules.authorizedToMakeChange.takerType).to.equal('NoOne');

      expect(config.baseSupply).to.equal(BigInt(999999999));

      expect(config.keepsHistory).to.be.an.instanceof(wasm.TokenKeepsHistoryRules);

      expect(config.isStartedAsPaused).to.equal(false);
      expect(config.isAllowedTransferToFrozenBalance).to.equal(false);
      expect(config.maxSupply).to.equal(undefined);

      expect(config.maxSupplyChangeRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.distributionRules).to.be.an.instanceof(wasm.TokenDistributionRules);
      expect(config.marketplaceRules).to.be.an.instanceof(wasm.TokenMarketplaceRules);

      expect(config.manualMintingRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.manualBurningRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.freezeRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.unfreezeRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.destroyFrozenFundsRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(config.emergencyActionRules).to.be.an.instanceof(wasm.ChangeControlRules);

      expect(config.mainControlGroup).to.equal(undefined);
      expect(config.mainControlGroupCanBeModified.takerType).to.equal('NoOne');
      expect(config.description).to.equal('note');
    });
  });
});
