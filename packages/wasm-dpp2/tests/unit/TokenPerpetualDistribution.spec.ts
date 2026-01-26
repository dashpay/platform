import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('TokenPerpetualDistribution', () => {
  describe('constructor', () => {
    it('should create instance from values', () => {
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

  describe('distributionType', () => {
    it('should return distributionType', () => {
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

    it('should set distributionType', () => {
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
  });

  describe('distributionRecipient', () => {
    it('should return distributionRecipient', () => {
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

    it('should set distributionRecipient', () => {
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
