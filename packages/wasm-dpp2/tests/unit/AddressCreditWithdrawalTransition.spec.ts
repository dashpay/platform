import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('AddressCreditWithdrawalTransition', () => {
  const addr1Bytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ]);
  const addr2Bytes = new Uint8Array([
    0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
  ]);

  function createTransition() {
    const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
    const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);
    const script = wasm.CoreScript.fromP2PKH([
      1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ]);

    const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
    const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));

    return new wasm.AddressCreditWithdrawalTransition({
      inputs: [input],
      output,
      outputScript: script,
      pooling: 'never',
      coreFeePerByte: 1,
    });
  }

  describe('constructor()', () => {
    it('should create transition', () => {
      const transition = createTransition();
      expect(transition).to.exist();
      expect(transition).to.be.an.instanceof(wasm.AddressCreditWithdrawalTransition);
    });

    it('should create transition without output', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const script = wasm.CoreScript.fromP2PKH([
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
      ]);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));

      const transition = new wasm.AddressCreditWithdrawalTransition({
        inputs: [input],
        outputScript: script,
        pooling: 'never',
        coreFeePerByte: 1,
      });

      expect(transition).to.exist();
    });

    it('should create transition with user fee increase', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const script = wasm.CoreScript.fromP2PKH([
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
      ]);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));

      const transition = new wasm.AddressCreditWithdrawalTransition({
        inputs: [input],
        outputScript: script,
        pooling: 'never',
        coreFeePerByte: 1,
        userFeeIncrease: 100,
      });

      expect(transition.userFeeIncrease).to.equal(100);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes', () => {
      const transition = createTransition();
      const bytes = transition.toBytes();
      const restored = wasm.AddressCreditWithdrawalTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toBase64() / fromBase64()', () => {
    it('should round-trip via base64', () => {
      const transition = createTransition();
      const base64 = transition.toBase64();
      const bytes = transition.toBytes();
      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.AddressCreditWithdrawalTransition.fromBase64(base64);
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
    });
  });

  describe('output', () => {
    it('should return output', () => {
      const transition = createTransition();
      const { output } = transition;
      expect(output).to.exist();
      expect(output.amount).to.equal(BigInt(90000));
    });

    it('should set output to undefined', () => {
      const transition = createTransition();
      transition.output = undefined;
      expect(transition.output).to.be.undefined();
    });
  });

  describe('outputScript', () => {
    it('should return outputScript', () => {
      const transition = createTransition();
      expect(transition.outputScript).to.exist();
      expect(transition.outputScript).to.be.an.instanceof(wasm.CoreScript);
    });

    it('should set outputScript', () => {
      const transition = createTransition();
      const script2 = wasm.CoreScript.fromP2PKH([
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
      ]);

      const origScript = transition.outputScript.toString();
      transition.outputScript = script2;
      expect(transition.outputScript.toString()).to.not.equal(origScript);
      expect(transition.outputScript.toString()).to.equal(script2.toString());
    });
  });

  describe('pooling', () => {
    it('should return pooling', () => {
      const transition = createTransition();
      expect(transition.pooling).to.equal('Never');
    });

    it('should set pooling', () => {
      const transition = createTransition();
      transition.pooling = 'Standard';
      expect(transition.pooling).to.equal('Standard');
    });
  });

  describe('coreFeePerByte', () => {
    it('should return coreFeePerByte', () => {
      const transition = createTransition();
      expect(transition.coreFeePerByte).to.equal(1);
    });

    it('should set coreFeePerByte', () => {
      const transition = createTransition();
      transition.coreFeePerByte = 10;
      expect(transition.coreFeePerByte).to.equal(10);
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

  describe('toObject() / toJSON() / fromObject() / fromJSON()', () => {
    it('toObject() emits inputs as typed array of {address, nonce, amount}', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.inputs).to.be.an('array').with.lengthOf(1);
      expect(obj.inputs[0].address).to.be.instanceOf(Uint8Array);
      expect(obj.inputs[0].address.length).to.equal(21);
      expect(obj.inputs[0].nonce).to.equal(0);
      expect(obj.inputs[0].amount).to.equal(BigInt(100000));
    });

    it('toObject() emits output as typed singular {address, amount}', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.output).to.be.an('object');
      expect(obj.output.address).to.be.instanceOf(Uint8Array);
      expect(obj.output.address.length).to.equal(21);
      expect(obj.output.amount).to.equal(BigInt(90000));
    });

    it('toObject() omits output when not provided', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const script = wasm.CoreScript.fromP2PKH([
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
      ]);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const transition = new wasm.AddressCreditWithdrawalTransition({
        inputs: [input],
        outputScript: script,
        pooling: 'never',
        coreFeePerByte: 1,
      });

      const obj = transition.toObject();
      expect(obj.output == null).to.be.true(); // null OR undefined
    });

    it('toObject() emits feeStrategy with {type, index} shape', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.feeStrategy).to.be.an('array');
      expect(obj.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
      expect(obj.feeStrategy[0].index).to.be.a('number');
    });

    it('toJSON() emits hex addresses, string outputScript, and pooling name', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json.inputs[0].address).to.be.a('string').with.lengthOf(42);
      expect(json.output).to.be.an('object');
      expect(json.output.address).to.be.a('string').with.lengthOf(42);
      expect(json.output.amount).to.satisfy((v: unknown) => typeof v === 'number' || typeof v === 'string');
      expect(json.outputScript).to.be.a('string');
      expect(json.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
    });

    it('fromObject(toObject()) round-trips identically', () => {
      const transition = createTransition();
      const obj = transition.toObject();
      const restored = wasm.AddressCreditWithdrawalTransition.fromObject(obj);
      expect(restored.toObject()).to.deep.equal(obj);
    });

    it('fromJSON(toJSON()) round-trips identically', () => {
      const transition = createTransition();
      const json = transition.toJSON();
      const restored = wasm.AddressCreditWithdrawalTransition.fromJSON(json);
      expect(restored.toJSON()).to.deep.equal(json);
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from StateTransition', () => {
      const transition = createTransition();
      const st = transition.toStateTransition();
      expect(st).to.exist();

      const restored =
        wasm.AddressCreditWithdrawalTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(
        Buffer.from(transition.toBytes()),
      );
    });
  });
});
