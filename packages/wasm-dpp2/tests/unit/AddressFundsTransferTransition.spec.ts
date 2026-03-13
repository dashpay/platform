import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('AddressFundsTransferTransition', () => {
  const addr1Bytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ]);
  const addr2Bytes = new Uint8Array([
    0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
  ]);

  function createTransition() {
    const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
    const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

    const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
    const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

    return new wasm.AddressFundsTransferTransition({
      inputs: [input],
      outputs: [output],
    });
  }

  describe('constructor()', () => {
    it('should create transition with single input and output', () => {
      const transition = createTransition();
      expect(transition).to.exist();
      expect(transition).to.be.an.instanceof(wasm.AddressFundsTransferTransition);
    });

    it('should create transition with custom fee strategy', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(100000));

      const transition = new wasm.AddressFundsTransferTransition({
        inputs: [input],
        outputs: [output],
        feeStrategy: [wasm.FeeStrategyStep.reduceOutput(0)],
      });

      expect(transition).to.exist();
    });

    it('should create transition with user fee increase', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

      const transition = new wasm.AddressFundsTransferTransition({
        inputs: [input],
        outputs: [output],
        userFeeIncrease: 100,
      });

      expect(transition).to.exist();
      expect(transition.userFeeIncrease).to.equal(100);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes', () => {
      const transition = createTransition();
      const bytes = transition.toBytes();
      const restored = wasm.AddressFundsTransferTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toBase64() / fromBase64()', () => {
    it('should round-trip via base64', () => {
      const transition = createTransition();
      const base64 = transition.toBase64();
      const bytes = transition.toBytes();
      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.AddressFundsTransferTransition.fromBase64(base64);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toHex() / fromHex()', () => {
    it('should round-trip via hex', () => {
      const transition = createTransition();
      const hex = transition.toHex();
      const bytes = transition.toBytes();

      const restored = wasm.AddressFundsTransferTransition.fromHex(hex);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('inputs', () => {
    it('should return inputs array', () => {
      const transition = createTransition();
      const { inputs } = transition;
      expect(inputs).to.be.an('array');
      expect(inputs).to.have.lengthOf(1);
      expect(inputs[0].nonce).to.equal(0);
      expect(inputs[0].amount).to.equal(BigInt(100000));
    });

    it('should set inputs', () => {
      const transition = createTransition();
      const newAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);
      const newInput = new wasm.PlatformAddressInput(newAddr, 5, BigInt(50000));

      transition.inputs = [newInput];
      const { inputs } = transition;
      expect(inputs).to.have.lengthOf(1);
      expect(inputs[0].nonce).to.equal(5);
      expect(inputs[0].amount).to.equal(BigInt(50000));
    });
  });

  describe('outputs', () => {
    it('should return outputs array', () => {
      const transition = createTransition();
      const { outputs } = transition;
      expect(outputs).to.be.an('array');
      expect(outputs).to.have.lengthOf(1);
      expect(outputs[0].amount).to.equal(BigInt(90000));
    });

    it('should set outputs', () => {
      const transition = createTransition();
      const newAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const newOutput = new wasm.PlatformAddressOutput(newAddr, BigInt(80000));

      transition.outputs = [newOutput];
      const { outputs } = transition;
      expect(outputs).to.have.lengthOf(1);
      expect(outputs[0].amount).to.equal(BigInt(80000));
    });
  });

  describe('userFeeIncrease', () => {
    it('should return default userFeeIncrease', () => {
      const transition = createTransition();
      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should set userFeeIncrease', () => {
      const transition = createTransition();
      transition.userFeeIncrease = 42;
      expect(transition.userFeeIncrease).to.equal(42);
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from StateTransition', () => {
      const transition = createTransition();
      const st = transition.toStateTransition();
      expect(st).to.exist();

      const restored = wasm.AddressFundsTransferTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(
        Buffer.from(transition.toBytes()),
      );
    });
  });
});
