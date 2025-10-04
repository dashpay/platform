import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('RewardDistributionType', () => {
  describe('serialization / deserialization', () => {
    it('shoulda allow to create BlockBasedDistribution', () => {
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

    it('shoulda allow to create TimeBasedDistribution', () => {
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

    it('shoulda allow to create EpochBasedDistribution', () => {
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
    it('shoulda allow return value BlockBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.BlockBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.getDistribution().constructor.name).to.equal('BlockBasedDistribution');
    });

    it('shoulda allow return value TimeBasedDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      const distributionType = wasm.RewardDistributionType.TimeBasedDistribution(
        BigInt(111),
        distributionFunction,
      );

      expect(distributionType.getDistribution().constructor.name).to.equal('TimeBasedDistribution');
    });

    it('shoulda allow return value EpochBasedDistribution', () => {
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
