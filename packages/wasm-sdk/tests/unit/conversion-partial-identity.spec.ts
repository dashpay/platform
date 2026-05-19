import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('PartialIdentity Conversions', () => {
  const testId = 'H2pb35GtKpjLinncBYeMsXkdDYXCbsFzzVmssce6pSJ1';

  // 33-byte compressed ECDSA secp256k1 public key
  const pubKeyBytes = new Uint8Array([
    0x02, 0xea, 0xb2, 0x22, 0xe3, 0x2d, 0x46, 0xb9, 0x7f, 0x56, 0xfb,
    0x90, 0xbb, 0x22, 0xc3, 0xd6, 0x5e, 0x27, 0x9b, 0x18, 0xbd, 0xa2,
    0x03, 0xf3, 0x0b, 0xd2, 0xd3, 0xee, 0xd7, 0x69, 0xa3, 0x47, 0x62,
  ]);
  const pubKeyBase64 = Buffer.from(pubKeyBytes).toString('base64');

  before(async () => {
    await init();
  });

  describe('toJSON()', () => {
    it('should serialize with Base58 id and null for missing optional fields', () => {
      const pi = new sdk.PartialIdentity({
        id: testId,
        loadedPublicKeys: {},
        notFoundPublicKeys: [1, 2],
      });

      const json = pi.toJSON();

      expect(json.id).to.equal(testId);
      expect(json.loadedPublicKeys).to.deep.equal({});
      expect(json.balance).to.equal(null);
      expect(json.revision).to.equal(null);
      expect(json.notFoundPublicKeys).to.deep.equal([1, 2]);

      pi.free();
    });

    it('should serialize with balance and revision when present', () => {
      const pi = new sdk.PartialIdentity({
        id: testId,
        loadedPublicKeys: {},
        balance: 500000000n,
        revision: 3n,
      });

      const json = pi.toJSON();

      expect(json.balance).to.equal(500000000);
      expect(json.revision).to.equal(3);

      pi.free();
    });

    it('should serialize loaded public keys as JSON objects', () => {
      const key = new sdk.IdentityPublicKey({
        keyId: 0,
        purpose: 0,
        securityLevel: 0,
        keyType: 0,
        isReadOnly: false,
        data: pubKeyBytes,
      });

      const pi = new sdk.PartialIdentity({
        id: testId,
        loadedPublicKeys: { 0: key },
        balance: 100n,
        revision: 1n,
      });

      const json = pi.toJSON();

      expect(json.loadedPublicKeys).to.have.property('0');
      expect(json.loadedPublicKeys['0'].data).to.equal(pubKeyBase64);

      pi.free();
    });
  });

  describe('fromJSON()', () => {
    it('should deserialize from JSON fixture with empty keys', () => {
      const fixture = {
        id: testId,
        loadedPublicKeys: {},
        balance: 500000000,
        revision: 3,
        notFoundPublicKeys: [1, 2],
      };

      const pi = sdk.PartialIdentity.fromJSON(fixture);

      expect(pi.id.toBase58()).to.equal(testId);
      expect(pi.balance).to.equal(500000000n);
      expect(pi.revision).to.equal(3n);
      expect(pi.notFoundPublicKeys).to.deep.equal([1, 2]);

      pi.free();
    });

    it('should deserialize from JSON fixture with null balance and revision', () => {
      const fixture = {
        id: testId,
        loadedPublicKeys: {},
        balance: null,
        revision: null,
        notFoundPublicKeys: [],
      };

      const pi = sdk.PartialIdentity.fromJSON(fixture);

      expect(pi.id.toBase58()).to.equal(testId);
      expect(pi.balance).to.be.undefined();
      expect(pi.revision).to.be.undefined();

      pi.free();
    });

    it('should round-trip through JSON', () => {
      const pi = new sdk.PartialIdentity({
        id: testId,
        loadedPublicKeys: {},
        balance: 42n,
        revision: 1n,
        notFoundPublicKeys: [5],
      });

      const json = pi.toJSON();
      const restored = sdk.PartialIdentity.fromJSON(json);
      const json2 = restored.toJSON();

      expect(json2).to.deep.equal(json);

      pi.free();
      restored.free();
    });
  });

  describe('toObject()', () => {
    it('should serialize with Uint8Array id and BigInt for u64 fields', () => {
      const pi = new sdk.PartialIdentity({
        id: testId,
        loadedPublicKeys: {},
        balance: 500000000n,
        revision: 3n,
        notFoundPublicKeys: [1, 2],
      });

      const obj = pi.toObject();

      expect(obj.id).to.be.instanceOf(Uint8Array);
      expect(obj.id.length).to.equal(32);
      expect(obj.balance).to.equal(500000000n);
      expect(obj.revision).to.equal(3n);
      expect(obj.notFoundPublicKeys).to.deep.equal([1, 2]);

      pi.free();
    });
  });

  describe('fromObject()', () => {
    it('should round-trip through Object', () => {
      const pi = new sdk.PartialIdentity({
        id: testId,
        loadedPublicKeys: {},
        balance: 99n,
        revision: 2n,
        notFoundPublicKeys: [3],
      });

      const obj = pi.toObject();
      const restored = sdk.PartialIdentity.fromObject(obj);

      expect(restored.id.toBase58()).to.equal(testId);
      expect(restored.balance).to.equal(99n);
      expect(restored.revision).to.equal(2n);

      pi.free();
      restored.free();
    });
  });
});
