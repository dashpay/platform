import getWasm from './helpers/wasm.js';
import { identifier } from './mocks/Identity/index.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenDistributionRules', () => {
  describe('serialization / deserialization', () => {
    it('shoulda allow to create with undefined values', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );

      const distributionRules = new wasm.TokenDistributionRulesWASM(
        undefined,
        changeRules,
        undefined,
        undefined,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      expect(distributionRules.__wbg_ptr).to.not.equal(0);
      expect(changeRules.__wbg_ptr).to.not.equal(0);
    });

    it('shoulda allow to create without undefined values', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
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

      const recipient = wasm.TokenDistributionRecipientWASM.ContractOwner();

      const distributionFunction = wasm.DistributionFunctionWASM.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionTypeWASM.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const perpetualDistribution = new wasm.TokenPerpetualDistributionWASM(
        distributionType,
        recipient,
      );

      const distributionRules = new wasm.TokenDistributionRulesWASM(
        perpetualDistribution,
        changeRules,
        preProgrammedDistribution,
        identifier,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      expect(distributionRules.__wbg_ptr).to.not.equal(0);
      expect(perpetualDistribution.__wbg_ptr).to.not.equal(0);
      expect(preProgrammedDistribution.__wbg_ptr).to.not.equal(0);
      expect(changeRules.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('shoulda allow to get values', () => {
      const noOne = wasm.AuthorizedActionTakersWASM.NoOne();

      const changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
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

      const recipient = wasm.TokenDistributionRecipientWASM.ContractOwner();

      const distributionFunction = wasm.DistributionFunctionWASM.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionTypeWASM.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const perpetualDistribution = new wasm.TokenPerpetualDistributionWASM(
        distributionType,
        recipient,
      );

      const distributionRules = new wasm.TokenDistributionRulesWASM(
        perpetualDistribution,
        changeRules,
        preProgrammedDistribution,
        identifier,
        changeRules,
        true,
        changeRules,
        changeRules,
      );

      expect(distributionRules.perpetualDistribution.constructor.name).to.deep.equal('TokenPerpetualDistributionWASM');
      expect(distributionRules.perpetualDistributionRules.constructor.name).to.deep.equal('ChangeControlRulesWASM');
      expect(distributionRules.preProgrammedDistribution.constructor.name).to.deep.equal('TokenPreProgrammedDistributionWASM');
      expect(distributionRules.newTokenDestinationIdentity.constructor.name).to.deep.equal('IdentifierWASM');
      expect(distributionRules.newTokenDestinationIdentityRules.constructor.name).to.deep.equal('ChangeControlRulesWASM');
      expect(distributionRules.mintingAllowChoosingDestination).to.deep.equal(true);
      expect(distributionRules.mintingAllowChoosingDestinationRules.constructor.name).to.deep.equal('ChangeControlRulesWASM');
      expect(distributionRules.changeDirectPurchasePricingRules.constructor.name).to.deep.equal('ChangeControlRulesWASM');
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
      noOne = wasm.AuthorizedActionTakersWASM.NoOne();
      changeRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        true,
        true,
        true,
      );
      preProgrammedDistribution = new wasm.TokenPreProgrammedDistributionWASM(
        {
          1750140416485: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10000),
          },
        },
      );
      recipient = wasm.TokenDistributionRecipientWASM.ContractOwner();
      distributionFunction = wasm.DistributionFunctionWASM.FixedAmountDistribution(
        BigInt(111),
      );
      distributionType = wasm.RewardDistributionTypeWASM.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );
      perpetualDistribution = new wasm.TokenPerpetualDistributionWASM(
        distributionType,
        recipient,
      );
      distributionRules = new wasm.TokenDistributionRulesWASM(
        perpetualDistribution,
        changeRules,
        preProgrammedDistribution,
        identifier,
        changeRules,
        true,
        changeRules,
        changeRules,
      );
    });

    it('should allow to set mintingAllowChoosingDestination', () => {
      distributionRules.mintingAllowChoosingDestination = false;

      expect(distributionRules.mintingAllowChoosingDestination).to.deep.equal(false);
    });

    it('should allow to set changeDirectPurchasePricingRules', () => {
      const newRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        false,
        false,
        false,
      );

      distributionRules.changeDirectPurchasePricingRules = newRules;

      expect(newRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.changeDirectPurchasePricingRules.selfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.changeDirectPurchasePricingRules.changingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.changeDirectPurchasePricingRules.changingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set mintingAllowChoosingDestinationRules', () => {
      const newRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        false,
        false,
        false,
      );

      distributionRules.mintingAllowChoosingDestinationRules = newRules;

      expect(newRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.mintingAllowChoosingDestinationRules.selfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.mintingAllowChoosingDestinationRules.changingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.mintingAllowChoosingDestinationRules.changingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set newTokenDestinationIdentityRules', () => {
      const newRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        false,
        false,
        false,
      );

      distributionRules.newTokenDestinationIdentityRules = newRules;

      expect(newRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.newTokenDestinationIdentityRules.selfChangingAdminActionTakersAllowed).to.deep.equal(false);
      expect(distributionRules.newTokenDestinationIdentityRules.changingAdminActionTakersToNoOneAllowed).to.deep.equal(false);
      expect(distributionRules.newTokenDestinationIdentityRules.changingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set newTokenDestinationIdentity', () => {
      distributionRules.newTokenDestinationIdentity = '12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

      expect(distributionRules.newTokenDestinationIdentity.base58()).to.deep.equal('12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
    });

    it('should allow to set preProgrammedDistribution', () => {
      const newPreProgrammedDistribution = new wasm.TokenPreProgrammedDistributionWASM(
        {
          1750140416411: {
            PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10011120),
          },
        },
      );

      distributionRules.preProgrammedDistribution = newPreProgrammedDistribution;

      expect(newPreProgrammedDistribution.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.preProgrammedDistribution.distributions).to.deep.equal({
        1750140416411: {
          PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB: BigInt(10011120),
        },
      });
    });

    it('should allow to set perpetualDistributionRules', () => {
      const newPerpetualDistributionRules = new wasm.ChangeControlRulesWASM(
        noOne,
        noOne,
        false,
        false,
        false,
      );

      distributionRules.perpetualDistributionRules = newPerpetualDistributionRules;

      expect(newPerpetualDistributionRules.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.perpetualDistributionRules.changingAuthorizedActionTakersToNoOneAllowed).to.deep.equal(false);
    });

    it('should allow to set perpetualDistribution', () => {
      const newRecipient = wasm.TokenDistributionRecipientWASM.EvonodesByParticipation();

      const newPerpetualDistribution = new wasm.TokenPerpetualDistributionWASM(
        distributionType,
        newRecipient,
      );

      distributionRules.perpetualDistribution = newPerpetualDistribution;

      expect(newPerpetualDistribution.__wbg_ptr).to.not.equal(0);
      expect(distributionRules.perpetualDistribution.distributionRecipient.getType()).to.deep.equal('EvonodesByParticipation');
    });
  });
});
