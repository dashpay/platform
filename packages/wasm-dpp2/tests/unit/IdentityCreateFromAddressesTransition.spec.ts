import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreateFromAddressesTransition', () => {
  const addr1Bytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ]);
  const addr2Bytes = new Uint8Array([
    0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
  ]);

  function createPublicKey() {
    return new wasm.IdentityPublicKeyInCreation({
      keyId: 0,
      purpose: 'AUTHENTICATION',
      securityLevel: 'master',
      keyType: 'ECDSA_SECP256K1',
      isReadOnly: false,
      data: Buffer.from(
        '0333d5cf3674001d2f64c55617b7b11a2e8fc62aab09708b49355e30c7205bdb2e',
        'hex',
      ),
      signature: [],
    });
  }

  function createTransition() {
    const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
    const outputAddr = wasm.PlatformAddress.fromBytes(addr2Bytes);

    const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
    const output = new wasm.PlatformAddressOutput(outputAddr, BigInt(90000));
    const pk = createPublicKey();

    return new wasm.IdentityCreateFromAddressesTransition({
      publicKeys: [pk],
      inputs: [input],
      output,
    });
  }

  describe('constructor()', () => {
    it('should create transition with public keys', () => {
      const transition = createTransition();
      expect(transition).to.exist();
      expect(transition).to.be.an.instanceof(wasm.IdentityCreateFromAddressesTransition);
    });

    it('should create transition without output', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const pk = createPublicKey();

      const transition = new wasm.IdentityCreateFromAddressesTransition({
        publicKeys: [pk],
        inputs: [input],
      });

      expect(transition).to.exist();
    });

    it('should create transition with user fee increase', () => {
      const inputAddr = wasm.PlatformAddress.fromBytes(addr1Bytes);
      const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100000));
      const pk = createPublicKey();

      const transition = new wasm.IdentityCreateFromAddressesTransition({
        publicKeys: [pk],
        inputs: [input],
        userFeeIncrease: 100,
      });

      expect(transition.userFeeIncrease).to.equal(100);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes', () => {
      const transition = createTransition();
      const bytes = transition.toBytes();
      const restored = wasm.IdentityCreateFromAddressesTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toBase64() / fromBase64()', () => {
    it('should round-trip via base64', () => {
      const transition = createTransition();
      const base64 = transition.toBase64();
      const bytes = transition.toBytes();
      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));

      const restored = wasm.IdentityCreateFromAddressesTransition.fromBase64(base64);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('publicKeys', () => {
    it('should return public keys', () => {
      const transition = createTransition();
      const keys = transition.publicKeys;
      expect(keys).to.be.an('array');
      expect(keys).to.have.lengthOf(1);
      expect(keys[0].keyId).to.equal(0);
      expect(keys[0].purpose).to.equal('AUTHENTICATION');
    });

    it('should set public keys', () => {
      const transition = createTransition();
      const pk = createPublicKey();
      pk.keyId = 5;

      transition.publicKeys = [pk];
      const keys = transition.publicKeys;
      expect(keys).to.have.lengthOf(1);
      expect(keys[0].keyId).to.equal(5);
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

    it('toObject() emits singular output as {address, amount}', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.output).to.be.an('object');
      expect(obj.output.address).to.be.instanceOf(Uint8Array);
      expect(obj.output.address.length).to.equal(21);
      expect(obj.output.amount).to.equal(BigInt(90000));
    });

    it('toObject() emits feeStrategy with {type, index} shape', () => {
      const transition = createTransition();
      const obj = transition.toObject();

      expect(obj.feeStrategy).to.be.an('array');
      expect(obj.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
      expect(obj.feeStrategy[0].index).to.be.a('number');
    });

    it('toJSON() emits hex addresses and number/string amounts', () => {
      const transition = createTransition();
      const json = transition.toJSON();

      expect(json.inputs[0].address).to.be.a('string').with.lengthOf(42);
      expect(json.inputs[0].nonce).to.equal(0);
      expect(json.output).to.be.an('object');
      expect(json.output.address).to.be.a('string').with.lengthOf(42);
      expect(json.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
    });

    it('fromObject(toObject()) round-trips identically', () => {
      const transition = createTransition();
      const obj = transition.toObject();
      const restored = wasm.IdentityCreateFromAddressesTransition.fromObject(obj);
      expect(restored.toObject()).to.deep.equal(obj);
    });

    it('fromJSON(toJSON()) round-trips identically', () => {
      const transition = createTransition();
      const json = transition.toJSON();
      const restored = wasm.IdentityCreateFromAddressesTransition.fromJSON(json);
      expect(restored.toJSON()).to.deep.equal(json);
    });
  });

  describe('toStateTransition() / fromStateTransition()', () => {
    it('should convert to and from StateTransition', () => {
      const transition = createTransition();
      const st = transition.toStateTransition();
      expect(st).to.exist();

      const restored =
        wasm.IdentityCreateFromAddressesTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(
        Buffer.from(transition.toBytes()),
      );
    });
  });
});
