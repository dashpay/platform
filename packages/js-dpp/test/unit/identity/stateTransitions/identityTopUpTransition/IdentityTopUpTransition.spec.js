const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const Identifier = require('../../../../../lib/identifier/Identifier');

const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');
const AssetLock = require('../../../../../lib/identity/stateTransitions/assetLock/AssetLock');

describe('IdentityTopUpTransition', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(() => {
    stateTransition = getIdentityTopUpTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#constructor', () => {
    it('should create an instance with specified data from specified raw transition', () => {
      expect(stateTransition.getAssetLock().toObject()).to.be.deep.equal(
        rawStateTransition.assetLock,
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

  describe('#setAssetLock', () => {
    it('should set asset lock', () => {
      const newAssetLock = new AssetLock({
        transaction: rawStateTransition.assetLock.transaction,
        outputIndex: 2,
        proof: rawStateTransition.assetLock.proof,
      });

      stateTransition.setAssetLock(newAssetLock);

      expect(stateTransition.assetLock).to.deep.equal(newAssetLock);
    });
  });

  describe('#getAssetLock', () => {
    it('should return currently set asset lock', () => {
      expect(stateTransition.getAssetLock().toObject()).to.deep.equal(
        rawStateTransition.assetLock,
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
        assetLock: rawStateTransition.assetLock,
        identityId: rawStateTransition.identityId,
        signature: undefined,
      });
    });

    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: 0,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        assetLock: rawStateTransition.assetLock,
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
        assetLock: stateTransition.getAssetLock().toJSON(),
        identityId: Identifier(rawStateTransition.identityId).toString(),
        signature: undefined,
      });
    });
  });
});
