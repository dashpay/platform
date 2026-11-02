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

describe('IdentityTopUpFromShieldedPoolTransition', () => {
  const identityId = '11111111111111111111111111111111';

  function createTransition() {
    return new wasm.IdentityTopUpFromShieldedPoolTransition({
      identityId,
      actions: [fakeOrchardAction()],
      topUpAmount: BigInt(50_000),
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
    });
  }

  describe('constructor()', () => {
    it('should construct with required fields', () => {
      expect(createTransition()).to.be.an.instanceof(wasm.IdentityTopUpFromShieldedPoolTransition);
    });

    it('should reject anchor of wrong length', () => {
      expect(() => new wasm.IdentityTopUpFromShieldedPoolTransition({
        identityId,
        actions: [fakeOrchardAction()],
        topUpAmount: BigInt(1),
        anchor: new Uint8Array(31),
        proof: ZERO_PROOF,
        bindingSignature: ZERO_BINDING_SIG,
      })).to.throw();
    });
  });

  describe('getters', () => {
    it('returns identityId, actions and topUpAmount', () => {
      const t = createTransition();
      expect(t.identityId).to.be.an.instanceof(wasm.Identifier);
      expect(t.identityId.toString()).to.equal(identityId);
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
      expect(t.topUpAmount).to.equal(BigInt(50_000));
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes, base64 and hex', () => {
      const T = wasm.IdentityTopUpFromShieldedPoolTransition;
      const t = createTransition();
      const bytes = t.toBytes();
      expect(Buffer.from(T.fromBytes(bytes).toBytes())).to.deep.equal(Buffer.from(bytes));
      expect(Buffer.from(T.fromBase64(t.toBase64()).toBytes())).to.deep.equal(Buffer.from(bytes));
      expect(Buffer.from(T.fromHex(t.toHex()).toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toStateTransition()', () => {
    it('converts to the umbrella wrapper with type 22 and back', () => {
      const t = createTransition();
      const st = t.toStateTransition();
      expect(st.actionTypeNumber).to.equal(22);
      const restored = wasm.IdentityTopUpFromShieldedPoolTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
    });
  });
});
