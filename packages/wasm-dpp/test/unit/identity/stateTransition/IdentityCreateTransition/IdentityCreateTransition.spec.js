const IdentityPublicKeyJS = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const getChainAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('IdentityCreateTransition', () => {
  let stateTransitionJS;
  let stateTransition;
  let IdentityCreateTransition;
  let KeyType;
  let KeyPurpose;
  let KeySecurityLevel;
  let IdentityPublicKey;
  let AssetLockProof;

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
      AssetLockProof,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    stateTransitionJS = getIdentityCreateTransitionFixture();
    // Fill public keys with signatures (to test skipSignature later)
    stateTransitionJS.publicKeys = [new IdentityPublicKeyJS(mockRawPublicKey())];
    // Fill signature (to test skipSignature later)
    stateTransitionJS.signature = Buffer.alloc(32).fill(1);
    stateTransition = new IdentityCreateTransition(stateTransitionJS.toObject());
  });

  describe('#constructor', () => {
    it('should create instance with instant asset lock proof', () => {
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(stateTransitionJS.getAssetLockProof().toObject());

      expect(stateTransition.publicKeys.map((key) => key.toJSON()))
        .to.deep.equal(stateTransitionJS.publicKeys.map((key) => key.toJSON()));
    });

    it('should create instance with chain asset lock proof', () => {
      const stObject = stateTransitionJS.toObject();
      stObject.assetLockProof = getChainAssetLockProofFixture().toObject();
      stateTransition = new IdentityCreateTransition(stObject);
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(stObject.assetLockProof);
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionJS.getType());
    });
  });

  describe('#setAssetLockProof', () => {
    let assetLockProofObject;
    beforeEach(() => {
      const assetLockProof = new AssetLockProof(getChainAssetLockProofFixture().toObject());
      assetLockProofObject = assetLockProof.toObject();
      stateTransition.setAssetLockProof(assetLockProof);
    });

    it('should set asset lock proof', () => {
      expect(stateTransition.assetLockProof.toObject())
        .to.deep.equal(assetLockProofObject);
    });

    it('should set `identityId`', () => {
      stateTransition.setAssetLockProof(
        stateTransition.assetLockProof,
      );

      expect(stateTransition.identityId.toBuffer()).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier().toBuffer(),
      );
    });
  });

  describe('#getAssetLockProof', () => {
    it('should return currently set locked asset lock proof', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        stateTransitionJS.assetLockProof.toObject(),
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should set public keys', () => {
      const publicKeys = [
        new IdentityPublicKey(mockRawPublicKey({ id: 0 })),
        new IdentityPublicKey(mockRawPublicKey({ id: 1 })),
      ];

      // TODO: method accepts JS value and errors if we pass instances of IdentityPublicKey
      // Should it be fixed?
      stateTransition.setPublicKeys(publicKeys.map((key) => key.toObject()));
      stateTransitionJS.setPublicKeys(publicKeys);

      expect(stateTransition.publicKeys.map((key) => key.toObject()))
        .to.deep.equal(stateTransitionJS.publicKeys.map((key) => key.toObject()));
    });
  });

  describe('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(stateTransition.getPublicKeys().map((key) => key.toJSON()))
        .to.deep.equal(
          stateTransitionJS.getPublicKeys().map((key) => key.toJSON()),
        );
    });
  });

  describe('#addPublicKeys', () => {
    it('should add more public keys', () => {
      const publicKeys = [
        new IdentityPublicKey(mockRawPublicKey({ id: 0 })),
        new IdentityPublicKey(mockRawPublicKey({ id: 1 })),
      ];

      stateTransitionJS.publicKeys = [];
      stateTransitionJS.addPublicKeys(publicKeys);

      stateTransition.setPublicKeys([]);
      // TODO: method accepts JS value and errors if we pass instances of wasm IdentityPublicKey
      // Should it be fixed?
      stateTransition.addPublicKeys(publicKeys.map((key) => key.toObject()));

      expect(stateTransition.getPublicKeys().map((key) => key.toObject()))
        .to.deep.equal(stateTransitionJS.getPublicKeys().map((key) => key.toObject()));
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId().toBuffer())
        .to.deep.equal(stateTransitionJS.getIdentityId());

      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier().toBuffer(),
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId().toBuffer())
        .to.deep.equal(stateTransitionJS.getOwnerId());

      expect(stateTransition.getOwnerId().toBuffer()).to.deep.equal(
        stateTransition.getIdentityId().toBuffer(),
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      const stObject = stateTransition.toObject();
      const stObjectJS = stateTransitionJS.toObject();

      // TODO: fix? stObjectJS missing identityId.
      delete stObject.identityId;

      expect(stObject).to.deep.equal(stObjectJS);
    });

    it('should return raw state transition without signature', () => {
      const stObject = stateTransition.toObject({ skipSignature: true });
      const stObjectJS = stateTransitionJS.toObject({ skipSignature: true });

      // TODO: fix? stObjectJS missing identityId.
      delete stObject.identityId;

      expect(stObject.signature).to.not.exist();
      expect(stObject).to.deep.equal(stObjectJS);
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const stJson = stateTransition.toJSON();
      const stJsonJS = stateTransitionJS.toJSON();

      // TODO: fix? stObjectJS missing identityId.
      delete stJson.identityId;

      expect(stJson).to.deep.equal(stJsonJS);
    });
  });

  describe('#getModifiedDataIds', () => {
    it('should return ids of created identities', () => {
      const result = stateTransition.getModifiedDataIds();
      const resultJS = stateTransitionJS.getModifiedDataIds();

      expect(result.length).to.equal(resultJS.length);
      expect(result.map((id) => id.toBuffer())).to.deep.equal(resultJS);
      const identityId = result[0];

      expect(identityId.toBuffer()).to.be.deep.equal(
        stateTransition.getIdentityId().toBuffer(),
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
