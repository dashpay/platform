const getIdentityUpdateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityUpdateTransitionFixture');

const { default: loadWasmDpp } = require('../../../../..');
const { getLatestProtocolVersion, StateTransitionTypes } = require('../../../../..');
const generateRandomIdentifierAsync = require('../../../../../lib/test/utils/generateRandomIdentifierAsync');

describe('IdentityUpdateTransition', () => {
  let rawStateTransition;
  let stateTransition;

  let IdentityPublicKey;
  let Identifier;
  let IdentityPublicKeyWithWitness;

  before(async () => {
    ({
      IdentityPublicKey,
      Identifier,
      IdentityPublicKeyWithWitness,
    } = await loadWasmDpp());
  });

  beforeEach(async () => {
    stateTransition = await getIdentityUpdateTransitionFixture();
    rawStateTransition = stateTransition.toObject();
  });

  describe('#getType', () => {
    it('should return IDENTITY_UPDATE type', () => {
      expect(stateTransition.getType()).to.equal(StateTransitionTypes.IdentityUpdate);
    });
  });

  describe('#setIdentityId', () => {
    it('should set identityId', async () => {
      const id = await generateRandomIdentifierAsync();

      stateTransition.setIdentityId(id);

      expect(stateTransition.identityId.toBuffer())
        .to.deep.equal(id.toBuffer());
    });
  });

  describe('#getIdentityId', () => {
    it('should return identityId', () => {
      expect(stateTransition.getIdentityId().toBuffer())
        .to.deep.equal(rawStateTransition.identityId);
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

      expect(stateTransition.getRevision()).to.equal(42);
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId().toBuffer()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getPublicKeysToAdd', () => {
    it('should return public keys to add', () => {
      expect(stateTransition.getPublicKeysToAdd().map((key) => key.toObject()))
        .to.deep.equal(
          rawStateTransition.addPublicKeys
            .map((rawPublicKey) => new IdentityPublicKeyWithWitness(rawPublicKey).toObject()),
        );
    });
  });

  describe('#setPublicKeysToAdd', () => {
    it('should set public keys to add', () => {
      const publicKeys = [new IdentityPublicKeyWithWitness({
        id: 0,
        type: IdentityPublicKey.TYPES.BLS12_381,
        purpose: 0,
        securityLevel: 0,
        readOnly: true,
        signature: Buffer.alloc(0),
        data: Buffer.from('01fac99ca2c8f39c286717c213e190aba4b7af76db320ec43f479b7d9a2012313a0ae59ca576edf801444bc694686694', 'hex'),
      })];

      stateTransition.setPublicKeysToAdd(publicKeys);

      expect(stateTransition.addPublicKeys.map((key) => key.toObject()))
        .to.have.deep.members(publicKeys.map((key) => key.toObject()));
    });
  });

  describe('#getPublicKeyIdsToDisable', () => {
    it('should return public key ids to disable', () => {
      expect(stateTransition.getPublicKeyIdsToDisable())
        .to.deep.equal(rawStateTransition.disablePublicKeys);
    });
  });

  describe('#setPublicKeyIdsToDisable', () => {
    it('should set public key ids to disable', () => {
      stateTransition.setPublicKeyIdsToDisable([1, 2]);

      expect(stateTransition.getPublicKeyIdsToDisable())
        .to.deep.equal([1, 2]);
    });
  });

  describe('#getPublicKeysDisabledAt', () => {
    it('should return time to disable public keys', () => {
      expect(stateTransition.getPublicKeysDisabledAt())
        .to.deep.equal(new Date(rawStateTransition.publicKeysDisabledAt));
    });
  });

  describe('#setPublicKeysDisabledAt', () => {
    it('should set time to disable public keys', () => {
      const now = new Date();

      stateTransition.setPublicKeysDisabledAt(now);

      expect(stateTransition.getPublicKeysDisabledAt()).to.deep.equal(new Date(now));
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: getLatestProtocolVersion(),
        type: StateTransitionTypes.IdentityUpdate,
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
        protocolVersion: getLatestProtocolVersion(),
        type: StateTransitionTypes.IdentityUpdate,
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
        protocolVersion: getLatestProtocolVersion(),
        type: StateTransitionTypes.IdentityUpdate,
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
        protocolVersion: getLatestProtocolVersion(),
        type: StateTransitionTypes.IdentityUpdate,
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
      expect(identityId.toBuffer()).to.be.deep.equal(rawStateTransition.identityId);
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
