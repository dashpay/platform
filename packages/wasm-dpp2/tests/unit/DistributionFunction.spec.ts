import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('DistributionFunction', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create FixedAmountDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111),
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should allow to create Random', () => {
      const distributionFunction = wasm.DistributionFunction.Random(
        BigInt(111),
        BigInt(113),
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should allow to create StepDecreasingAmount', () => {
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

    it('should allow to create Stepwise', () => {
      const distributionFunction = wasm.DistributionFunction.Stepwise(
        {
          11111111121: BigInt(111),
        },
      );

      expect(distributionFunction).to.be.an.instanceof(wasm.DistributionFunction);
    });

    it('should allow to create Linear', () => {
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

    it('should allow to create Polynomial', () => {
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

    it('should allow to create Exponential', () => {
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

    it('should allow to create Logarithmic', () => {
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

    it('should allow to create InvertedLogarithmic', () => {
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
  });

  describe('getters', () => {
    describe('function name', () => {
      it('should get FixedAmountDistribution', () => {
        const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
          BigInt(111),
        );

        expect(distributionFunction.functionName).to.deep.equal('FixedAmount');
      });

      it('should get Random', () => {
        const distributionFunction = wasm.DistributionFunction.Random(
          BigInt(111),
          BigInt(113),
        );

        expect(distributionFunction.functionName).to.deep.equal('Random');
      });

      it('should get StepDecreasingAmount', () => {
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

      it('should get Stepwise', () => {
        const distributionFunction = wasm.DistributionFunction.Stepwise(
          {
            11111111121: BigInt(111),
          },
        );

        expect(distributionFunction.functionName).to.deep.equal('Stepwise');
      });

      it('should get Linear', () => {
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

      it('should get Polynomial', () => {
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

      it('should get Exponential', () => {
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

      it('should get Logarithmic', () => {
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

      it('should get InvertedLogarithmic', () => {
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
    });
    describe('function value', () => {
      it('should get FixedAmountDistribution', () => {
        const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
          BigInt(111),
        );

        expect(distributionFunction.functionValue.amount).to.deep.equal(111n);
      });

      it('should get Random', () => {
        const distributionFunction = wasm.DistributionFunction.Random(
          BigInt(111),
          BigInt(113),
        );

        expect(distributionFunction.functionValue.min).to.deep.equal(111n);
        expect(distributionFunction.functionValue.max).to.deep.equal(113n);
      });

      it('should get StepDecreasingAmount', () => {
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

      it('should get Stepwise', () => {
        const distributionFunction = wasm.DistributionFunction.Stepwise(
          {
            11111111121: BigInt(111),
          },
        );

        expect(distributionFunction.functionValue).to.deep.equal({
          11111111121: BigInt(111),
        });
      });

      it('should get Linear', () => {
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

      it('should get Polynomial', () => {
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

      it('should get Exponential', () => {
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

      it('should get Logarithmic', () => {
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

      it('should get InvertedLogarithmic', () => {
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
});
