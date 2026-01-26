import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreateTransition', () => {
  describe('serialization / deserialization', () => {
    it('should allow to create transition', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition).to.be.an.instanceof(wasm.IdentityCreateTransition);
    });

    it('should allow to serialize to bytes', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const bytes = transition.toBytes();

      expect(bytes.length > 0).to.equal(true);
    });

    it('should allow to deserialize to bytes', () => {
      // prettier-ignore
      const bytes = [
        0, 0, 0, 162, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 60, 0,
        0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 1, 255, 255, 255,
        255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      ];

      const transition = wasm.IdentityCreateTransition.fromBytes(bytes);

      expect(transition).to.be.an.instanceof(wasm.IdentityCreateTransition);
    });

    it('should serialize to JSON', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const json = transition.toJSON();
      // JSON uses camelCase (serde rename_all)
      expect(json).to.have.property('publicKeys');
      expect(json).to.have.property('assetLockProof');
      expect(json).to.have.property('userFeeIncrease');
      expect(json).to.have.property('signature');
      expect(json.userFeeIncrease).to.equal(0);
    });

    it('should serialize to object', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const obj = transition.toObject();
      expect(obj).to.have.property('publicKeys');
      expect(obj).to.have.property('assetLockProof');
      expect(obj).to.have.property('userFeeIncrease');
      expect(obj).to.have.property('signature');
    });

    it('should serialize to hex and base64', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const hex = transition.toHex();
      expect(hex).to.be.a('string');
      expect(hex.length).to.be.greaterThan(0);

      const base64 = transition.toBase64();
      expect(base64).to.be.a('string');
      expect(base64.length).to.be.greaterThan(0);

      // Verify they can be deserialized back
      const fromHex = wasm.IdentityCreateTransition.fromHex(hex);
      expect(fromHex.identityId.toBase58()).to.equal(transition.identityId.toBase58());

      const fromBase64 = wasm.IdentityCreateTransition.fromBase64(base64);
      expect(fromBase64.identityId.toBase58()).to.equal(transition.identityId.toBase58());
    });
  });

  describe('getters', () => {
    it('should allow to get userFeeIncrease', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should allow to get AssetLockProof', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.assetLockProof).to.be.an.instanceof(wasm.AssetLockProof);
    });

    it('should allow to get Identifier', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.identityId.toBase58()).to.equal('11111111111111111111111111111111');
    });

    it('should allow to get PublicKeys', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.publicKeys.length).to.equal(0);
    });

    it('should allow to get signature', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should allow to get signable bytes', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.getSignableBytes().length).to.equal(229);
    });
  });

  describe('setters', () => {
    it('should allow to set the userFeeIncrease', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      transition.userFeeIncrease = 100;

      expect(transition.userFeeIncrease).to.equal(100);
    });

    // TODO: Implement publickeys in creation setter
    // it('should allow to set the publicKeys', function () {
    //
    // })

    // TODO: Implement asset lock setter
    // it('should allow to set the asset lock', function () {
    //
    // })
  });
});
