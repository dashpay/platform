import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('DistributionFunction', () => {
  describe('FixedAmountDistribution()', () => {
    it('should create FixedAmountDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return FixedAmount for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      expect(distributionFunction.functionName).to.deep.equal('FixedAmount');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      expect(distributionFunction.functionValue.amount).to.deep.equal(111n);
    });
  });

  describe('Random()', () => {
    it('should create Random', () => {
      const distributionFunction = wasm.DistributionFunction.Random(
        BigInt(111),
        BigInt(113),
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return Random for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.Random(
        BigInt(111),
        BigInt(113),
      );

      expect(distributionFunction.functionName).to.deep.equal('Random');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.Random(
        BigInt(111),
        BigInt(113),
      );

      expect(distributionFunction.functionValue.min).to.deep.equal(111n);
      expect(distributionFunction.functionValue.max).to.deep.equal(113n);
    });
  });

  describe('StepDecreasingAmount()', () => {
    it('should create StepDecreasingAmount', () => {
      const distributionFunction = wasm.DistributionFunction.StepDecreasingAmount(
        11,
        11,
        11,
        undefined,
        undefined,
        BigInt(111),
        BigInt(113),
        BigInt(1),
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return StepDecreasingAmount for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.StepDecreasingAmount(
        11,
        11,
        11,
        undefined,
        undefined,
        BigInt(111),
        BigInt(113),
        BigInt(1),
      );

      expect(distributionFunction.functionName).to.deep.equal('StepDecreasingAmount');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.StepDecreasingAmount(
        11,
        11,
        11,
        undefined,
        undefined,
        BigInt(111),
        BigInt(113),
        BigInt(1),
      );

      expect(distributionFunction.functionValue.stepCount).to.deep.equal(11);
      expect(distributionFunction.functionValue.decreasePerIntervalNumerator).to.deep.equal(11);
      expect(distributionFunction.functionValue.decreasePerIntervalDenominator).to.deep.equal(11);
      expect(distributionFunction.functionValue.startDecreasingOffset).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.maxIntervalCount).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.distributionStartAmount).to.deep.equal(111n);
      expect(distributionFunction.functionValue.trailingDistributionIntervalAmount).to.deep.equal(113n);
      expect(distributionFunction.functionValue.minValue).to.deep.equal(1n);
    });
  });

  describe('Stepwise()', () => {
    it('should create Stepwise', () => {
      const distributionFunction = wasm.DistributionFunction.Stepwise(
        {
          11111111121: BigInt(111),
        },
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return Stepwise for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.Stepwise(
        {
          11111111121: BigInt(111),
        },
      );

      expect(distributionFunction.functionName).to.deep.equal('Stepwise');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.Stepwise(
        {
          11111111121: BigInt(111),
        },
      );

      expect(distributionFunction.functionValue).to.deep.equal({
        11111111121: BigInt(111),
      });
    });
  });

  describe('Linear()', () => {
    it('should create Linear', () => {
      const distributionFunction = wasm.DistributionFunction.Linear(
        BigInt(111),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return Linear for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.Linear(
        BigInt(111),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionName).to.deep.equal('Linear');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.Linear(
        BigInt(111),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionValue.a).to.deep.equal(111n);
      expect(distributionFunction.functionValue.d).to.deep.equal(113n);
      expect(distributionFunction.functionValue.startStep).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.startingAmount).to.deep.equal(113n);
      expect(distributionFunction.functionValue.minValue).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.maxValue).to.deep.equal(undefined);
    });
  });

  describe('Polynomial()', () => {
    it('should create Polynomial', () => {
      const distributionFunction = wasm.DistributionFunction.Polynomial(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return Polynomial for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.Polynomial(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionName).to.deep.equal('Polynomial');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.Polynomial(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionValue.a).to.deep.equal(111n);
      expect(distributionFunction.functionValue.d).to.deep.equal(113n);
      expect(distributionFunction.functionValue.m).to.deep.equal(113n);
      expect(distributionFunction.functionValue.n).to.deep.equal(113n);
      expect(distributionFunction.functionValue.o).to.deep.equal(113n);
      expect(distributionFunction.functionValue.startMoment).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.b).to.deep.equal(113n);
      expect(distributionFunction.functionValue.minValue).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.maxValue).to.deep.equal(undefined);
    });
  });

  describe('Exponential()', () => {
    it('should create Exponential', () => {
      const distributionFunction = wasm.DistributionFunction.Exponential(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return Exponential for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.Exponential(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionName).to.deep.equal('Exponential');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.Exponential(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionValue.a).to.deep.equal(111n);
      expect(distributionFunction.functionValue.d).to.deep.equal(113n);
      expect(distributionFunction.functionValue.m).to.deep.equal(113n);
      expect(distributionFunction.functionValue.n).to.deep.equal(113n);
      expect(distributionFunction.functionValue.o).to.deep.equal(113n);
      expect(distributionFunction.functionValue.startMoment).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.b).to.deep.equal(113n);
      expect(distributionFunction.functionValue.minValue).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.maxValue).to.deep.equal(undefined);
    });
  });

  describe('Logarithmic()', () => {
    it('should create Logarithmic', () => {
      const distributionFunction = wasm.DistributionFunction.Logarithmic(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return Logarithmic for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.Logarithmic(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionName).to.deep.equal('Logarithmic');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.Logarithmic(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionValue.a).to.deep.equal(111n);
      expect(distributionFunction.functionValue.d).to.deep.equal(113n);
      expect(distributionFunction.functionValue.m).to.deep.equal(113n);
      expect(distributionFunction.functionValue.n).to.deep.equal(113n);
      expect(distributionFunction.functionValue.o).to.deep.equal(113n);
      expect(distributionFunction.functionValue.startMoment).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.b).to.deep.equal(113n);
      expect(distributionFunction.functionValue.minValue).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.maxValue).to.deep.equal(undefined);
    });
  });

  describe('InvertedLogarithmic()', () => {
    it('should create InvertedLogarithmic', () => {
      const distributionFunction = wasm.DistributionFunction.InvertedLogarithmic(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should return InvertedLogarithmic for functionName', () => {
      const distributionFunction = wasm.DistributionFunction.InvertedLogarithmic(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionName).to.deep.equal('InvertedLogarithmic');
    });

    it('should return correct functionValue', () => {
      const distributionFunction = wasm.DistributionFunction.InvertedLogarithmic(
        BigInt(111),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined,
      );

      expect(distributionFunction.functionValue.a).to.deep.equal(111n);
      expect(distributionFunction.functionValue.d).to.deep.equal(113n);
      expect(distributionFunction.functionValue.m).to.deep.equal(113n);
      expect(distributionFunction.functionValue.n).to.deep.equal(113n);
      expect(distributionFunction.functionValue.o).to.deep.equal(113n);
      expect(distributionFunction.functionValue.startMoment).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.b).to.deep.equal(113n);
      expect(distributionFunction.functionValue.minValue).to.deep.equal(undefined);
      expect(distributionFunction.functionValue.maxValue).to.deep.equal(undefined);
    });
  });
});
