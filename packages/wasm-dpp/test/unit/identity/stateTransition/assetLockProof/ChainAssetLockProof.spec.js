const crypto = require('crypto');

const { default: loadWasmDpp } = require('../../../../../dist');
const getRawChainAssetLockProofFixture = require('../../../../../lib/test/fixtures/getRawChainAssetLockProofFixture');

function sha256(payload) {
  return crypto.createHash('sha256')
    .update(payload)
    .digest();
}

describe('ChainAssetLockProof', () => {
  let ChainAssetLockProof;
  let Identifier;
  let rawChainAssetLockProof;
  let chainAssetLockProof;

  before(async () => {
    ({ ChainAssetLockProof, Identifier } = await loadWasmDpp());

    rawChainAssetLockProof = getRawChainAssetLockProofFixture();
  });

  beforeEach(() => {
    const { coreChainLockedHeight, outPoint } = rawChainAssetLockProof;

    chainAssetLockProof = new ChainAssetLockProof({
      coreChainLockedHeight,
      outPoint,
    });
  });

  describe('#getType', () => {
    it('should return correct type', () => {
      expect(chainAssetLockProof.getType())
        .to.equal(rawChainAssetLockProof.type);
    });
  });

  describe('#getCoreChainLockedHeight', () => {
    it('should return correct coreChainLockedHeight', () => {
      expect(chainAssetLockProof.getCoreChainLockedHeight())
        .to.equal(rawChainAssetLockProof.coreChainLockedHeight);
    });
  });

  describe('#getOutPoint', () => {
    it('should return correct outPoint', () => {
      expect(chainAssetLockProof.getOutPoint())
        .to.deep.equal(rawChainAssetLockProof.outPoint);
    });
  });

  describe('#toJSON', () => {
    it('should return correct JSON', () => {
      const json = {
        ...rawChainAssetLockProof,
        outPoint: rawChainAssetLockProof.outPoint.toString('base64'),
      };

      expect(chainAssetLockProof.toJSON())
        .to.deep.equal(json);
    });
  });

  describe('#toObject', () => {
    it('should return correct object', () => {
      expect(chainAssetLockProof.toObject())
        .to.deep.equal(rawChainAssetLockProof);
    });
  });

  describe('#createIdentifier', () => {
    it('should return correct identifier', () => {
      const identifier = chainAssetLockProof.createIdentifier();

      const expectedIdentifier = new Identifier(
        sha256(sha256(rawChainAssetLockProof.outPoint)),
      );

      expect(identifier.toBuffer())
        .to.deep.equal(expectedIdentifier.toBuffer());
    });
  });
});
