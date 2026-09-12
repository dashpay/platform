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

describe('ShieldFromIdentityTransition', () => {
  const identityId = '11111111111111111111111111111111';

  function createTransition() {
    return new wasm.ShieldFromIdentityTransition({
      identityId,
      amount: BigInt(50_000),
      actions: [fakeOrchardAction()],
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
      nonce: BigInt(1),
    });
  }

  describe('constructor()', () => {
    it('should construct with required fields', () => {
      const t = createTransition();
      expect(t).to.be.an.instanceof(wasm.ShieldFromIdentityTransition);
    });

    it('should accept an Identifier instance and a user fee increase', () => {
      const t = new wasm.ShieldFromIdentityTransition({
        identityId: new wasm.Identifier(identityId),
        amount: BigInt(1),
        actions: [fakeOrchardAction()],
        anchor: ZERO_ANCHOR,
        proof: ZERO_PROOF,
        bindingSignature: ZERO_BINDING_SIG,
        nonce: BigInt(7),
        userFeeIncrease: 50,
      });
      expect(t.userFeeIncrease).to.equal(50);
      expect(t.nonce).to.equal(BigInt(7));
    });

    it('should reject anchor of wrong length', () => {
      expect(() => new wasm.ShieldFromIdentityTransition({
        identityId,
        amount: BigInt(1),
        actions: [fakeOrchardAction()],
        anchor: new Uint8Array(31),
        proof: ZERO_PROOF,
        bindingSignature: ZERO_BINDING_SIG,
        nonce: BigInt(1),
      })).to.throw();
    });
  });

  describe('getters / setters', () => {
    it('returns identityId, amount, actions, nonce and identity-signature fields', () => {
      const t = createTransition();
      expect(t.identityId).to.be.an.instanceof(wasm.Identifier);
      expect(t.identityId.toString()).to.equal(identityId);
      expect(t.amount).to.equal(BigInt(50_000));
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
      expect(t.nonce).to.equal(BigInt(1));
      expect(t.signaturePublicKeyId).to.equal(0);
      expect(t.signature).to.be.an.instanceof(Uint8Array);
    });

    it('supports setting signature, signaturePublicKeyId, nonce and userFeeIncrease', () => {
      const t = createTransition();
      t.signature = new Uint8Array(65).fill(7);
      t.signaturePublicKeyId = 2;
      t.nonce = BigInt(9);
      t.userFeeIncrease = 3;
      expect(t.signature.length).to.equal(65);
      expect(t.signaturePublicKeyId).to.equal(2);
      expect(t.nonce).to.equal(BigInt(9));
      expect(t.userFeeIncrease).to.equal(3);
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const restored = wasm.ShieldFromIdentityTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });

    it('round-trips via base64 and hex', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const fromBase64 = wasm.ShieldFromIdentityTransition.fromBase64(t.toBase64());
      const fromHex = wasm.ShieldFromIdentityTransition.fromHex(t.toHex());
      expect(Buffer.from(fromBase64.toBytes())).to.deep.equal(Buffer.from(bytes));
      expect(Buffer.from(fromHex.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toStateTransition()', () => {
    it('converts to the umbrella wrapper with type 21 and back', () => {
      const t = createTransition();
      const st = t.toStateTransition();
      expect(st).to.exist();
      expect(st.actionTypeNumber).to.equal(21);
      const restored = wasm.ShieldFromIdentityTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
    });
  });
});
