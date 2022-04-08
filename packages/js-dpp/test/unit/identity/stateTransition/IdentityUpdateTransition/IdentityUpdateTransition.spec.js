const IdentityPublicKey = require('../../../../../lib/identity/IdentityPublicKey');

const stateTransitionTypes = require(
  '../../../../../lib/stateTransition/stateTransitionTypes',
);

const protocolVersion = require('../../../../../lib/version/protocolVersion');
const Identifier = require('../../../../../lib/identifier/Identifier');

const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');
const generateRandomIdentifier = require('../../../../../lib/test/utils/generateRandomIdentifier');

describe('IdentityUpdateTransition', () => {
  let rawStateTransition;
  let stateTransition;

  beforeEach(() => {
    stateTransition = getIdentityUpdateTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#getType', () => {
    it('should return IDENTITY_UPDATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_UPDATE);
    });
  });

  describe('#setIdentityId', () => {
    it('should set identityId', () => {
      const id = generateRandomIdentifier();

      stateTransition.setIdentityId(id);

      expect(stateTransition.identityId).to.deep.equal(id);
    });
  });

  describe('#getIdentityId', () => {
    it('should return identityId', () => {
      expect(stateTransition.getIdentityId()).to.deep.equal(rawStateTransition.identityId);
    });
  });

  describe('#getRevision', () => {
    it('should return revision', () => {
      expect(stateTransition.getRevision()).to.equal(rawStateTransition.revision);
    });
  });

  describe('#setRevision', () => {
    it('should set revision', () => {
      stateTransition.setRevision(42);

      expect(stateTransition.revision).to.equal(42);
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getPublicKeysToAdd', () => {
    it('should return public keys to add', () => {
      expect(stateTransition.getPublicKeysToAdd()).to.deep.equal(
        rawStateTransition.addPublicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe('#setPublicKeysToAdd', () => {
    it('should set public keys to add', () => {
      const publicKeys = [new IdentityPublicKey({
        id: 0,
        type: IdentityPublicKey.TYPES.BLS12_381,
        purpose: 0,
        securityLevel: 0,
        readOnly: true,
        data: Buffer.from('01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694', 'hex'),
      })];

      stateTransition.setPublicKeysToAdd(publicKeys);

      expect(stateTransition.addPublicKeys).to.have.deep.members(publicKeys);
    });
  });

  describe('#getPublicKeyIdsToDisable', () => {
    it('should return public key ids to disable', () => {
      expect(stateTransition.getPublicKeyIdsToDisable())
        .to.deep.equal(stateTransition.disablePublicKeys);
    });
  });

  describe('#setPublicKeyIdsToDisable', () => {
    it('should set public key ids to disable', () => {
      stateTransition.setPublicKeyIdsToDisable([1, 2]);

      expect(stateTransition.disablePublicKeys).to.deep.equal([1, 2]);
    });
  });

  describe('#getPublicKeysDisabledAt', () => {
    it('should return time to disable public keys', () => {
      expect(stateTransition.getPublicKeysDisabledAt())
        .to.deep.equal(new Date(stateTransition.publicKeysDisabledAt));
    });
  });

  describe('#setPublicKeysDisabledAt', () => {
    it('should set time to disable public keys', () => {
      const now = new Date();

      stateTransition.setPublicKeysDisabledAt(now);

      expect(stateTransition.publicKeysDisabledAt).to.deep.equal(new Date(now));
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_UPDATE,
        signature: undefined,
        identityId: rawStateTransition.identityId,
        revision: rawStateTransition.revision,
        publicKeysDisabledAt: rawStateTransition.publicKeysDisabledAt,
        addPublicKeys: rawStateTransition.addPublicKeys,
        disablePublicKeys: rawStateTransition.disablePublicKeys,
        signaturePublicKeyId: undefined,
      });
    });

    it('should return raw state transition without signature', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_UPDATE,
        identityId: rawStateTransition.identityId,
        revision: rawStateTransition.revision,
        publicKeysDisabledAt: rawStateTransition.publicKeysDisabledAt,
        addPublicKeys: rawStateTransition.addPublicKeys,
        disablePublicKeys: rawStateTransition.disablePublicKeys,
      });
    });

    it('should return raw state transition without optional properties', () => {
      stateTransition.setPublicKeyIdsToDisable(undefined);
      stateTransition.setPublicKeysDisabledAt(undefined);
      stateTransition.setPublicKeysToAdd(undefined);

      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_UPDATE,
        signature: undefined,
        identityId: rawStateTransition.identityId,
        revision: rawStateTransition.revision,
        signaturePublicKeyId: undefined,
      });
    });
  });

  describe('#toJSON ', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_UPDATE,
        signature: undefined,
        identityId: stateTransition.getIdentityId().toString(),
        revision: rawStateTransition.revision,
        publicKeysDisabledAt: rawStateTransition.publicKeysDisabledAt,
        addPublicKeys: stateTransition.getPublicKeysToAdd().map((k) => k.toJSON()),
        disablePublicKeys: rawStateTransition.disablePublicKeys,
        signaturePublicKeyId: undefined,
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
