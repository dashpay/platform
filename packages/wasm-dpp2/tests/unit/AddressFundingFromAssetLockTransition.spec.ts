import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { instantLockBytes, transactionBytes } from './mocks/Locks/index.js';

before(async () => {
  await initWasm();
});

describe('AddressFundingFromAssetLockTransition', () => {
  const addr1Bytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ]);
  const addr2Bytes = new Uint8Array([
    0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
  ]);

  function createAssetLockProof() {
    return wasm.AssetLockProof.createInstantAssetLockProof(
      instantLockBytes,
      transactionBytes,
      0,
    );
  }

  function createTransition() {
    const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
    const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

    const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
    // For asset lock funding, output amount can be optional (undefined = remainder)
    const output = new wasm.PlatformAddressOutput(outputAddr);

    return new wasm.AddressFundingFromAssetLockTransition({
      assetLockProof: createAssetLockProof(),
      inputs: [input],
      outputs: [output],
    });
  }

  describe('constructor()', () => {
    it('should create transition with asset lock proof', () => {
      const transition = createTransition();
      expect(transition).to.exist();
      expect(transition).to.be.an.instanceof(wasm.AddressFundingFromAssetLockTransition);
    });

    it('should create transition with explicit output amounts', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(50000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(40000));

      const transition = new wasm.AddressFundingFromAssetLockTransition({
        assetLockProof: createAssetLockProof(),
        inputs: [input],
        outputs: [output],
      });

      expect(transition).to.exist();
    });

    it('should create transition with user fee increase', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const output = new wasm.PlatformAddressOutput(outputAddr);

      const transition = new wasm.AddressFundingFromAssetLockTransition({
        assetLockProof: createAssetLockProof(),
        inputs: [input],
        outputs: [output],
        userFeeIncrease: 100,
      });

      expect(transition.userFeeIncrease).to.equal(100);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes', () => {
      const transition = createTransition();
      const bytes = transition.toBytes();
      const restored = wasm.AddressFundingFromAssetLockTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toBase64() / fromBase64()', () => {
    it('should round-trip via base64', () => {
      const transition = createTransition();
      const base64 = transition.toBase64();
      const bytes = transition.toBytes();
      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.AddressFundingFromAssetLockTransition.fromBase64(base64);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toHex() / fromHex()', () => {
    it('should round-trip via hex', () => {
      const transition = createTransition();
      const hex = transition.toHex();
      const bytes = transition.toBytes();

      const restored = wasm.AddressFundingFromAssetLockTransition.fromHex(hex);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('assetLockProof', () => {
    it('should return asset lock proof', () => {
      const transition = createTransition();
      const proof = transition.assetLockProof;
      expect(proof).to.exist();
      expect(proof.lockType).to.equal('instant');
    });

    it('should set asset lock proof', () => {
      const transition = createTransition();
      const outpoint = new wasm.OutPoint(
        'e8b43025641eea4fd21190f01bd870ef90f1a8b199d8fc3376c5b62c0b1a179d',
        1,
      );
      const chainProof = wasm.AssetLockProof.createChainAssetLockProof(11, outpoint);

      transition.assetLockProof = chainProof;
      expect(transition.assetLockProof.lockType).to.equal('chain');
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
      const newInput = new wasm.PlatformAddressInput(newAddr, 3, BigInt(50000));

      transition.inputs = [newInput];
      const { inputs } = transition;
      expect(inputs).to.have.lengthOf(1);
      expect(inputs[0].nonce).to.equal(3);
    });
  });

  describe('outputs', () => {
    it('should return outputs array', () => {
      const transition = createTransition();
      const { outputs } = transition;
      expect(outputs).to.be.an('array');
      expect(outputs).to.have.lengthOf(1);
    });

    it('should set outputs', () => {
      const transition = createTransition();
      const newAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const newOutput = new wasm.PlatformAddressOutput(newAddr, BigInt(50000));

      transition.outputs = [newOutput];
      const { outputs } = transition;
      expect(outputs).to.have.lengthOf(1);
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

      expect(obj.inputs).to.be.an('array');
      expect(obj.inputs).to.have.lengthOf(1);
      expect(obj.inputs[0].address).to.be.instanceOf(Uint8Array);
      expect(obj.inputs[0].address.length).to.equal(21);
      expect(obj.inputs[0].nonce).to.equal(0);
      expect(obj.inputs[0].amount).to.equal(BigInt(100000));
    });

    it('toObject() emits outputs with absent amount for unspecified', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.outputs).to.be.an('array');
      expect(obj.outputs).to.have.lengthOf(1);
      expect(obj.outputs[0].address).to.be.instanceOf(Uint8Array);
      expect(obj.outputs[0].address.length).to.equal(21);
      // serde Option::None becomes undefined in the wasm Object form (JSON form is null).
      expect(obj.outputs[0].amount == null).to.be.true(); // null OR undefined
    });

    it('toObject() emits outputs with explicit bigint amount when set', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(50000));
      const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(40000));
      const transition = new wasm.AddressFundingFromAssetLockTransition({
        assetLockProof: createAssetLockProof(),
        inputs: [input],
        outputs: [output],
      });

      const obj = transition.toObject();
      expect(obj.outputs[0].amount).to.equal(BigInt(40000));
    });

    it('toObject() emits feeStrategy as typed array of {type, index}', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.feeStrategy).to.be.an('array');
      expect(obj.feeStrategy.length).to.be.greaterThan(0);
      expect(obj.feeStrategy[0]).to.have.property('$type');
      expect(obj.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
      expect(obj.feeStrategy[0].index).to.be.a('number');
    });

    it('toJSON() emits inputs as typed array with hex address and number/string amount', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json.inputs).to.be.an('array');
      expect(json.inputs[0].address).to.be.a('string');
      expect(json.inputs[0].address).to.have.lengthOf(42); // 21 bytes hex-encoded
      expect(json.inputs[0].nonce).to.equal(0);
      expect(json.inputs[0].amount).to.satisfy((v: unknown) => typeof v === 'number' || typeof v === 'string');
    });

    it('toJSON() emits outputs with hex address and null amount when unset', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json.outputs).to.be.an('array');
      expect(json.outputs[0].address).to.be.a('string');
      expect(json.outputs[0].address).to.have.lengthOf(42);
      expect(json.outputs[0].amount).to.be.null();
    });

    it('toJSON() emits feeStrategy with {type, index} shape', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json.feeStrategy).to.be.an('array');
      expect(json.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
      expect(json.feeStrategy[0].index).to.be.a('number');
    });

    it('fromObject(toObject()) round-trips identically', () => {
      const transition = createTransition();
      const obj = transition.toObject();
      const restored = wasm.AddressFundingFromAssetLockTransition.fromObject(obj);
      expect(restored.toObject()).to.deep.equal(obj);
    });

    it('fromJSON(toJSON()) round-trips identically', () => {
      const transition = createTransition();
      const json = transition.toJSON();
      const restored = wasm.AddressFundingFromAssetLockTransition.fromJSON(json);
      expect(restored.toJSON()).to.deep.equal(json);
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from StateTransition', () => {
      const transition = createTransition();
      const st = transition.toStateTransition();
      expect(st).to.exist();

      const restored =
        wasm.AddressFundingFromAssetLockTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(
        Buffer.from(transition.toBytes()),
      );
    });
  });
});
