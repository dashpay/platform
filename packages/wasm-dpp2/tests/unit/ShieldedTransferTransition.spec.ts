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

describe('ShieldedTransferTransition', () => {
  function createTransition() {
    return new wasm.ShieldedTransferTransition({
      actions: [fakeOrchardAction()],
      valueBalance: BigInt(0),
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
    });
  }

  describe('constructor()', () => {
    it('should construct with valid Orchard fields', () => {
      const t = createTransition();
      expect(t).to.be.an.instanceof(wasm.ShieldedTransferTransition);
    });

    it('should reject anchor of wrong length', () => {
      expect(() => new wasm.ShieldedTransferTransition({
          actions: [fakeOrchardAction()],
          valueBalance: BigInt(0),
          anchor: new Uint8Array(31), // too short
          proof: ZERO_PROOF,
          bindingSignature: ZERO_BINDING_SIG,
        })).to.throw();
    });

    it('should reject bindingSignature of wrong length', () => {
      expect(() => new wasm.ShieldedTransferTransition({
          actions: [fakeOrchardAction()],
          valueBalance: BigInt(0),
          anchor: ZERO_ANCHOR,
          proof: ZERO_PROOF,
          bindingSignature: new Uint8Array(63),
        })).to.throw();
    });
  });

  describe('getters', () => {
    it('should expose actions, anchor, proof, bindingSignature, valueBalance', () => {
      const t = createTransition();
      expect(t.actions).to.be.an('array').with.lengthOf(1);
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
      expect(t.anchor).to.be.instanceOf(Uint8Array);
      expect(t.anchor.length).to.equal(32);
      expect(t.bindingSignature.length).to.equal(64);
      expect(t.valueBalance).to.equal(BigInt(0));
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const restored = wasm.ShieldedTransferTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject() / toJSON()', () => {
    it('toObject() emits typed actions array with Uint8Array byte fields', () => {
      const t = createTransition();
      const obj = t.toObject();

      expect(obj.actions).to.be.an('array').with.lengthOf(1);
      const a = obj.actions[0];
      expect(a.nullifier).to.be.instanceOf(Uint8Array);
      expect(a.nullifier.length).to.equal(32);
      expect(a.rk).to.be.instanceOf(Uint8Array);
      expect(a.rk.length).to.equal(32);
      expect(a.cmx).to.be.instanceOf(Uint8Array);
      expect(a.cmx.length).to.equal(32);
      expect(a.encryptedNote).to.be.instanceOf(Uint8Array);
      expect(a.cvNet).to.be.instanceOf(Uint8Array);
      expect(a.cvNet.length).to.equal(32);
      expect(a.spendAuthSig).to.be.instanceOf(Uint8Array);
      expect(a.spendAuthSig.length).to.equal(64);
    });

    it('toJSON() emits action byte fields as base64 strings', () => {
      const t = createTransition();
      const json = t.toJSON();

      const a = json.actions[0];
      expect(a.nullifier).to.be.a('string').with.lengthOf(44); // 32 bytes base64
      expect(a.rk).to.be.a('string').with.lengthOf(44);
      expect(a.cmx).to.be.a('string').with.lengthOf(44);
      expect(a.encryptedNote).to.be.a('string');
      expect(a.cvNet).to.be.a('string').with.lengthOf(44);
      expect(a.spendAuthSig).to.be.a('string').with.lengthOf(88); // 64 bytes base64
    });

    it('toObject() emits anchor / proof / bindingSignature as Uint8Array', () => {
      const t = createTransition();
      const obj = t.toObject();

      expect(obj.anchor).to.be.instanceOf(Uint8Array);
      expect(obj.anchor.length).to.equal(32);
      expect(obj.proof).to.be.instanceOf(Uint8Array);
      expect(obj.bindingSignature).to.be.instanceOf(Uint8Array);
      expect(obj.bindingSignature.length).to.equal(64);
    });

    it('toJSON() emits anchor / proof / bindingSignature as base64 strings', () => {
      const t = createTransition();
      const json = t.toJSON();

      expect(json.anchor).to.be.a('string').with.lengthOf(44); // 32 bytes base64
      expect(json.proof).to.be.a('string');
      expect(json.bindingSignature).to.be.a('string').with.lengthOf(88); // 64 bytes base64
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to StateTransition wrapper', () => {
      const t = createTransition();
      const st = t.toStateTransition();
      expect(st).to.exist();
    });
  });
});
