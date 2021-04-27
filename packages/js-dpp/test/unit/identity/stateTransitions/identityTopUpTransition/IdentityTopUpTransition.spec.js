const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const Identifier = require('../../../../../lib/identifier/Identifier');

const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

describe('IdentityTopUpTransition', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(() => {
    stateTransition = getIdentityTopUpTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#constructor', () => {
    it('should create an instance with specified data from specified raw transition', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.be.deep.equal(
        rawStateTransition.assetLockProof,
      );
      expect(stateTransition.getIdentityId()).to.be.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_TOP_UP);
    });
  });

  describe('#setAssetLockProof', () => {
    it('should set asset lock proof', () => {
      stateTransition.setAssetLockProof(rawStateTransition.assetLockProof);

      expect(stateTransition.assetLockProof).to.deep.equal(rawStateTransition.assetLockProof);
    });
  });

  describe('#getAssetLock', () => {
    it('should return currently set asset lock proof', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        rawStateTransition.assetLockProof,
      );
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        assetLockProof: rawStateTransition.assetLockProof,
        identityId: rawStateTransition.identityId,
        signature: undefined,
      });
    });

    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        assetLockProof: rawStateTransition.assetLockProof,
        identityId: rawStateTransition.identityId,
      });
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        assetLockProof: stateTransition.getAssetLockProof().toJSON(),
        identityId: Identifier(rawStateTransition.identityId).toString(),
        signature: undefined,
      });
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of topped up identity', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(1);
      const identityId = result[0];

      expect(identityId).to.be.an.instanceOf(Identifier);
      expect(identityId).to.be.deep.equal(rawStateTransition.identityId);
    });
  });

  describe('#isDataContractStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.false();
    });
  });

  describe('#isDocumentStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.false();
    });
  });

  describe('#isIdentityStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.true();
    });
  });
});
