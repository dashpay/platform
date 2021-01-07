const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const Identity = require('../../../../../lib/identity/Identity');
const IdentityCreateTransition = require('../../../../../lib/identity/stateTransitions/identityCreateTransition/IdentityCreateTransition');
const AssetLock = require('../../../../../lib/identity/stateTransitions/assetLock/AssetLock');
const Identifier = require('../../../../../lib/identifier/Identifier');

const getIdentityCreateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

describe('IdentityCreateTransition', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(() => {
    stateTransition = getIdentityCreateTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#constructor', () => {
    it('should create an instance with specified data', () => {
      expect(stateTransition.getAssetLock().toObject()).to.deep.equal(
        rawStateTransition.assetLock,
      );

      expect(stateTransition.publicKeys).to.deep.equal([
        new IdentityPublicKey(rawStateTransition.publicKeys[0]),
      ]);
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_CREATE);
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

    it('should set `identityId`', () => {
      expect(stateTransition.identityId).to.deep.equal(
        stateTransition.getAssetLock().createIdentifier(),
      );

      const newAssetLock = new AssetLock({
        transaction: rawStateTransition.assetLock.transaction,
        outputIndex: 2,
        proof: rawStateTransition.assetLock.proof,
      });

      stateTransition.setAssetLock(newAssetLock);

      expect(stateTransition.identityId).to.deep.equal(newAssetLock.createIdentifier());
    });
  });

  describe('#getAssetLock', () => {
    it('should return currently set locked OutPoint', () => {
      expect(stateTransition.getAssetLock().toObject()).to.deep.equal(
        rawStateTransition.assetLock,
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should set public keys', () => {
      const publicKeys = [new IdentityPublicKey(), new IdentityPublicKey()];

      stateTransition.setPublicKeys(publicKeys);

      expect(stateTransition.publicKeys).to.have.deep.members(publicKeys);
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(stateTransition.getPublicKeys()).to.deep.equal(
        rawStateTransition.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe('#addPublicKeys', () => {
    it('should add more public keys', () => {
      const publicKeys = [new IdentityPublicKey(), new IdentityPublicKey()];

      stateTransition.publicKeys = [];
      stateTransition.addPublicKeys(publicKeys);
      expect(stateTransition.getPublicKeys()).to.have.deep.members(publicKeys);
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId()).to.deep.equal(
        stateTransition.getAssetLock().createIdentifier(),
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.equal(
        stateTransition.getIdentityId(),
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: Identity.PROTOCOL_VERSION,
        type: stateTransitionTypes.IDENTITY_CREATE,
        assetLock: rawStateTransition.assetLock,
        publicKeys: rawStateTransition.publicKeys,
        signature: undefined,
      });
    });

    it('should return raw state transition without signature', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: Identity.PROTOCOL_VERSION,
        type: stateTransitionTypes.IDENTITY_CREATE,
        assetLock: rawStateTransition.assetLock,
        publicKeys: rawStateTransition.publicKeys,
      });
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        protocolVersion: Identity.PROTOCOL_VERSION,
        type: stateTransitionTypes.IDENTITY_CREATE,
        assetLock: stateTransition.getAssetLock().toJSON(),
        publicKeys: stateTransition.getPublicKeys().map((k) => k.toJSON()),
        signature: undefined,
      });
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of created identities', () => {
      const result = stateTransition.getModifiedDataIds();

      expect(result.length).to.be.equal(1);
      const identityId = result[0];

      expect(identityId).to.be.an.instanceOf(Identifier);
      expect(identityId).to.be.deep.equal(
        new IdentityCreateTransition(rawStateTransition).getIdentityId(),
      );
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
