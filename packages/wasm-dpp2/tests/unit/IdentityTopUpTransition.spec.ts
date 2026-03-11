import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentityTopUpTransition', () => {
  // Shared test data - instant lock bytes
  // prettier-ignore
  const instantLockBytes = [
    1, 1, 193, 219, 244, 102, 77, 20, 251, 187, 201, 109, 168, 125, 7, 244, 118, 119, 210, 53,
    238, 105, 138, 31, 7, 73, 30, 128, 131, 175, 114, 76, 187, 37, 0, 0, 0, 0, 177, 215, 65,
    96, 204, 228, 86, 13, 14, 185, 46, 65, 241, 38, 226, 172, 59, 96, 158, 15, 126, 90, 225,
    3, 140, 221, 96, 131, 254, 12, 236, 26, 48, 124, 31, 23, 184, 202, 122, 231, 123, 59, 43,
    118, 27, 214, 225, 58, 44, 191, 232, 10, 33, 35, 8, 113, 145, 159, 158, 88, 80, 0, 0, 0,
    173, 103, 128, 160, 29, 226, 161, 219, 167, 194, 158, 38, 19, 51, 143, 248, 161, 87, 126,
    32, 143, 209, 152, 44, 174, 6, 25, 210, 101, 127, 131, 65, 202, 241, 47, 166, 132, 10,
    199, 15, 187, 136, 11, 217, 237, 13, 173, 64, 11, 6, 112, 188, 234, 239, 204, 29, 3, 6,
    35, 154, 44, 106, 44, 183, 171, 126, 146, 240, 153, 210, 187, 56, 133, 161, 11, 4, 151,
    63, 89, 20, 44, 66, 153, 242, 97, 207, 44, 110, 208, 51, 198, 113, 104, 79, 154, 19,
  ];

  // Shared test data - transaction bytes
  // prettier-ignore
  const transactionBytes = [
    3, 0, 8, 0, 1, 193, 219, 244, 102, 77, 20, 251, 187, 201, 109, 168, 125, 7, 244, 118, 119,
    210, 53, 238, 105, 138, 31, 7, 73, 30, 128, 131, 175, 114, 76, 187, 37, 0, 0, 0, 0, 106,
    71, 48, 68, 2, 32, 2, 146, 73, 163, 223, 124, 140, 225, 109, 126, 239, 87, 105, 184, 118,
    87, 182, 100, 182, 6, 15, 200, 26, 44, 33, 215, 165, 110, 211, 232, 245, 233, 2, 32, 69,
    161, 168, 128, 101, 26, 171, 20, 63, 30, 219, 87, 23, 4, 142, 115, 73, 73, 170, 203, 46,
    187, 149, 32, 210, 171, 46, 136, 253, 188, 36, 12, 1, 33, 2, 144, 231, 28, 153, 30, 92,
    52, 10, 111, 47, 107, 74, 225, 237, 151, 33, 188, 184, 247, 121, 70, 135, 174, 31, 160,
    16, 216, 239, 225, 103, 88, 112, 255, 255, 255, 255, 2, 64, 66, 15, 0, 0, 0, 0, 0, 2,
    106, 0, 216, 154, 230, 5, 0, 0, 0, 0, 25, 118, 169, 20, 229, 154, 45, 140, 145, 114, 18,
    52, 205, 13, 179, 84, 94, 174, 149, 207, 101, 115, 51, 240, 136, 172, 0, 0, 0, 0, 36, 1,
    1, 64, 66, 15, 0, 0, 0, 0, 0, 25, 118, 169, 20, 176, 213, 107, 82, 203, 147, 209, 128,
    255, 63, 69, 219, 41, 250, 232, 254, 185, 168, 85, 184, 136, 172,
  ];

  const testIdentityId = 'B7kcE1juMBWEWkuYRJhVdAE2e6RaevrGxRsa1DrLCpQH';

  function createAssetLockProof() {
    return wasm.AssetLockProof.createInstantAssetLockProof(
      instantLockBytes,
      transactionBytes,
      0,
    );
  }

  function createTransition() {
    return new wasm.IdentityTopUpTransition({
      assetLockProof: createAssetLockProof(),
      identityId: testIdentityId,
      userFeeIncrease: 11,
    });
  }

  describe('constructor()', () => {
    it('should create IdentityTopUpTransition', () => {
      const assetLockProof = createAssetLockProof();
      const transition = new wasm.IdentityTopUpTransition({
        assetLockProof,
        identityId: testIdentityId,
        userFeeIncrease: 11,
      });

      expect(transition).to.be.an.instanceof(wasm.IdentityTopUpTransition);
      expect(assetLockProof).to.be.an.instanceof(wasm.AssetLockProof);
    });
  });

  describe('toBase64()', () => {
    it('should convert IdentityTopUpTransition to base64', () => {
      const transition = createTransition();

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      expect(Buffer.from(base64, 'base64')).to.deep.equal(Buffer.from(bytes));
    });
  });

  describe('fromBase64()', () => {
    it('should restore IdentityTopUpTransition from base64', () => {
      const transition = createTransition();

      const base64 = transition.toBase64();
      const bytes = transition.toBytes();

      const restored = wasm.IdentityTopUpTransition.fromBase64(base64);

      expect(restored.toBytes()).to.deep.equal(bytes);
    });
  });

  describe('userFeeIncrease', () => {
    it('should return userFeeIncrease', () => {
      const transition = createTransition();

      expect(transition.userFeeIncrease).to.deep.equal(11);
    });

    it('should set userFeeIncrease', () => {
      const transition = createTransition();

      transition.userFeeIncrease = 21;

      expect(transition.userFeeIncrease).to.deep.equal(21);
    });
  });

  describe('identityIdentifier', () => {
    it('should return identityIdentifier', () => {
      const transition = createTransition();

      expect(transition.identityIdentifier.toBase58()).to.deep.equal(testIdentityId);
    });

    it('should set identityIdentifier', () => {
      const transition = createTransition();

      const identifier = new wasm.Identifier('777cE1juMBWEWkuYRJhVdAE2e6RaevrGxRsa1DrLCpQH');

      transition.identityIdentifier = identifier;

      expect(transition.identityIdentifier.toBase58()).to.deep.equal(
        '777cE1juMBWEWkuYRJhVdAE2e6RaevrGxRsa1DrLCpQH',
      );
    });
  });

  describe('signature', () => {
    it('should return signature', () => {
      const transition = createTransition();

      expect(transition.signature).to.deep.equal(Uint8Array.from([]));
    });

    it('should set signature', () => {
      const transition = createTransition();

      transition.signature = Uint8Array.from([1, 1, 1]);

      expect(transition.signature).to.deep.equal(Uint8Array.from([1, 1, 1]));
    });
  });

  describe('assetLockProof', () => {
    it('should return AssetLockProof instance', () => {
      const transition = createTransition();

      expect(transition.assetLockProof).to.be.an.instanceof(wasm.AssetLockProof);
    });
  });

  describe('toJSON()', () => {
    it('should produce expected JSON structure', () => {
      const transition = createTransition();

      const json = transition.toJSON();

      expect(json.$formatVersion).to.equal('0');
      expect(json.identityId).to.equal(testIdentityId);
      expect(json.assetLockProof).to.be.an('object');
      expect(json.assetLockProof).to.have.property('instantLock');
      expect(json.assetLockProof).to.have.property('transaction');
      expect(json.assetLockProof.outputIndex).to.equal(0);
      expect(json.userFeeIncrease).to.equal(11);
      expect(json.signature).to.equal('');
    });
  });

  describe('fromJSON()', () => {
    it('should restore transition from JSON and verify getters', () => {
      const transition = createTransition();

      const json = transition.toJSON();
      const restored = wasm.IdentityTopUpTransition.fromJSON(json);

      expect(restored.identityIdentifier.toBase58()).to.equal(testIdentityId);
      expect(restored.userFeeIncrease).to.equal(11);
      expect(restored.signature).to.deep.equal(Uint8Array.from([]));
      expect(restored.assetLockProof).to.be.an.instanceof(wasm.AssetLockProof);
      // Verify bytes round-trip
      expect(restored.toBytes()).to.deep.equal(transition.toBytes());
    });
  });

  describe('toObject()', () => {
    it('should produce expected object structure', () => {
      const transition = createTransition();

      const obj = transition.toObject();

      expect(obj.$formatVersion).to.equal('0');
      expect(obj.identityId).to.be.instanceOf(Uint8Array);
      expect(obj.identityId.length).to.equal(32);
      expect(obj.assetLockProof).to.be.an('object');
      expect(obj.userFeeIncrease).to.equal(11);
      expect(obj.signature).to.be.instanceOf(Uint8Array);
      expect(obj.signature.length).to.equal(0);
    });
  });
});
