const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const getChainAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const { default: loadWasmDpp } = require('../../../../../dist');

describe('IdentityCreateTransition', () => {
  let rawStateTransition;
  let stateTransition;

  let IdentityCreateTransition;
  let InstantAssetLockProof;
  let KeyType;
  let KeyPurpose;
  let KeySecurityLevel;
  let IdentityPublicKey;
  let Identifier;

  const mockRawPublicKey = (params = {}) => ({
    id: 0,
    type: KeyType.ECDSA_SECP256K1,
    data: Buffer.from('AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH', 'base64'),
    purpose: KeyPurpose.AUTHENTICATION,
    securityLevel: KeySecurityLevel.MASTER,
    signature: Buffer.alloc(32).fill(1),
    readOnly: false,
    ...params,
  });

  before(async () => {
    ({
      IdentityCreateTransition, IdentityPublicKey, KeyType, KeyPurpose, KeySecurityLevel,
      InstantAssetLockProof, Identifier,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    rawStateTransition = getIdentityCreateTransitionFixture().toObject();
    stateTransition = new IdentityCreateTransition(
      rawStateTransition,
    );
  });

  describe('#constructor', () => {
    it('should create instance with specified data', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        rawStateTransition.assetLockProof,
      );

      expect(stateTransition.publicKeys.map((key) => key.toObject())).to.deep.equal([
        new IdentityPublicKey(rawStateTransition.publicKeys[0]).toObject(),
      ]);
    });

    it('should create instance with chain asset lock proof', () => {
      const stObject = stateTransition.toObject();
      stObject.assetLockProof = getChainAssetLockProofFixture().toObject();
      stateTransition = new IdentityCreateTransition(stObject);
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(stObject.assetLockProof);
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_CREATE);
    });
  });

  describe('#setAssetLockProof', () => {
    it('should set asset lock proof', () => {
      stateTransition.setAssetLockProof(
        new InstantAssetLockProof(rawStateTransition.assetLockProof),
      );

      expect(stateTransition.assetLockProof.toObject())
        .to.deep.equal(rawStateTransition.assetLockProof);
    });

    it('should set `identityId`', () => {
      stateTransition.setAssetLockProof(
        new InstantAssetLockProof(rawStateTransition.assetLockProof),
      );

      expect(stateTransition.identityId.toBuffer()).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier().toBuffer(),
      );
    });
  });

  describe('#getAssetLockProof', () => {
    it('should return currently set locked OutPoint', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        rawStateTransition.assetLockProof,
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should set public keys', () => {
      const publicKeys = [
        new IdentityPublicKey(mockRawPublicKey({ id: 0 })),
        new IdentityPublicKey(mockRawPublicKey({ id: 1 })),
      ];

      stateTransition.setPublicKeys(publicKeys);

      expect(stateTransition.publicKeys.map((key) => key.toObject()))
        .to.have.deep.members(publicKeys.map((key) => key.toObject()));
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(stateTransition.getPublicKeys().map((key) => key.toObject())).to.deep.equal(
        rawStateTransition.publicKeys
          .map((rawPublicKey) => new IdentityPublicKey(rawPublicKey).toObject()),
      );
    });
  });

  describe('#addPublicKeys', () => {
    it('should add more public keys', () => {
      const publicKeys = [
        new IdentityPublicKey(mockRawPublicKey({ id: 0 })),
        new IdentityPublicKey(mockRawPublicKey({ id: 1 })),
      ];

      stateTransition.setPublicKeys([]);
      stateTransition.addPublicKeys(publicKeys);
      expect(stateTransition.getPublicKeys().map((key) => key.toObject()))
        .to.have.deep.members(publicKeys.map((key) => key.toObject()));
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier().toBuffer(),
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId().toBuffer()).to.deep.equal(
        stateTransition.getIdentityId().toBuffer(),
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_CREATE,
        assetLockProof: rawStateTransition.assetLockProof,
        publicKeys: rawStateTransition.publicKeys,
        signature: undefined,
      });
    });

    it('should return raw state transition without signature', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_CREATE,
        assetLockProof: rawStateTransition.assetLockProof,
        publicKeys: rawStateTransition.publicKeys,
      });
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const jsonStateTransition = stateTransition.toJSON();

      expect(jsonStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_CREATE,
        assetLockProof: stateTransition.getAssetLockProof().toJSON(),
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
      expect(identityId.toBuffer()).to.be.deep.equal(
        new IdentityCreateTransition(rawStateTransition).getIdentityId().toBuffer(),
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
