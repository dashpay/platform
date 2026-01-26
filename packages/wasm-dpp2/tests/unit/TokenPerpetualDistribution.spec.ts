import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';


before(async () => {
  await initWasm();
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

      expect(recipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
      expect(distributionType).to.be.an.instanceof(wasm.RewardDistributionType);
      expect(distribution).to.be.an.instanceof(wasm.TokenPerpetualDistribution);
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
      expect(distribution.distributionRecipient.recipientType).to.deep.equal('ContractOwner');
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

      expect(newDistribution).to.be.an.instanceof(wasm.RewardDistributionType);
      expect(distribution.distributionType.constructor.name).to.deep.equal('RewardDistributionType');
      expect(distribution.distributionType.distribution.constructor.name).to.deep.equal('TimeBasedDistribution');
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

      expect(newRecipient).to.be.an.instanceof(wasm.TokenDistributionRecipient);
      expect(distribution.distributionRecipient.constructor.name).to.deep.equal('TokenDistributionRecipient');
      expect(distribution.distributionRecipient.recipientType).to.deep.equal('EvonodesByParticipation');
    });
  });
});
