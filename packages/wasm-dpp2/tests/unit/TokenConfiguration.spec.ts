import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { object, ownerId } from './mocks/DataContract/index.js';

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

interface ShieldedPoolOptions {
  hasShieldedPool?: boolean;
  minimumPoolNotesForOutgoing?: bigint;
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

  describe('minimumPoolNotesForOutgoing', () => {
    // A configuration whose shielded pool settings are the only thing under test. Everything
    // else is the least interesting valid value, and `hasShieldedPool` defaults to false so a
    // test has to ask for a pool to get one.
    function createConfiguration(shieldedPool: ShieldedPoolOptions = {}) {
      const changeRules = createChangeControlRules();

      return new wasm.TokenConfiguration({
        conventions: new wasm.TokenConfigurationConvention(
          {
            en: {
              $formatVersion: '0',
              shouldCapitalize: true,
              singularForm: 'TOKEN',
              pluralForm: 'TOKENS',
            },
          },
          8,
        ),
        conventionsChangeRules: changeRules,
        baseSupply: BigInt(1000),
        maxSupply: undefined,
        keepsHistory: createKeepsHistoryRules(),
        isStartedAsPaused: false,
        isAllowedTransferToFrozenBalance: false,
        maxSupplyChangeRules: changeRules,
        distributionRules: new wasm.TokenDistributionRules({
          perpetualDistribution: undefined,
          perpetualDistributionRules: changeRules,
          preProgrammedDistribution: undefined,
          newTokensDestinationIdentityRules: changeRules,
          mintingAllowChoosingDestination: true,
          mintingAllowChoosingDestinationRules: changeRules,
          changeDirectPurchasePricingRules: changeRules,
        }),
        marketplaceRules: new wasm.TokenMarketplaceRules(
          wasm.TokenTradeMode.NotTradeable(),
          changeRules,
        ),
        manualMintingRules: changeRules,
        manualBurningRules: changeRules,
        freezeRules: changeRules,
        unfreezeRules: changeRules,
        destroyFrozenFundsRules: changeRules,
        emergencyActionRules: changeRules,
        mainControlGroup: undefined,
        mainControlGroupCanBeModified: wasm.AuthorizedActionTakers.NoOne(),
        description: 'note',
        ...shieldedPool,
      });
    }

    // The configuration a data contract hands back, rebuilt from the one it was given. Reading
    // a threshold off it proves the value reached the configuration itself: assigning to a
    // property the wasm class does not define lands on the JavaScript object instead, where
    // the same object's getter reads it straight back and agrees with anything.
    function configurationHeldByContract(config: wasm.TokenConfiguration) {
      const contract = new wasm.DataContract({
        ownerId,
        identityNonce: BigInt(2),
        schemas: object.documentSchemas,
        definitions: null,
        fullValidation: false,
        tokens: { 0: config },
      });

      return (contract.tokens as Record<number, wasm.TokenConfiguration>)[0];
    }

    it('should keep a threshold asked for at creation', () => {
      const config = createConfiguration({
        hasShieldedPool: true,
        minimumPoolNotesForOutgoing: BigInt(5),
      });

      // A threshold reachable only through a later TokenConfigUpdate would leave every token
      // created from JavaScript with an unguarded pool for its first blocks.
      expect(config.minimumPoolNotesForOutgoing).to.equal(BigInt(5));
      expect(config.hasShieldedPool).to.equal(true);
      expect(config.formatVersion).to.equal(1);
    });

    it('should read no threshold as zero for a pool that asks for none', () => {
      const config = createConfiguration({ hasShieldedPool: true });

      expect(config.minimumPoolNotesForOutgoing).to.equal(BigInt(0));
    });

    it('should read no threshold as zero for a token without a shielded pool', () => {
      const config = createConfiguration();

      expect(config.hasShieldedPool).to.equal(false);
      expect(config.formatVersion).to.equal(0);
      expect(config.minimumPoolNotesForOutgoing).to.equal(BigInt(0));
    });

    it('should round-trip a threshold written through the setter', () => {
      const config = createConfiguration({ hasShieldedPool: true });

      config.minimumPoolNotesForOutgoing = BigInt(7);

      expect(config.minimumPoolNotesForOutgoing).to.equal(BigInt(7));
      expect(configurationHeldByContract(config).minimumPoolNotesForOutgoing)
        .to.equal(BigInt(7));
    });

    it('should drop the threshold when the setter is given undefined', () => {
      const config = createConfiguration({
        hasShieldedPool: true,
        minimumPoolNotesForOutgoing: BigInt(9),
      });

      config.minimumPoolNotesForOutgoing = undefined;

      expect(config.minimumPoolNotesForOutgoing).to.equal(BigInt(0));
    });

    it('should refuse a threshold at creation for a token without a shielded pool', () => {
      // Silently accepting it is the failure this guards: the only configurations that carry a
      // threshold are the ones that carry a pool, so the value would go nowhere unreported.
      expect(() => createConfiguration({ minimumPoolNotesForOutgoing: BigInt(5) }))
        .to.throw(/hasShieldedPool/);
    });

    it('should refuse a threshold through the setter for a token without a shielded pool', () => {
      const config = createConfiguration();

      expect(() => {
        config.minimumPoolNotesForOutgoing = BigInt(5);
      }).to.throw(/hasShieldedPool/);
    });
  });
});
