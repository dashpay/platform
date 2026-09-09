import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('FeeStrategyStep', () => {
  describe('deductFromInput()', () => {
    it('should create a deductFromInput step with the given index', () => {
      const step = wasm.FeeStrategyStep.deductFromInput(2);

      expect(step).to.exist();
      expect(step).to.be.an.instanceof(wasm.FeeStrategyStep);
      expect(step.isDeductFromInput).to.be.true();
      expect(step.isReduceOutput).to.be.false();
      expect(step.index).to.equal(2);
    });

    it('should accept index 0', () => {
      const step = wasm.FeeStrategyStep.deductFromInput(0);
      expect(step.index).to.equal(0);
    });

    it('should accept large indices up to u16::MAX', () => {
      const step = wasm.FeeStrategyStep.deductFromInput(65535);
      expect(step.index).to.equal(65535);
    });
  });

  describe('reduceOutput()', () => {
    it('should create a reduceOutput step with the given index', () => {
      const step = wasm.FeeStrategyStep.reduceOutput(7);

      expect(step.isReduceOutput).to.be.true();
      expect(step.isDeductFromInput).to.be.false();
      expect(step.index).to.equal(7);
    });

    it('should accept index 0', () => {
      const step = wasm.FeeStrategyStep.reduceOutput(0);
      expect(step.index).to.equal(0);
    });
  });

  describe('use as constructor argument to AddressFundsTransferTransition', () => {
    const addrBytes = new Uint8Array([
      0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ]);

    function makeTransition(steps: any[]) {
      const inputAddr = wasm.PlatformAddress.fromBytes(addrBytes);
      const outputAddr = wasm.PlatformAddress.fromBytes(addrBytes);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      return new wasm.AddressFundsTransferTransition({
        inputs: [input],
        outputs: [output],
        feeStrategy: steps,
      });
    }

    it('emits {$type: "deductFromInput", index} in toObject() output', () => {
      const transition = makeTransition([wasm.FeeStrategyStep.deductFromInput(0)]);
      const obj = transition.toObject();

      expect(obj.feeStrategy).to.be.an('array').with.lengthOf(1);
      expect(obj.feeStrategy[0]).to.deep.equal({ $type: 'deductFromInput', index: 0 });
    });

    it('emits {$type: "reduceOutput", index} in toObject() output', () => {
      const transition = makeTransition([wasm.FeeStrategyStep.reduceOutput(3)]);
      const obj = transition.toObject();

      expect(obj.feeStrategy).to.deep.equal([{ $type: 'reduceOutput', index: 3 }]);
    });

    it('emits {type, index} in toJSON() output (matches Object form for this enum)', () => {
      const transition = makeTransition([
        wasm.FeeStrategyStep.deductFromInput(1),
        wasm.FeeStrategyStep.reduceOutput(2),
      ]);
      const json = transition.toJSON();

      expect(json.feeStrategy).to.deep.equal([
        { $type: 'deductFromInput', index: 1 },
        { $type: 'reduceOutput', index: 2 },
      ]);
    });

    it('round-trips through fromObject(toObject())', () => {
      const transition = makeTransition([
        wasm.FeeStrategyStep.deductFromInput(0),
        wasm.FeeStrategyStep.reduceOutput(0),
      ]);
      const obj = transition.toObject();
      const restored = wasm.AddressFundsTransferTransition.fromObject(obj);
      expect(restored.toObject().feeStrategy).to.deep.equal(obj.feeStrategy);
    });

    it('round-trips through fromJSON(toJSON())', () => {
      const transition = makeTransition([wasm.FeeStrategyStep.reduceOutput(0)]);
      const json = transition.toJSON();
      const restored = wasm.AddressFundsTransferTransition.fromJSON(json);
      expect(restored.toJSON().feeStrategy).to.deep.equal(json.feeStrategy);
    });
  });
});
