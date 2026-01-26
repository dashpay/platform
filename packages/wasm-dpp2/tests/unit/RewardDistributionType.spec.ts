import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';


before(async () => {
  await initWasm();
});

describe('RewardDistributionType', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create BlockBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
      expect(distributionType).to.be.an.instanceof(wasm.RewardDistributionType);
    });

    it('should allow to create TimeBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
      expect(distributionType).to.be.an.instanceof(wasm.RewardDistributionType);
    });

    it('should allow to create EpochBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.EpochBasedDistribution(
        111,
        distributionFunction,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
      expect(distributionType).to.be.an.instanceof(wasm.RewardDistributionType);
    });
  });

  describe('getters', () => {
    it('should allow return value BlockBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.distribution.constructor.name).to.equal('BlockBasedDistribution');
    });

    it('should allow return value TimeBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.distribution.constructor.name).to.equal('TimeBasedDistribution');
    });

    it('should allow return value EpochBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.EpochBasedDistribution(
        111,
        distributionFunction,
      );

      expect(distributionType.distribution.constructor.name).to.equal('EpochBasedDistribution');
    });
  });
});
