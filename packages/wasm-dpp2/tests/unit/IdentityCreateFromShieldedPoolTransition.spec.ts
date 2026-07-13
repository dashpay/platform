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

describe('IdentityCreateFromShieldedPoolTransition', () => {
  const addrBytes = new Uint8Array([
    0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
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

  function createTransition(actionSeed = 1) {
    const fallbackAddr = wasm.PlatformAddress.fromBytes(addrBytes);

    return new wasm.IdentityCreateFromShieldedPoolTransition({
      publicKeys: [createPublicKey()],
      denomination: BigInt(10_000_000_000),
      actions: [fakeOrchardAction(actionSeed)],
      anchor: ZERO_ANCHOR,
      proof: ZERO_PROOF,
      bindingSignature: ZERO_BINDING_SIG,
      sendToAddressOnCreationFailure: fallbackAddr,
    });
  }

  describe('constructor()', () => {
    it('should construct with publicKeys + Orchard fields + fallback address', () => {
      const t = createTransition();
      expect(t).to.be.an.instanceof(wasm.IdentityCreateFromShieldedPoolTransition);
    });

    it('should reject missing sendToAddressOnCreationFailure', () => {
      expect(() => new wasm.IdentityCreateFromShieldedPoolTransition({
          publicKeys: [createPublicKey()],
          denomination: BigInt(10_000_000_000),
          actions: [fakeOrchardAction()],
          anchor: ZERO_ANCHOR,
          proof: ZERO_PROOF,
          bindingSignature: ZERO_BINDING_SIG,
        })).to.throw();
    });
  });

  describe('getters', () => {
    it('returns typed public keys', () => {
      const t = createTransition();
      expect(t.publicKeys[0]).to.be.an.instanceof(wasm.IdentityPublicKeyInCreation);
    });

    it('returns the denomination', () => {
      const t = createTransition();
      expect(t.denomination).to.equal(BigInt(10_000_000_000));
    });

    it('returns typed Orchard actions', () => {
      const t = createTransition();
      expect(t.actions[0]).to.be.an.instanceof(wasm.SerializedOrchardAction);
    });

    it('returns typed PlatformAddress for sendToAddressOnCreationFailure', () => {
      const t = createTransition();
      const addr = t.sendToAddressOnCreationFailure;
      expect(addr).to.be.an.instanceof(wasm.PlatformAddress);
      expect(addr.toBytes()).to.deep.equal(addrBytes);
    });

    // The identity id is consensus-derived from the spend nullifiers (the
    // constructor must not let callers pick an arbitrary id), so the same
    // actions must yield the same id and different nullifiers a different one.
    it('derives identityId deterministically from the action nullifiers', () => {
      const a = createTransition(1);
      const b = createTransition(1);
      const c = createTransition(7);
      expect(a.identityId.toHex()).to.equal(b.identityId.toHex());
      expect(a.identityId.toHex()).to.not.equal(c.identityId.toHex());
    });
  });

  describe('toBytes() / fromBytes()', () => {
    it('round-trips via bytes', () => {
      const t = createTransition();
      const bytes = t.toBytes();
      const restored = wasm.IdentityCreateFromShieldedPoolTransition.fromBytes(bytes);
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('toObject() / toJSON()', () => {
    it('toObject() emits the canonical wire shape', () => {
      const t = createTransition();
      const obj = t.toObject();
      expect(obj.$formatVersion).to.equal('0');
      expect(obj.denomination).to.equal(BigInt(10_000_000_000));
      expect(obj.sendToAddressOnCreationFailure).to.be.instanceOf(Uint8Array);
      expect(obj.sendToAddressOnCreationFailure.length).to.equal(21);
      expect(obj.identityId).to.be.instanceOf(Uint8Array);
      expect(obj.publicKeys).to.have.lengthOf(1);
    });

    it('toJSON() emits hex address + base58 identityId', () => {
      const t = createTransition();
      const json = t.toJSON();
      expect(json.$formatVersion).to.equal('0');
      expect(json.sendToAddressOnCreationFailure).to.be.a('string').with.lengthOf(42); // 21 bytes hex
      expect(json.identityId).to.be.a('string');
      expect(json.anchor).to.be.a('string');
    });

    it('fromJSON(toJSON()) round-trips', () => {
      const t = createTransition();
      const restored = wasm.IdentityCreateFromShieldedPoolTransition.fromJSON(t.toJSON());
      expect(Buffer.from(restored.toBytes())).to.deep.equal(Buffer.from(t.toBytes()));
    });
  });

  describe('toStateTransition()', () => {
    it('should convert to StateTransition wrapper', () => {
      const t = createTransition();
      expect(t.toStateTransition()).to.exist();
    });
  });
});
