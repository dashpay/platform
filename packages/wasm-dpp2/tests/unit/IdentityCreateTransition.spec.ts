import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityCreateTransition', () => {
  describe('default()', () => {
    it('should create transition with default values', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition).to.be.an.instanceof(wasm.IdentityCreateTransition);
    });
  });

  describe('toBytes()', () => {
    it('should serialize transition to bytes', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const bytes = transition.toBytes();

      expect(bytes.length > 0).to.equal(true);
    });
  });

  describe('fromBytes()', () => {
    it('should deserialize transition from bytes', () => {
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
  });

  describe('toJSON()', () => {
    it('should serialize transition to JSON', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const json = transition.toJSON();
      // JSON uses camelCase (serde rename_all)
      expect(json).to.have.property('publicKeys');
      expect(json).to.have.property('assetLockProof');
      expect(json).to.have.property('userFeeIncrease');
      expect(json).to.have.property('signature');
      expect(json.userFeeIncrease).to.equal(0);
    });
  });

  describe('toObject()', () => {
    it('should serialize transition to object', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const obj = transition.toObject();
      expect(obj).to.have.property('publicKeys');
      expect(obj).to.have.property('assetLockProof');
      expect(obj).to.have.property('userFeeIncrease');
      expect(obj).to.have.property('signature');
    });
  });

  describe('toHex()', () => {
    it('should serialize transition to hex string', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const hex = transition.toHex();
      expect(hex).to.be.a('string');
      expect(hex.length).to.be.greaterThan(0);
    });
  });

  describe('fromHex()', () => {
    it('should deserialize transition from hex string', () => {
      const transition = wasm.IdentityCreateTransition.default(1);
      const hex = transition.toHex();

      const fromHex = wasm.IdentityCreateTransition.fromHex(hex);
      expect(fromHex.identityId.toBase58()).to.equal(transition.identityId.toBase58());
    });
  });

  describe('toBase64()', () => {
    it('should serialize transition to base64 string', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      const base64 = transition.toBase64();
      expect(base64).to.be.a('string');
      expect(base64.length).to.be.greaterThan(0);
    });
  });

  describe('fromBase64()', () => {
    it('should deserialize transition from base64 string', () => {
      const transition = wasm.IdentityCreateTransition.default(1);
      const base64 = transition.toBase64();

      const fromBase64 = wasm.IdentityCreateTransition.fromBase64(base64);
      expect(fromBase64.identityId.toBase58()).to.equal(transition.identityId.toBase58());
    });
  });

  describe('userFeeIncrease', () => {
    it('should return userFeeIncrease', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.userFeeIncrease).to.equal(0);
    });

    it('should set userFeeIncrease', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      transition.userFeeIncrease = 100;

      expect(transition.userFeeIncrease).to.equal(100);
    });
  });

  describe('assetLockProof', () => {
    it('should return AssetLockProof instance', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.assetLockProof).to.be.an.instanceof(wasm.AssetLockProof);
    });
  });

  describe('identityId', () => {
    it('should return identity Identifier', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.identityId.toBase58()).to.equal('11111111111111111111111111111111');
    });
  });

  describe('publicKeys', () => {
    it('should return public keys array', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.publicKeys.length).to.equal(0);
    });
  });

  describe('signature', () => {
    it('should return signature bytes', () => {
      const transition = wasm.IdentityCreateTransition.default(1);

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });
  });

  describe('getSignableBytes()', () => {
    it('should return signable bytes', () => {
      const transition = wasm.IdentityCreateTransition.default(1);
      const st = transition.toStateTransition();

      expect(st.getSignableBytes().length).to.equal(229);
    });
  });

  // TODO: Implement publickeys in creation setter
  // describe('setPublicKeys()', () => {
  //   it('should set public keys', () => {
  //   });
  // });

  // TODO: Implement asset lock setter
  // describe('setAssetLockProof()', () => {
  //   it('should set asset lock proof', () => {
  //   });
  // });
});
