import getWasm from './helpers/wasm.js';
import { identifier } from './mocks/Identity/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

// Helper function to create ChangeControlRules with default options
function createChangeControlRules(authorizedToMakeChange, adminActionTakers, options = {}) {
  return new wasm.ChangeControlRules({
    authorizedToMakeChange,
    adminActionTakers,
    isChangingAuthorizedActionTakersToNoOneAllowed: options.isChangingAuthorizedActionTakersToNoOneAllowed ?? true,
    isChangingAdminActionTakersToNoOneAllowed: options.isChangingAdminActionTakersToNoOneAllowed ?? true,
    isSelfChangingAdminActionTakersAllowed: options.isSelfChangingAdminActionTakersAllowed ?? true,
  });
}

// Helper to create pre-programmed distribution with proper Map format
function createPreProgrammedDistribution(timestamp, identifierBase58, amount) {
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

      expect(distributionRules.__wbg_ptr).to.not.equal(0);
      expect(changeRules.__wbg_ptr).to.not.equal(0);
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

      expect(distributionRules.__wbg_ptr).to.not.equal(0);
      expect(perpetualDistribution.__wbg_ptr).to.not.equal(0);
      expect(preProgrammedDistribution.__wbg_ptr).to.not.equal(0);
      expect(changeRules.__wbg_ptr).to.not.equal(0);
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
      expect(distributionRules.newTokenDestinationIdentity.constructor.name).to.deep.equal('Identifier');
      expect(distributionRules.newTokenDestinationIdentityRules.constructor.name).to.deep.equal('ChangeControlRules');
      expect(distributionRules.isMintingAllowingChoosingDestination).to.deep.equal(true);
      expect(distributionRules.mintingAllowChoosingDestinationRules.constructor.name).to.deep.equal('ChangeControlRules');
      expect(distributionRules.changeDirectPurchasePricingRules.constructor.name).to.deep.equal('ChangeControlRules');
    });
  });

  describe('setters', () => {
    let noOne;

    let changeRules;

    let preProgrammedDistribution;

    let recipient;

    let distributionFunction;

    let distributionType;

    let perpetualDistribution;

    let distributionRules;

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
      distributionRules.isMintingAllowingChoosingDestination = false;

      expect(distributionRules.isMintingAllowingChoosingDestination).to.deep.equal(false);
    });

    it('should allow to set changeDirectPurchasePricingRules', () => {
      const newRules = createChangeControlRules(noOne, noOne, {
        isChangingAuthorizedActionTakersToNoOneAllowed: false,
        isChangingAdminActionTakersToNoOneAllowed: false,
        isSelfChangingAdminActionTakersAllowed: false,
      });

      distributionRules.changeDirectPurchasePricingRules = newRules;

      expect(newRules.__wbg_ptr).to.not.equal(0);
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

      expect(newRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.mintingAllowChoosingDestinationRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.mintingAllowChoosingDestinationRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.mintingAllowChoosingDestinationRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set newTokenDestinationIdentityRules', () => {
      const newRules = createChangeControlRules(noOne, noOne, {
        isChangingAuthorizedActionTakersToNoOneAllowed: false,
        isChangingAdminActionTakersToNoOneAllowed: false,
        isSelfChangingAdminActionTakersAllowed: false,
      });

      distributionRules.newTokenDestinationIdentityRules = newRules;

      expect(newRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.newTokenDestinationIdentityRules.isSelfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.newTokenDestinationIdentityRules.isChangingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.newTokenDestinationIdentityRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set newTokenDestinationIdentity', () => {
      distributionRules.newTokenDestinationIdentity = '12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

      expect(distributionRules.newTokenDestinationIdentity.toBase58()).to.deep.equal('12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
    });

    it('should allow to set preProgrammedDistribution', () => {
      const newPreProgrammedDistribution = createPreProgrammedDistribution(
        1750140416411,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10011120),
      );

      distributionRules.preProgrammedDistribution = newPreProgrammedDistribution;

      expect(newPreProgrammedDistribution.__wbg_ptr).to.not.equal(0);
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

      expect(newPerpetualDistributionRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.perpetualDistributionRules.isChangingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set perpetualDistribution', () => {
      const newRecipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      const newPerpetualDistribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        newRecipient,
      );

      distributionRules.perpetualDistribution = newPerpetualDistribution;

      expect(newPerpetualDistribution.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.perpetualDistribution.distributionRecipient.recipientType).to.deep.equal('EvonodesByParticipation');
    });
  });
});
