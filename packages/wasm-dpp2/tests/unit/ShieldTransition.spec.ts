import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import {
  fakeOrchardAction,
  ZERO_ANCHOR,
  ZERO_BINDING_SIG,
  ZERO_PROOF,
} from './helpers/shielded.ts';

before(async () => {
  await initWasm();
});

describe('ShieldTransition', () => {
  const addrBytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ]);

  function createTransition() {
    const inputAddr = wasm.PlatformAddress.fromBytes(addrBytes);
    const input = new wasm.PlatformAddressInput(inputAddr, 0, BigInt(100_000));
    const witness = wasm.AddressWitness.p2pkh(new Uint8Array(65));

    return new wasm.ShieldTransition({
      inputs: [input],
      actions: [fakeOrchardAction()],
      amount: BigInt(50_000),
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
      inputWitnesses: [witness],
    });
  }

  describe('constructor()', () => {
    it('should construct with required fields', () => {
      const t = createTransition();
      expect(t).to.be.an.instanceof(wasm.ShieldTransition);
    });

    it('should reject anchor of wrong length', () => {
      expect(() => new wasm.ShieldTransition({
          inputs: [],
          actions: [fakeOrchardAction()],
          amount: BigInt(0),
          anchor: new Uint8Array(31),
          proof: ZERO_PROOF,
          bindingSignature: ZERO_BINDING_SIG,
          inputWitnesses: [],
        })).to.throw();
    });
  });

  describe('getters', () => {
    it('returns typed inputs / actions / inputWitnesses / feeStrategy', () => {
      const t = createTransition();
      expect(t.inputs).to.be.an('array').with.lengthOf(1);
      expect(t.inputs[0]).to.be.an.instanceof(wasm.PlatformAddressInput);
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
      expect(t.inputWitnesses[0]).to.be.an.instanceof(wasm.AddressWitness);
      expect(t.feeStrategy).to.be.an('array');
      expect(t.amount).to.equal(BigInt(50_000));
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const restored = wasm.ShieldTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject() / toJSON()', () => {
    it('toObject() emits inputs as typed array of {address, nonce, amount}', () => {
      const t = createTransition();
      const obj = t.toObject();

      expect(obj.inputs).to.be.an('array').with.lengthOf(1);
      expect(obj.inputs[0].address).to.be.instanceOf(Uint8Array);
      expect(obj.inputs[0].address.length).to.equal(21);
      expect(obj.inputs[0].nonce).to.equal(0);
      expect(obj.inputs[0].amount).to.equal(BigInt(100_000));
    });

    it('toObject() emits feeStrategy with {type, index} shape', () => {
      const t = createTransition();
      const obj = t.toObject();
      expect(obj.feeStrategy).to.be.an('array');
      if (obj.feeStrategy.length > 0) {
        expect(obj.feeStrategy[0].$type).to.be.oneOf(['deductFromInput', 'reduceOutput']);
        expect(obj.feeStrategy[0].index).to.be.a('number');
      }
    });

    it('toJSON() emits inputs with hex address and number/string amount', () => {
      const t = createTransition();
      const json = t.toJSON();

      expect(json.inputs[0].address).to.be.a('string').with.lengthOf(42);
      expect(json.inputs[0].nonce).to.equal(0);
      expect(json.inputs[0].amount).to.satisfy((v: unknown) => typeof v === 'number' || typeof v === 'string');
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to StateTransition wrapper', () => {
      const t = createTransition();
      expect(t.toStateTransition()).to.exist();
    });
  });
});
