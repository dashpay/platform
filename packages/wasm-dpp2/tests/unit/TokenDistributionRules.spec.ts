import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { identifier } from './mocks/Identity/index.js';

before(async () => {
  await initWasm();
});

interface ChangeControlRulesOptions {
  isChangingAuthorizedActionTakersToNoOneAllowed?: boolean;
  isChangingAdminActionTakersToNoOneAllowed?: boolean;
  isSelfChangingAdminActionTakersAllowed?: boolean;
}

// Helper function to create ChangeControlRules with default options
function createChangeControlRules(
  authorizedToMakeChange: unknown,
  adminActionTakers: unknown,
  options: ChangeControlRulesOptions = {},
) {
  return new wasm.ChangeControlRules({
    authorizedToMakeChange,
    adminActionTakers,
    isChangingAuthorizedActionTakersToNoOneAllowed: options.isChangingAuthorizedActionTakersToNoOneAllowed ?? true,
    isChangingAdminActionTakersToNoOneAllowed: options.isChangingAdminActionTakersToNoOneAllowed ?? true,
    isSelfChangingAdminActionTakersAllowed: options.isSelfChangingAdminActionTakersAllowed ?? true,
  });
}

// Helper to create pre-programmed distribution with proper Map format
function createPreProgrammedDistribution(timestamp: number, identifierBase58: string, amount: bigint) {
  const id = new wasm.Identifier(identifierBase58);
  const innerMap = new Map();
  innerMap.set(id, amount);
  const outerMap = new Map();
  outerMap.set(timestamp.toString(), innerMap);
  return new wasm.TokenPreProgrammedDistribution(outerMap);
}

describe('TokenDistributionRules', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create with undefined values', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const distributionRules = new wasm.TokenDistributionRules({
        perpetualDistribution: undefined,
        perpetualDistributionRules: changeRules,
        preProgrammedDistribution: undefined,
        newTokensDestinationIdentity: undefined,
        newTokensDestinationIdentityRules: changeRules,
        mintingAllowChoosingDestination: true,
        mintingAllowChoosingDestinationRules: changeRules,
        changeDirectPurchasePricingRules: changeRules,
      });

      expect(distributionRules).to.be.an.instanceof(wasm.TokenDistributionRules);
      expect(changeRules).to.be.an.instanceof(wasm.ChangeControlRules);
    });

    it('should allow to create without undefined values', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );

      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const perpetualDistribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      const distributionRules = new wasm.TokenDistributionRules({
        perpetualDistribution,
        perpetualDistributionRules: changeRules,
        preProgrammedDistribution,
        newTokensDestinationIdentity: identifier,
        newTokensDestinationIdentityRules: changeRules,
        mintingAllowChoosingDestination: true,
        mintingAllowChoosingDestinationRules: changeRules,
        changeDirectPurchasePricingRules: changeRules,
      });

      expect(distributionRules).to.be.an.instanceof(wasm.TokenDistributionRules);
      expect(perpetualDistribution).to.be.an.instanceof(wasm.TokenPerpetualDistribution);
      expect(preProgrammedDistribution).to.be.an.instanceof(wasm.TokenPreProgrammedDistribution);
      expect(changeRules).to.be.an.instanceof(wasm.ChangeControlRules);
    });
  });

  describe('getters', () => {
    it('should allow to get values', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );

      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const perpetualDistribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      const distributionRules = new wasm.TokenDistributionRules({
        perpetualDistribution,
        perpetualDistributionRules: changeRules,
        preProgrammedDistribution,
        newTokensDestinationIdentity: identifier,
        newTokensDestinationIdentityRules: changeRules,
        mintingAllowChoosingDestination: true,
        mintingAllowChoosingDestinationRules: changeRules,
        changeDirectPurchasePricingRules: changeRules,
      });

      expect(distributionRules.perpetualDistribution.constructor.name).to.deep.equal('TokenPerpetualDistribution');
      expect(distributionRules.perpetualDistributionRules.constructor.name).to.deep.equal('ChangeControlRules');
      expect(distributionRules.preProgrammedDistribution.constructor.name).to.deep.equal('TokenPreProgrammedDistribution');
      expect(distributionRules.newTokensDestinationIdentity.constructor.name).to.deep.equal('Identifier');
      expect(distributionRules.newTokensDestinationIdentityRules.constructor.name).to.deep.equal('ChangeControlRules');
      expect(distributionRules.mintingAllowChoosingDestination).to.deep.equal(true);
      expect(distributionRules.mintingAllowChoosingDestinationRules.constructor.name).to.deep.equal('ChangeControlRules');
      expect(distributionRules.changeDirectPurchasePricingRules.constructor.name).to.deep.equal('ChangeControlRules');
    });
  });

  describe('setters', () => {
    let noOne: unknown;
    let changeRules: unknown;
    let preProgrammedDistribution: unknown;
    let recipient: unknown;
    let distributionFunction: unknown;
    let distributionType: unknown;
    let perpetualDistribution: unknown;
    let distributionRules: InstanceType<typeof wasm.TokenDistributionRules>;

    before(() => {
      noOne = wasm.AuthorizedActionTakers.NoOne();
      changeRules = createChangeControlRules(noOne, noOne);
      preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      recipient = wasm.TokenDistributionRecipient.ContractOwner();
      distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );
      distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );
      perpetualDistribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );
      distributionRules = new wasm.TokenDistributionRules({
        perpetualDistribution,
        perpetualDistributionRules: changeRules,
        preProgrammedDistribution,
        newTokensDestinationIdentity: identifier,
        newTokensDestinationIdentityRules: changeRules,
        mintingAllowChoosingDestination: true,
        mintingAllowChoosingDestinationRules: changeRules,
        changeDirectPurchasePricingRules: changeRules,
      });
    });

    it('should allow to set mintingAllowChoosingDestination', () => {
      distributionRules.mintingAllowChoosingDestination = false;

      expect(distributionRules.mintingAllowChoosingDestination).to.deep.equal(false);
    });

    it('should allow to set changeDirectPurchasePricingRules', () => {
      const newRules = createChangeControlRules(noOne, noOne, {
        isChangingAuthorizedActionTakersToNoOneAllowed: false,
        isChangingAdminActionTakersToNoOneAllowed: false,
        isSelfChangingAdminActionTakersAllowed: false,
      });

      distributionRules.changeDirectPurchasePricingRules = newRules;

      expect(newRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(distributionRules.changeDirectPurchasePricingRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.changeDirectPurchasePricingRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.changeDirectPurchasePricingRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set mintingAllowChoosingDestinationRules', () => {
      const newRules = createChangeControlRules(noOne, noOne, {
        isChangingAuthorizedActionTakersToNoOneAllowed: false,
        isChangingAdminActionTakersToNoOneAllowed: false,
        isSelfChangingAdminActionTakersAllowed: false,
      });

      distributionRules.mintingAllowChoosingDestinationRules = newRules;

      expect(newRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(distributionRules.mintingAllowChoosingDestinationRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.mintingAllowChoosingDestinationRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.mintingAllowChoosingDestinationRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set newTokensDestinationIdentityRules', () => {
      const newRules = createChangeControlRules(noOne, noOne, {
        isChangingAuthorizedActionTakersToNoOneAllowed: false,
        isChangingAdminActionTakersToNoOneAllowed: false,
        isSelfChangingAdminActionTakersAllowed: false,
      });

      distributionRules.newTokensDestinationIdentityRules = newRules;

      expect(newRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(distributionRules.newTokensDestinationIdentityRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.newTokensDestinationIdentityRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.newTokensDestinationIdentityRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set newTokensDestinationIdentity', () => {
      distributionRules.newTokensDestinationIdentity = '12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

      expect(distributionRules.newTokensDestinationIdentity.toBase58()).to.deep.equal('12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
    });

    it('should allow to set preProgrammedDistribution', () => {
      const newPreProgrammedDistribution = createPreProgrammedDistribution(
        1750140416411,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10011120),
      );

      distributionRules.preProgrammedDistribution = newPreProgrammedDistribution;

      expect(newPreProgrammedDistribution).to.be.an.instanceof(wasm.TokenPreProgrammedDistribution);
      // Just check it's a map-like structure
      expect(distributionRules.preProgrammedDistribution.distributions).to.not.equal(undefined);
    });

    it('should allow to set perpetualDistributionRules', () => {
      const newPerpetualDistributionRules = createChangeControlRules(noOne, noOne, {
        isChangingAuthorizedActionTakersToNoOneAllowed: false,
        isChangingAdminActionTakersToNoOneAllowed: false,
        isSelfChangingAdminActionTakersAllowed: false,
      });

      distributionRules.perpetualDistributionRules = newPerpetualDistributionRules;

      expect(newPerpetualDistributionRules).to.be.an.instanceof(wasm.ChangeControlRules);
      expect(distributionRules.perpetualDistributionRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set perpetualDistribution', () => {
      const newRecipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      const newPerpetualDistribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        newRecipient,
      );

      distributionRules.perpetualDistribution = newPerpetualDistribution;

      expect(newPerpetualDistribution).to.be.an.instanceof(wasm.TokenPerpetualDistribution);
      expect(distributionRules.perpetualDistribution.distributionRecipient.recipientType).to.deep.equal('EvonodesByParticipation');
    });
  });
});
