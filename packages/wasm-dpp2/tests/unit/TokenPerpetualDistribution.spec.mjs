import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('TokenPerpetualDistribution', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create from values', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const distribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      expect(recipient.__wbg_ptr).to.not.equal(0);
      expect(distributionFunction.__wbg_ptr).to.not.equal(0);
      expect(distributionType.__wbg_ptr).to.not.equal(0);
      expect(distribution.__wbg_ptr).to.not.equal(0);
    });
  });

  describe('getters', () => {
    it('should allow to get distributionType', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const distribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      expect(distribution.distributionType.constructor.name).to.deep.equal('RewardDistributionType');
    });

    it('should allow to get distributionRecipient', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const distribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      expect(distribution.distributionRecipient.constructor.name).to.deep.equal('TokenDistributionRecipient');
      expect(distribution.distributionRecipient.getType()).to.deep.equal('ContractOwner');
    });
  });

  describe('setters', () => {
    it('should allow to set distributionType', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const distribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      const newDistribution = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      distribution.distributionType = newDistribution;

      expect(newDistribution.__wbg_ptr).to.not.equal(0);
      expect(distribution.distributionType.constructor.name).to.deep.equal('RewardDistributionType');
      expect(distribution.distributionType.getDistribution().constructor.name).to.deep.equal('TimeBasedDistribution');
    });

    it('should allow to set distributionRecipient', () => {
      const recipient = wasm.TokenDistributionRecipient.ContractOwner();

      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      const distribution = new wasm.TokenPerpetualDistribution(
        distributionType,
        recipient,
      );

      const newRecipient = wasm.TokenDistributionRecipient.EvonodesByParticipation();

      distribution.distributionRecipient = newRecipient;

      expect(newRecipient.__wbg_ptr).to.not.equal(0);
      expect(distribution.distributionRecipient.constructor.name).to.deep.equal('TokenDistributionRecipient');
      expect(distribution.distributionRecipient.getType()).to.deep.equal('EvonodesByParticipation');
    });
  });
});
