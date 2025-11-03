import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('DistributionFunction', function () {
  describe('serialization / deserialization', function () {
    it('should allow to create FixedAmountDistribution', () => {
      const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
        BigInt(111)
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

    it('should allow to create Random', () => {
      const distributionFunction = wasm.DistributionFunction.Random(
        BigInt(111),
        BigInt(113)
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

    it('should allow to create StepDecreasingAmount', () => {
      const distributionFunction = wasm.DistributionFunction.StepDecreasingAmount(
        11,
        11,
        11,
        undefined,
        undefined,
        BigInt(111),
        BigInt(113),
        BigInt(1)
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

    it('should allow to create Stepwise', () => {
      const distributionFunction = wasm.DistributionFunction.Stepwise(
        {
          11111111121: BigInt(111)
        }
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

    it('should allow to create Linear', () => {
      const distributionFunction = wasm.DistributionFunction.Linear(
        BigInt(111),
        BigInt(113),
        undefined,
        BigInt(113),
        undefined,
        undefined
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

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
        undefined
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

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
        undefined
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

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
        undefined
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })

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
        undefined
      )

      expect(distributionFunction.__wbg_ptr).to.not.equal(0)
    })
  })

  describe('getters', function () {
    describe('function name', function () {
      it('FixedAmountDistribution', () => {
        const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
          BigInt(111)
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('FixedAmount')
      })

      it('Random', () => {
        const distributionFunction = wasm.DistributionFunction.Random(
          BigInt(111),
          BigInt(113)
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('Random')
      })

      it('StepDecreasingAmount', () => {
        const distributionFunction = wasm.DistributionFunction.StepDecreasingAmount(
          11,
          11,
          11,
          undefined,
          undefined,
          BigInt(111),
          BigInt(113),
          BigInt(1)
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('StepDecreasingAmount')
      })

      it('Stepwise', () => {
        const distributionFunction = wasm.DistributionFunction.Stepwise(
          {
            11111111121: BigInt(111)
          }
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('Stepwise')
      })

      it('Linear', () => {
        const distributionFunction = wasm.DistributionFunction.Linear(
          BigInt(111),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('Linear')
      })

      it('Polynomial', () => {
        const distributionFunction = wasm.DistributionFunction.Polynomial(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('Polynomial')
      })

      it('Exponential', () => {
        const distributionFunction = wasm.DistributionFunction.Exponential(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('Exponential')
      })

      it('Logarithmic', () => {
        const distributionFunction = wasm.DistributionFunction.Logarithmic(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('Logarithmic')
      })

      it('InvertedLogarithmic', () => {
        const distributionFunction = wasm.DistributionFunction.InvertedLogarithmic(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionName()).to.deep.equal('InvertedLogarithmic')
      })
    })
    describe('function value', function () {
      it('FixedAmountDistribution', () => {
        const distributionFunction = wasm.DistributionFunction.FixedAmountDistribution(
          BigInt(111)
        )

        expect(distributionFunction.getFunctionValue().amount).to.deep.equal(111n)
      })

      it('Random', () => {
        const distributionFunction = wasm.DistributionFunction.Random(
          BigInt(111),
          BigInt(113)
        )

        expect(distributionFunction.getFunctionValue().min).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().max).to.deep.equal(113n)
      })

      it('StepDecreasingAmount', () => {
        const distributionFunction = wasm.DistributionFunction.StepDecreasingAmount(
          11,
          11,
          11,
          undefined,
          undefined,
          BigInt(111),
          BigInt(113),
          BigInt(1)
        )

        expect(distributionFunction.getFunctionValue().stepCount).to.deep.equal(11)
        expect(distributionFunction.getFunctionValue().decreasePerIntervalNumerator).to.deep.equal(11)
        expect(distributionFunction.getFunctionValue().decreasePerIntervalDenominator).to.deep.equal(11)
        expect(distributionFunction.getFunctionValue().startDecreasingOffset).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().maxIntervalCount).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().distributionStartAmount).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().trailingDistributionIntervalAmount).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().minValue).to.deep.equal(1n)
      })

      it('Stepwise', () => {
        const distributionFunction = wasm.DistributionFunction.Stepwise(
          {
            11111111121: BigInt(111)
          }
        )

        expect(distributionFunction.getFunctionValue()).to.deep.equal({
          11111111121: BigInt(111)
        })
      })

      it('Linear', () => {
        const distributionFunction = wasm.DistributionFunction.Linear(
          BigInt(111),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionValue().a).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().d).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().startStep).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().startingAmount).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().minValue).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().maxValue).to.deep.equal(undefined)
      })

      it('Polynomial', () => {
        const distributionFunction = wasm.DistributionFunction.Polynomial(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionValue().a).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().d).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().m).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().n).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().o).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().startMoment).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().b).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().minValue).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().maxValue).to.deep.equal(undefined)
      })

      it('Exponential', () => {
        const distributionFunction = wasm.DistributionFunction.Exponential(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionValue().a).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().d).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().m).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().n).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().o).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().startMoment).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().b).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().minValue).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().maxValue).to.deep.equal(undefined)
      })

      it('Logarithmic', () => {
        const distributionFunction = wasm.DistributionFunction.Logarithmic(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionValue().a).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().d).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().m).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().n).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().o).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().startMoment).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().b).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().minValue).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().maxValue).to.deep.equal(undefined)
      })

      it('InvertedLogarithmic', () => {
        const distributionFunction = wasm.DistributionFunction.InvertedLogarithmic(
          BigInt(111),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          BigInt(113),
          undefined,
          BigInt(113),
          undefined,
          undefined
        )

        expect(distributionFunction.getFunctionValue().a).to.deep.equal(111n)
        expect(distributionFunction.getFunctionValue().d).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().m).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().n).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().o).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().startMoment).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().b).to.deep.equal(113n)
        expect(distributionFunction.getFunctionValue().minValue).to.deep.equal(undefined)
        expect(distributionFunction.getFunctionValue().maxValue).to.deep.equal(undefined)
      })
    })
  })
})
