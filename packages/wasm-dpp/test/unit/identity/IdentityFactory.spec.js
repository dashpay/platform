const getInstantAssetLockProofFixture = require('../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');
const getChainAssetLockProofFixture = require('../../../lib/test/fixtures/getChainAssetLockProofFixture');
const getIdentityCreditWithdrawalTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreditWithdrawalTransitionFixture');

const {
  Identity, IdentityFactory,
  InstantAssetLockProof, ChainAssetLockProof, IdentityUpdateTransition,
  IdentityCreateTransition, IdentityTopUpTransition, IdentityPublicKeyWithWitness,
  InvalidIdentityError, UnsupportedProtocolVersionError,
} = require('../../..');
const { SerializedObjectParsingError } = require('../../..');

describe('IdentityFactory', () => {
  let factory;
  let identity;
  let instantAssetLockProof;
  let chainAssetLockProof;
  let fakeTime;

  beforeEach(async function beforeEach() {
    instantAssetLockProof = await getInstantAssetLockProofFixture();
    chainAssetLockProof = new ChainAssetLockProof(getChainAssetLockProofFixture().toObject());

    // const blsAdapter = await getBlsAdapterMock();

    // const identityValidator = new IdentityValidator(blsAdapter);

    factory = new IdentityFactory(3);

    identity = await getIdentityFixture(instantAssetLockProof.createIdentifier());
    identity.setBalance(0);

    fakeTime = this.sinon.useFakeTimers(new Date());
  });

  afterEach(() => {
    fakeTime.reset();
  });

  describe('#create', () => {
    it('should create Identity from asset lock transaction, output index, proof and public keys', () => {
      const publicKeys = identity.getPublicKeys();
      publicKeys.forEach((key) => key.setReadOnly(true));
      identity.setPublicKeys(publicKeys);

      const result = factory.create(
        instantAssetLockProof.createIdentifier(),
        publicKeys,
      );

      expect(result).to.be.an.instanceOf(Identity);
      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  // TODO(versioning): re-check. Not used anymore
  describe.skip('#createFromObject', () => {
    it('should skip validation if options is set', () => {
      const identityObject = identity.toObject();
      identityObject.protocolVersion = 100;
      const result = factory.createFromObject(identityObject, { skipValidation: true });
      expect(result).to.exist();
    });

    // TODO(versioning): restore
    it.skip('should throw an error if validation have failed', () => {
      const identityObject = identity.toObject();
      identityObject.protocolVersion = 3;

      try {
        factory.createFromObject(identityObject);

        expect.fail('error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidIdentityError);
        expect(e.getErrors()[0]).to.be.instanceOf(UnsupportedProtocolVersionError);
        expect(e.getRawIdentity()).to.deep.equal(identityObject);
      }
    });

    it('should create an identity if validation passed', () => {
      const result = factory.createFromObject(identity.toObject());

      expect(result).to.be.an.instanceOf(Identity);
      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#createFromBuffer', () => {
    let serializedIdentity;
    let rawIdentity;

    beforeEach(() => {
      serializedIdentity = identity.toBuffer();
      rawIdentity = identity.toObject();
    });

    it('should return new Identity from serialized one', () => {
      const result = factory.createFromBuffer(serializedIdentity);
      expect(result.toObject()).to.deep.equal(rawIdentity);
    });

    // TODO(versioning): restore
    it.skip('should throw InvalidIdentityError if the decoding fails with consensus error', () => {
      try {
        // Mess up protocol version
        serializedIdentity[0] = 3;
        factory.createFromBuffer(serializedIdentity);

        expect.fail('should throw InvalidIdentityError');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidIdentityError);

        const [innerError] = e.getErrors();
        expect(innerError).to.be.instanceOf(UnsupportedProtocolVersionError);
      }
    });

    // TODO(versioning): restore
    it.skip('should throw an error if decoding fails with any other error', () => {
      try {
        serializedIdentity = serializedIdentity.slice(4);
        factory.createFromBuffer(serializedIdentity);

        expect.fail('should throw an error');
      } catch (e) {
        expect(e).to.be.instanceOf(SerializedObjectParsingError);
      }
    });
  });

  describe('#createInstantAssetLockProof', () => {
    it('should create instant asset lock proof from InstantLock', () => {
      const instantLock = instantAssetLockProof.getInstantLock();
      const assetLockTransaction = instantAssetLockProof.getTransaction();
      const outputIndex = instantAssetLockProof.getOutputIndex();

      const result = factory.createInstantAssetLockProof(
        instantLock,
        assetLockTransaction,
        outputIndex,
      );

      expect(result).to.be.instanceOf(InstantAssetLockProof);
      expect(result.getInstantLock()).to.deep.equal(instantLock);
    });
  });

  describe('#createIdentityCreateTransition', () => {
    it('should create IdentityCreateTransition from Identity model', () => {
      const stateTransition = factory.createIdentityCreateTransition(
        identity,
        instantAssetLockProof,
        1,
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

      expect(keysToExpect)
        .to.deep.equal(actualKeys);
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(instantAssetLockProof.toObject());
    });
  });

  describe('createChainAssetLockProof', () => {
    it('should create IdentityCreateTransition from Identity model', async () => {
      identity = await getIdentityFixture(chainAssetLockProof.createIdentifier());
      identity.setBalance(0);

      const stateTransition = factory.createIdentityCreateTransition(
        identity,
        chainAssetLockProof,
        1,
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

      expect(keysToExpect)
        .to.deep.equal(actualKeys);

      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(chainAssetLockProof.toObject());
    });
  });

  describe('#createIdentityTopUpTransition', () => {
    it('should create IdentityTopUpTransition from identity id and outpoint', () => {
      const stateTransition = factory
        .createIdentityTopUpTransition(
          identity.getId(),
          instantAssetLockProof,
        );

      expect(stateTransition).to.be.instanceOf(IdentityTopUpTransition);
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(identity.getId().toBuffer());
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(instantAssetLockProof.toObject());
    });
  });

  describe('createIdentityUpdateTransition', () => {
    it('should create IdentityUpdateTransition', () => {
      const revision = 1;
      const disablePublicKeys = [identity.getPublicKeyById(0)];
      const key = new IdentityPublicKeyWithWitness(1);
      key.setData(Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'));

      const addPublicKeys = [key];

      const stateTransition = factory
        .createIdentityUpdateTransition(
          identity,
          // eslint-disable-next-line
          BigInt(1),
          {
            add: addPublicKeys,
            disable: disablePublicKeys,
          },
        );

      expect(stateTransition).to.be.instanceOf(IdentityUpdateTransition);
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(identity.getId().toBuffer());
      expect(stateTransition.getRevision()).to.deep.equal(revision);
      expect(stateTransition.getPublicKeysToAdd().map((k) => k.toObject()))
        .to.deep.equal(addPublicKeys.map((k) => k.toObject()));
      expect(stateTransition.getPublicKeyIdsToDisable()).to.deep.equal([0]);
    });
  });

  describe('createIdentityCreditWithdrawalTransition', () => {
    it('should create IdentityCreditWithdrawalTransition', () => {
      const stateTransitionFixture = getIdentityCreditWithdrawalTransitionFixture();
      const stateTransition = factory
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
