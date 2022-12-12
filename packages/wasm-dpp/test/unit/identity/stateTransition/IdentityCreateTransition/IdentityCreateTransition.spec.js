const IdentityPublicKeyJS = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const stateTransitionTypes = require(
  '@dashevo/dpp/lib/stateTransition/stateTransitionTypes',
);

const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const Identifier = require('@dashevo/dpp/lib/identifier/Identifier');

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');
const InstantAssetLockProof = require('@dashevo/dpp/lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');

const { default: loadWasmDpp } = require('../../../../../dist');

describe('IdentityCreateTransition', () => {
  let rawStateTransition;
  let stateTransitionJS;
  let stateTransition;
  let IdentityCreateTransition;
  let IdentityPublicKey;
  let KeyType;
  let KeyPurpose;
  let KeySecurityLevel;

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
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    stateTransitionJS = getIdentityCreateTransitionFixture();
    // TODO: revisit
    // Provide publicKeys that have mocked signature, because in case of absence,
    // JS returns signature as undefined and wasm binding returns empty buffer
    // and we can not do deep.equal directly
    stateTransitionJS.publicKeys = [new IdentityPublicKeyJS(mockRawPublicKey())];
    // For the same reason we need to mock signature
    stateTransitionJS.signature = Buffer.alloc(32).fill(1);
    stateTransition = new IdentityCreateTransition(stateTransitionJS.toObject());
  });

  describe('#constructor', () => {
    it('should create an instance with specified data', () => {
      // console.log(stateTransition.getAssetLockProof().toObject());
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(stateTransition.getAssetLockProof().toObject());

      expect(stateTransition.publicKeys.map((key) => key.toJSON()))
        .to.deep.equal(stateTransitionJS.publicKeys.map((key) => key.toJSON()));
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionJS.getType());
    });
  });

  describe('#setAssetLockProof', () => {
    it('should set asset lock proof', () => {
      stateTransition.setAssetLockProof(
        stateTransitionJS.assetLockProof.toObject(),
      );

      expect(stateTransition.assetLockProof.toObject())
        .to.deep.equal(stateTransitionJS.assetLockProof.toObject());
    });

    it('should set `identityId`', () => {
      stateTransition.setAssetLockProof(
        // TODO: test with instance of binding assetLockProof
        stateTransitionJS.assetLockProof.toObject(),
      );

      expect(stateTransition.identityId.toBuffer()).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier().toBuffer(),
      );
    });
  });

  describe('#getAssetLockProof', () => {
    it('should return currently set locked OutPoint', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        stateTransitionJS.assetLockProof.toObject(),
      );
    });
  });

  describe('#setPublicKeys', () => {
    it('should set public keys', () => {
      const publicKeys = [
        new IdentityPublicKeyJS(mockRawPublicKey({ id: 0 })),
        new IdentityPublicKeyJS(mockRawPublicKey({ id: 1 })),
      ];

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
        new IdentityPublicKeyJS(mockRawPublicKey({ id: 0 })),
        new IdentityPublicKeyJS(mockRawPublicKey({ id: 1 })),
      ];

      stateTransitionJS.publicKeys = [];
      stateTransitionJS.addPublicKeys(publicKeys);

      stateTransition.setPublicKeys([]);
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

      expect(stObject.signature).to.deep.equal(stObjectJS.signature);

      // TODO: identityId is missing in JS object.
      // compare to `signature` option because it's returned as identityId
      // in case skipIdentifiersConversion is true
      expect(stObject.identityId).to.deep.equal(stObjectJS.signature);
      expect(stObject.assetLockProof.toObject()).to
        .deep.equal(stObjectJS.assetLockProof);
      expect(stObject.publicKeys.map((key) => key.toObject()))
        .to.deep.equal(stObjectJS.publicKeys);
      expect(stObject.type).to.equal(stObjectJS.type);

      // TODO: wasm-dpp version is 0 while JS version is 1
      // expect(stObject.protocolVersion).to.equal(stObjectJS.protocolVersion);
    });

    it('should return raw state transition without signature', () => {
      const stObject = stateTransition.toObject({ skipSignature: true });

      expect(stObject.signature).to.not.exist();
    });
  });

  describe('#toJSON', () => {
    it('should return JSON representation of state transition', () => {
      const stJson = stateTransition.toJSON();
      const stJsonJS = stateTransitionJS.toJSON();

      expect(stJson.signature).to.deep.equal(stJsonJS.signature);

      // TODO: identityId is missing in JS object.
      // compare to `signature` option because it's returned as identityId
      // in case skipIdentifiersConversion is true
      expect(stJson.identityId).to.deep.equal(stJsonJS.signature);
      expect(stJson.assetLockProof).to.deep.equal(stJsonJS.assetLockProof);
      expect(stJson.publicKeys).to.deep.equal(stJsonJS.publicKeys);
      expect(stJson.type).to.equal(stJsonJS.type);

      // TODO: wasm-dpp version is 0 while JS version is 1
      // expect(stJson.protocolVersion).to.equal(stJsonJS.protocolVersion);
      // console.log(stJsonJS);
    });
  });

  describe.skip('#getModifiedDataIds', () => {
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

  describe.skip('#isDataContractStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDataContractStateTransition()).to.be.false();
    });
  });

  describe.skip('#isDocumentStateTransition', () => {
    it('should return false', () => {
      expect(stateTransition.isDocumentStateTransition()).to.be.false();
    });
  });

  describe.skip('#isIdentityStateTransition', () => {
    it('should return true', () => {
      expect(stateTransition.isIdentityStateTransition()).to.be.true();
    });
  });
});
