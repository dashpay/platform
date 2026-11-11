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

describe('PrivateDocumentCreateFromShieldedPoolTransition', () => {
  const dataContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
  // Above Number.MAX_SAFE_INTEGER, so JSON has to carry it as a string.
  const valueBalance = BigInt('9007199254740993');
  const firstEntry = new Uint8Array(24).fill(1);
  const secondEntry = new Uint8Array(24).fill(2);

  function createTransition() {
    return new wasm.PrivateDocumentCreateFromShieldedPoolTransition({
      dataContractId,
      documentTypeName: 'privateContactRequest',
      entries: [firstEntry, secondEntry],
      actions: [fakeOrchardAction()],
      valueBalance,
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
    });
  }

  describe('constructor()', () => {
    it('should construct with required fields', () => {
      expect(createTransition()).to.be.an.instanceof(
        wasm.PrivateDocumentCreateFromShieldedPoolTransition,
      );
    });

    it('should reject an anchor of the wrong length', () => {
      expect(() => new wasm.PrivateDocumentCreateFromShieldedPoolTransition({
        dataContractId,
        documentTypeName: 'privateContactRequest',
        entries: [firstEntry],
        actions: [fakeOrchardAction()],
        valueBalance,
        anchor: new Uint8Array(31),
        proof: ZERO_PROOF,
        bindingSignature: ZERO_BINDING_SIG,
      })).to.throw();
    });
  });

  describe('getters', () => {
    it('should return the target, the entries in order and the value balance', () => {
      const t = createTransition();
      expect(t.dataContractId).to.be.an.instanceof(wasm.Identifier);
      expect(t.dataContractId.toString()).to.equal(dataContractId);
      expect(t.documentTypeName).to.equal('privateContactRequest');
      expect(t.entries).to.have.lengthOf(2);
      expect(Buffer.from(t.entries[0])).to.deep.equal(Buffer.from(firstEntry));
      expect(Buffer.from(t.entries[1])).to.deep.equal(Buffer.from(secondEntry));
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
      expect(t.valueBalance).to.equal(valueBalance);
    });

    it('should copy the entries in and out', () => {
      const entry = new Uint8Array(24).fill(7);
      const t = new wasm.PrivateDocumentCreateFromShieldedPoolTransition({
        dataContractId,
        documentTypeName: 'privateContactRequest',
        entries: [entry],
        actions: [fakeOrchardAction()],
        valueBalance,
        anchor: ZERO_ANCHOR,
        proof: ZERO_PROOF,
        bindingSignature: ZERO_BINDING_SIG,
      });
      entry.fill(0);
      t.entries[0].fill(0);
      expect(Buffer.from(t.entries[0])).to.deep.equal(Buffer.from(new Uint8Array(24).fill(7)));
    });
  });

  describe('toObject() / fromObject()', () => {
    it('should emit every property with bytes as Uint8Array and round-trip', () => {
      const t = createTransition();
      const obj = t.toObject();
      expect(obj.$formatVersion).to.equal('0');
      expect(obj.dataContractId).to.be.instanceOf(Uint8Array);
      expect(obj.dataContractId.length).to.equal(32);
      expect(obj.documentTypeName).to.equal('privateContactRequest');
      expect(obj.entries).to.be.an('array').with.lengthOf(2);
      expect(obj.entries[0]).to.be.instanceOf(Uint8Array);
      expect(Buffer.from(obj.entries[1])).to.deep.equal(Buffer.from(secondEntry));
      expect(obj.actions).to.be.an('array').with.lengthOf(1);
      expect(obj.valueBalance).to.equal(valueBalance);
      expect(obj.anchor).to.be.instanceOf(Uint8Array);
      expect(obj.proof).to.be.instanceOf(Uint8Array);
      expect(obj.bindingSignature).to.be.instanceOf(Uint8Array);

      const restored = wasm.PrivateDocumentCreateFromShieldedPoolTransition.fromObject(obj);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
    });
  });

  describe('toJSON() / fromJSON()', () => {
    it('should emit every property with bytes as base64 and round-trip', () => {
      const t = createTransition();
      const json = t.toJSON();
      expect(json.$formatVersion).to.equal('0');
      expect(json.dataContractId).to.equal(dataContractId);
      expect(json.documentTypeName).to.equal('privateContactRequest');
      expect(json.entries).to.deep.equal([
        Buffer.from(firstEntry).toString('base64'),
        Buffer.from(secondEntry).toString('base64'),
      ]);
      expect(json.actions).to.be.an('array').with.lengthOf(1);
      expect(json.valueBalance).to.equal(valueBalance.toString());
      expect(json.anchor).to.be.a('string').with.lengthOf(44);
      expect(json.bindingSignature).to.be.a('string').with.lengthOf(88);

      const restored = wasm.PrivateDocumentCreateFromShieldedPoolTransition.fromJSON(json);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('should round-trip via bytes, base64 and hex', () => {
      const T = wasm.PrivateDocumentCreateFromShieldedPoolTransition;
      const t = createTransition();
      const bytes = t.toBytes();
      expect(Buffer.from(T.fromBytes(bytes).toBytes())).to.deep.equal(Buffer.from(bytes));
      expect(Buffer.from(T.fromBase64(t.toBase64()).toBytes())).to.deep.equal(Buffer.from(bytes));
      expect(Buffer.from(T.fromHex(t.toHex()).toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to the umbrella wrapper with type 29 and back, keeping the bytes', () => {
      const t = createTransition();
      const st = t.toStateTransition();
      expect(st.actionTypeNumber).to.equal(29);
      expect(Buffer.from(st.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
      const restored = wasm.PrivateDocumentCreateFromShieldedPoolTransition.fromStateTransition(st);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
    });
  });
});
