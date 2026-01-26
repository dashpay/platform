import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('RewardDistributionType', () => {
  describe('BlockBasedDistribution()', () => {
    it('should create BlockBasedDistribution', () => {
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
  });

  describe('TimeBasedDistribution()', () => {
    it('should create TimeBasedDistribution', () => {
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
  });

  describe('EpochBasedDistribution()', () => {
    it('should create EpochBasedDistribution', () => {
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

  describe('distribution', () => {
    it('should return BlockBasedDistribution for block-based type', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.distribution.constructor.name).to.equal('BlockBasedDistribution');
    });

    it('should return TimeBasedDistribution for time-based type', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.distribution.constructor.name).to.equal('TimeBasedDistribution');
    });

    it('should return EpochBasedDistribution for epoch-based type', () => {
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
