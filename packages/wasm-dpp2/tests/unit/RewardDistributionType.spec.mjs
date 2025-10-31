import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
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

      expect(distributionFunction.__wbg_ptr).to.not.equal(0);
      expect(distributionType.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create TimeBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionFunction.__wbg_ptr).to.not.equal(0);
      expect(distributionType.__wbg_ptr).to.not.equal(0);
    });

    it('should allow to create EpochBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.EpochBasedDistribution(
        111,
        distributionFunction,
      );

      expect(distributionFunction.__wbg_ptr).to.not.equal(0);
      expect(distributionType.__wbg_ptr).to.not.equal(0);
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

      expect(distributionType.getDistribution().constructor.name).to.equal('BlockBasedDistribution');
    });

    it('should allow return value TimeBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.getDistribution().constructor.name).to.equal('TimeBasedDistribution');
    });

    it('should allow return value EpochBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.EpochBasedDistribution(
        111,
        distributionFunction,
      );

      expect(distributionType.getDistribution().constructor.name).to.equal('EpochBasedDistribution');
    });
  });
});
