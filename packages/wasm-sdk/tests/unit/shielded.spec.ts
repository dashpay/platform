import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

// `cmx`, `nullifier`, `cvNet`, `encryptedNote` use `bytes_b64` which emits raw bytes
// in non-human-readable mode (Object) and base64 strings in human-readable
// mode (JSON). These tests pin both representations so the round-trip stays
// correct if the helper is ever refactored.

describe('ShieldedEncryptedNote', () => {
  before(async () => {
    await init();
  });

  const cmxBytes = new Uint8Array(32).fill(0xa1);
  const nullifierBytes = new Uint8Array(32).fill(0xb2);
  const cvNetBytes = new Uint8Array(32).fill(0xd4);
  const encryptedNoteBytes = new Uint8Array(216).fill(0xc3);

  // Same bytes encoded as base64 for the JSON form.
  const objectFixture = {
    cmx: cmxBytes,
    nullifier: nullifierBytes,
    cvNet: cvNetBytes,
    encryptedNote: encryptedNoteBytes,
  };

  describe('fromObject() and getters', () => {
    it('should expose the four byte fields via getters', () => {
      const note = sdk.ShieldedEncryptedNote.fromObject(objectFixture);

      expect(note.cmx).to.be.instanceOf(Uint8Array);
      expect(note.cmx).to.deep.equal(cmxBytes);
      expect(note.nullifier).to.deep.equal(nullifierBytes);
      expect(note.cvNet).to.deep.equal(cvNetBytes);
      expect(note.encryptedNote).to.deep.equal(encryptedNoteBytes);
    });
  });

  describe('toObject()', () => {
    it('should round-trip the bytes through fromObject/toObject', () => {
      const note = sdk.ShieldedEncryptedNote.fromObject(objectFixture);
      const obj = note.toObject();

      expect(obj.cmx).to.deep.equal(cmxBytes);
      expect(obj.nullifier).to.deep.equal(nullifierBytes);
      expect(obj.cvNet).to.deep.equal(cvNetBytes);
      expect(obj.encryptedNote).to.deep.equal(encryptedNoteBytes);
    });
  });

  describe('toJSON() / fromJSON()', () => {
    it('should encode the byte fields as base64 strings', () => {
      const note = sdk.ShieldedEncryptedNote.fromObject(objectFixture);
      const json = note.toJSON();

      expect(json.cmx).to.be.a('string');
      expect(json.nullifier).to.be.a('string');
      expect(json.cvNet).to.be.a('string');
      expect(json.encryptedNote).to.be.a('string');
    });

    it('should round-trip via JSON', () => {
      const note = sdk.ShieldedEncryptedNote.fromObject(objectFixture);
      const json = note.toJSON();
      const restored = sdk.ShieldedEncryptedNote.fromJSON(json);

      expect(restored.cmx).to.deep.equal(cmxBytes);
      expect(restored.nullifier).to.deep.equal(nullifierBytes);
      expect(restored.cvNet).to.deep.equal(cvNetBytes);
      expect(restored.encryptedNote).to.deep.equal(encryptedNoteBytes);
    });
  });
});

describe('ShieldedNullifierStatus', () => {
  before(async () => {
    await init();
  });

  const nullifierBytes = new Uint8Array(32).fill(0xde);

  describe('fromObject() and getters', () => {
    it('should expose nullifier bytes and isSpent flag', () => {
      const status = sdk.ShieldedNullifierStatus.fromObject({
        nullifier: nullifierBytes,
        isSpent: true,
      });

      expect(status.nullifier).to.be.instanceOf(Uint8Array);
      expect(status.nullifier).to.deep.equal(nullifierBytes);
      expect(status.isSpent).to.equal(true);
    });

    it('should accept isSpent: false', () => {
      const status = sdk.ShieldedNullifierStatus.fromObject({
        nullifier: nullifierBytes,
        isSpent: false,
      });
      expect(status.isSpent).to.equal(false);
    });
  });

  describe('toObject()', () => {
    it('should round-trip the nullifier and flag', () => {
      const status = sdk.ShieldedNullifierStatus.fromObject({
        nullifier: nullifierBytes,
        isSpent: true,
      });
      const obj = status.toObject();

      expect(obj.nullifier).to.deep.equal(nullifierBytes);
      expect(obj.isSpent).to.equal(true);
    });
  });

  describe('toJSON() / fromJSON()', () => {
    it('should encode nullifier as base64 string', () => {
      const status = sdk.ShieldedNullifierStatus.fromObject({
        nullifier: nullifierBytes,
        isSpent: false,
      });
      const json = status.toJSON();

      expect(json.nullifier).to.be.a('string');
      expect(json.isSpent).to.equal(false);
    });

    it('should round-trip via JSON', () => {
      const status = sdk.ShieldedNullifierStatus.fromObject({
        nullifier: nullifierBytes,
        isSpent: true,
      });
      const restored = sdk.ShieldedNullifierStatus.fromJSON(status.toJSON());

      expect(restored.nullifier).to.deep.equal(nullifierBytes);
      expect(restored.isSpent).to.equal(true);
    });
  });
});
