const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

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

  before(async () => {
    ({ IdentityCreateTransition } = await loadWasmDpp());
  });

  beforeEach(() => {
    stateTransitionJS = getIdentityCreateTransitionFixture();
    rawStateTransition = stateTransitionJS.toObject();

    stateTransition = new IdentityCreateTransition(rawStateTransition);
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

  describe.skip('#getType', () => {
    it('should return IDENTITY_CREATE type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_CREATE);
    });
  });

  describe.skip('#setAssetLockProof', () => {
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

      expect(stateTransition.identityId).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier(),
      );
    });
  });

  describe.skip('#getAssetLockProof', () => {
    it('should return currently set locked OutPoint', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        rawStateTransition.assetLockProof,
      );
    });
  });

  describe.skip('#setPublicKeys', () => {
    it('should set public keys', () => {
      const publicKeys = [new IdentityPublicKey(), new IdentityPublicKey()];

      stateTransition.setPublicKeys(publicKeys);

      expect(stateTransition.publicKeys).to.have.deep.members(publicKeys);
    });
  });

  describe.skip('#getPublicKeys', () => {
    it('should return set public keys', () => {
      expect(stateTransition.getPublicKeys()).to.deep.equal(
        rawStateTransition.publicKeys.map((rawPublicKey) => new IdentityPublicKey(rawPublicKey)),
      );
    });
  });

  describe.skip('#addPublicKeys', () => {
    it('should add more public keys', () => {
      const publicKeys = [new IdentityPublicKey(), new IdentityPublicKey()];

      stateTransition.publicKeys = [];
      stateTransition.addPublicKeys(publicKeys);
      expect(stateTransition.getPublicKeys()).to.have.deep.members(publicKeys);
    });
  });

  describe.skip('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId()).to.deep.equal(
        stateTransition.getAssetLockProof().createIdentifier(),
      );
    });
  });

  describe.skip('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId()).to.equal(
        stateTransition.getIdentityId(),
      );
    });
  });

  describe.skip('#toObject', () => {
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

  describe.skip('#toJSON', () => {
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
