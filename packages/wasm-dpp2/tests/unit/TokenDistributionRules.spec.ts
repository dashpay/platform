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
  describe('constructor', () => {
    it('should create instance with undefined values', () => {
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

    it('should create instance without undefined values', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();

      const changeRules = createChangeControlRules(noOne, noOne);

      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );

      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        new wasm.DistributionFixedAmount({ amount: BigInt(111) }),
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

  describe('perpetualDistribution', () => {
    it('should return perpetualDistribution', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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
    });

    it('should set perpetualDistribution', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      const newRecipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();
      const newPerpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, newRecipient);

      distributionRules.perpetualDistribution = newPerpetualDistribution;

      expect(newPerpetualDistribution).to.be.an.instanceof(wasm.TokenPerpetualDistribution);
      expect(distributionRules.perpetualDistribution.distributionRecipient.recipientType).to.deep.equal('EvonodesByParticipation');
    });
  });

  describe('mintingAllowChoosingDestination', () => {
    it('should return mintingAllowChoosingDestination', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      expect(distributionRules.mintingAllowChoosingDestination).to.deep.equal(true);
    });

    it('should set mintingAllowChoosingDestination', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      distributionRules.mintingAllowChoosingDestination = false;

      expect(distributionRules.mintingAllowChoosingDestination).to.deep.equal(false);
    });
  });

  describe('newTokensDestinationIdentity', () => {
    it('should return newTokensDestinationIdentity', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      expect(distributionRules.newTokensDestinationIdentity.constructor.name).to.deep.equal('Identifier');
    });

    it('should set newTokensDestinationIdentity', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      distributionRules.newTokensDestinationIdentity = '12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

      expect(distributionRules.newTokensDestinationIdentity.toBase58()).to.deep.equal('12p3355tKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1');
    });
  });

  describe('preProgrammedDistribution', () => {
    it('should return preProgrammedDistribution', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      expect(distributionRules.preProgrammedDistribution.constructor.name).to.deep.equal('TokenPreProgrammedDistribution');
    });

    it('should set preProgrammedDistribution', () => {
      const noOne = wasm.AuthorizedActionTakers.NoOne();
      const changeRules = createChangeControlRules(noOne, noOne);
      const preProgrammedDistribution = createPreProgrammedDistribution(
        1750140416485,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10000),
      );
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();
      const fixedAmount = new wasm.DistributionFixedAmount({ amount: BigInt(111) });
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(fixedAmount);
      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(BigInt(111), distributionFunction);
      const perpetualDistribution = new wasm.TokenPerpetualDistribution(distributionType, recipient);

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

      const newPreProgrammedDistribution = createPreProgrammedDistribution(
        1750140416411,
        'PJUBWbXWmzEYCs99rAAbnCiHRzrnhKLQrXbmSsuPBYB',
        BigInt(10011120),
      );

      distributionRules.preProgrammedDistribution = newPreProgrammedDistribution;

      expect(newPreProgrammedDistribution).to.be.an.instanceof(wasm.TokenPreProgrammedDistribution);
      expect(distributionRules.preProgrammedDistribution.distributions).to.not.equal(undefined);
    });
  });
});
