const getIdentityTopUpTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityTopUpTransitionFixture');
const getChainAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const protocolVersion = require('@dashevo/dpp/lib/version/protocolVersion');
const { default: loadWasmDpp } = require('../../../../../dist');

describe('IdentityTopUpTransition', () => {
  let rawStateTransition;
  let stateTransition;

  let IdentityTopUpTransition;
  let InstantAssetLockProof;
  let Identifier;

  before(async () => {
    ({
      IdentityTopUpTransition, InstantAssetLockProof, Identifier,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    stateTransition = new IdentityTopUpTransition(
      getIdentityTopUpTransitionFixture().toObject(),
    );
    rawStateTransition = stateTransition.toObject();
  });

  describe('#constructor', () => {
    it('should create an instance with specified data from specified raw transition', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.be.deep.equal(
        rawStateTransition.assetLockProof,
      );
      expect(stateTransition.getIdentityId().toBuffer()).to.be.deep.equal(
        rawStateTransition.identityId,
      );
    });

    it('should create instance with chain asset lock proof', () => {
      rawStateTransition.assetLockProof = getChainAssetLockProofFixture().toObject();
      stateTransition = new IdentityTopUpTransition(rawStateTransition);
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(rawStateTransition.assetLockProof);
    });
  });

  describe('#getType', () => {
    it('should return IDENTITY_TOP_UP type', () => {
      expect(stateTransition.getType()).to.equal(stateTransitionTypes.IDENTITY_TOP_UP);
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
  });

  describe('#getAssetLockProof', () => {
    it('should return currently set asset lock proof', () => {
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        rawStateTransition.assetLockProof,
      );
    });
  });

  describe('#getIdentityId', () => {
    it('should return identity id', () => {
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#getOwnerId', () => {
    it('should return owner id', () => {
      expect(stateTransition.getOwnerId().toBuffer()).to.deep.equal(
        rawStateTransition.identityId,
      );
    });
  });

  describe('#toObject', () => {
    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject();

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        assetLockProof: rawStateTransition.assetLockProof,
        identityId: rawStateTransition.identityId,
        signature: undefined,
      });
    });

    it('should return raw state transition', () => {
      rawStateTransition = stateTransition.toObject({ skipSignature: true });

      expect(rawStateTransition).to.deep.equal({
        protocolVersion: protocolVersion.latestVersion,
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
        protocolVersion: protocolVersion.latestVersion,
        type: stateTransitionTypes.IDENTITY_TOP_UP,
        assetLockProof: stateTransition.getAssetLockProof().toJSON(),
        identityId: new Identifier(rawStateTransition.identityId).toString(),
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
