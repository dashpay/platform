const crypto = require('crypto');
const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');

const getInstantAssetLockProofFixture = require('../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const getChainAssetLockProofFixture = require('../../../lib/test/fixtures/getChainAssetLockProofFixture');

const {
  default: loadWasmDpp,
  Identity, InstantAssetLockProof, ChainAssetLockProof, IdentityUpdateTransition,
  IdentityCreateTransition, IdentityTopUpTransition, IdentityPublicKeyWithWitness,
  DashPlatformProtocol, ValidationResult,
} = require('../../..');
const getIdentityCreditWithdrawalTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreditWithdrawalTransitionFixture');

describe('IdentityFacade', () => {
  let dpp;

  let identity;
  let instantAssetLockProof;
  let chainAssetLockProof;

  before(loadWasmDpp);

  beforeEach(async () => {
    dpp = new DashPlatformProtocol(
      { generate: () => crypto.randomBytes(32) },
      3,
    );

    const chainAssetLockProofJS = getChainAssetLockProofFixture();
    instantAssetLockProof = await getInstantAssetLockProofFixture();
    chainAssetLockProof = new ChainAssetLockProof(chainAssetLockProofJS.toObject());

    identity = await getIdentityFixture(instantAssetLockProof.createIdentifier());
    identity.setBalance(0);
  });

  describe('#create', () => {
    it('should create Identity', () => {
      const publicKeys = identity.getPublicKeys();

      const result = dpp.identity.create(
        instantAssetLockProof.createIdentifier(),
        publicKeys,
      );

      expect(result).to.be.an.instanceOf(Identity);
      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  // TODO(versioning): obsolete?
  describe.skip('#createFromObject', () => {
    it('should create Identity from plain object', () => {
      const result = dpp.identity.createFromObject(identity.toObject());

      expect(result).to.be.an.instanceOf(Identity);

      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#createFromBuffer', () => {
    it('should create Identity from a Buffer', () => {
      let result;
      try {
        result = dpp.identity.createFromBuffer(identity.toBuffer());
      } catch (e) {
        console.dir(e.getErrors()[0].toString());
      }

      expect(result).to.be.an.instanceOf(Identity);

      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  // TODO(versioning): restore
  describe.skip('#validate', () => {
    it('should validate Identity', async () => {
      const result = await dpp.identity.validate(identity);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('#createInstantAssetLockProof', () => {
    it('should create instant asset lock proof', () => {
      const instantLock = instantAssetLockProof.getInstantLock();
      const assetLockTransaction = instantAssetLockProof.getTransaction();
      const outputIndex = instantAssetLockProof.getOutputIndex();

      const result = dpp.identity.createInstantAssetLockProof(
        instantLock,
        assetLockTransaction,
        outputIndex,
      );

      expect(result).to.be.instanceOf(InstantAssetLockProof);
      expect(result.getInstantLock()).to.deep.equal(instantLock);
      expect(result.getTransaction()).to.deep.equal(assetLockTransaction);
      expect(result.getOutputIndex()).to.equal(outputIndex);
    });
  });

  describe('#createChainAssetLockProof', () => {
    it('should create chain asset lock proof', () => {
      const coreChainLockedHeight = chainAssetLockProof.getCoreChainLockedHeight();
      const outPoint = chainAssetLockProof.getOutPoint();

      const result = dpp.identity.createChainAssetLockProof(
        coreChainLockedHeight,
        outPoint,
      );

      expect(result).to.be.instanceOf(ChainAssetLockProof);
      expect(result.getCoreChainLockedHeight()).to.equal(coreChainLockedHeight);
      expect(result.getOutPoint()).to.deep.equal(outPoint);
    });
  });

  describe('#createIdentityCreateTransition', () => {
    it('should create IdentityCreateTransition from Identity model', () => {
      const stateTransition = dpp.identity.createIdentityCreateTransition(
        identity,
        instantAssetLockProof,
      );

      expect(stateTransition).to.be.instanceOf(IdentityCreateTransition);
      const keysToExpect = stateTransition.getPublicKeys()
        .map((key) => {
          const keyObject = key.toObject();
          delete keyObject.signature;
          return keyObject;
        });

      const actualKeys = identity.getPublicKeys().map((key) => {
        const keyObject = key.toObject();
        delete keyObject.disabledAt;
        return keyObject;
      });

      expect(keysToExpect).to.deep.equal(actualKeys);
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        instantAssetLockProof.toObject(),
      );
    });
  });

  describe('#createIdentityTopUpTransition', () => {
    it('should create IdentityTopUpTransition from identity id and outpoint', () => {
      const stateTransition = dpp.identity
        .createIdentityTopUpTransition(
          identity.getId(),
          instantAssetLockProof,
        );

      expect(stateTransition).to.be.instanceOf(IdentityTopUpTransition);
      expect(stateTransition.getIdentityId().toBuffer())
        .to.be.deep.equal(identity.getId().toBuffer());
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        instantAssetLockProof.toObject(),
      );
    });
  });

  describe('#createIdentityUpdateTransition', () => {
    it('should create IdentityUpdateTransition from identity id and public keys', () => {
      const key = new IdentityPublicKeyWithWitness(1);
      key.setData(Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'));

      const publicKeys = {
        add: [key],
      };

      const stateTransition = dpp.identity
        .createIdentityUpdateTransition(
          identity,
          // eslint-disable-next-line
          BigInt(1),
          publicKeys,
        );

      expect(stateTransition).to.be.instanceOf(IdentityUpdateTransition);
      expect(stateTransition.getIdentityId().toBuffer())
        .to.be.deep.equal(identity.getId().toBuffer());
      expect(stateTransition.getRevision()).to.equal(
        identity.getRevision() + 1,
      );
      expect(
        stateTransition.getPublicKeysToAdd().map((pk) => pk.toObject()),
      ).to.deep.equal(publicKeys.add.map((k) => k.toObject()));
      expect(stateTransition.getPublicKeyIdsToDisable()).to.deep.equal([]);
    });
  });

  describe('createIdentityCreditWithdrawalTransition', () => {
    it('should create IdentityCreditWithdrawalTransition', () => {
      const stateTransitionFixture = getIdentityCreditWithdrawalTransitionFixture();
      const stateTransition = dpp.identity
        .createIdentityCreditWithdrawalTransition(
          stateTransitionFixture.getIdentityId(),
          stateTransitionFixture.getAmount(),
          stateTransitionFixture.getCoreFeePerByte(),
          stateTransitionFixture.getPooling(),
          stateTransitionFixture.getOutputScript(),
          stateTransitionFixture.getNonce(),
        );

      expect(stateTransition.toObject())
        .to.deep.equal(
          stateTransitionFixture.toObject(),
        );
    });
  });
});
