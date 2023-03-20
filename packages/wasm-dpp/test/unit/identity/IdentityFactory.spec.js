const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const getChainAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');
const { default: loadWasmDpp } = require('../../../dist');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');

describe('IdentityFactory', () => {
  let factory;
  let identity;
  let instantAssetLockProof;
  let chainAssetLockProof;
  let fakeTime;

  let Identity;
  let IdentityFactory;
  let IdentityValidator;
  let InstantAssetLockProof;
  let IdentityCreateTransition;
  let IdentityTopUpTransition;
  let IdentityUpdateTransition;
  let IdentityPublicKeyCreateTransition;
  let InvalidIdentityError;
  let SerializedObjectParsingError;
  let UnsupportedProtocolVersionError;
  let ChainAssetLockProof;

  before(async () => {
    ({
      Identity, IdentityFactory, IdentityValidator,
      InstantAssetLockProof, ChainAssetLockProof, IdentityUpdateTransition,
      IdentityCreateTransition, IdentityTopUpTransition, IdentityPublicKeyCreateTransition,
      InvalidIdentityError, UnsupportedProtocolVersionError, SerializedObjectParsingError,
    } = await loadWasmDpp());
  });

  beforeEach(async function () {
    const instantAssetLockProofJS = getInstantAssetLockProofFixture();
    const chainAssetLockProofJS = getChainAssetLockProofFixture();
    instantAssetLockProof = new InstantAssetLockProof(instantAssetLockProofJS.toObject());
    chainAssetLockProof = new ChainAssetLockProof(chainAssetLockProofJS.toObject());

    const blsAdapter = await getBlsAdapterMock();

    const identityValidator = new IdentityValidator(blsAdapter);

    factory = new IdentityFactory(
      1,
      identityValidator,
    );

    const identityObject = getIdentityFixture().toObject();
    identityObject.id = instantAssetLockProof.createIdentifier();

    identity = new Identity(identityObject);
    identity.setAssetLockProof(instantAssetLockProof);
    identity.setBalance(0);

    fakeTime = this.sinonSandbox.useFakeTimers(new Date());
  });

  afterEach(() => {
    fakeTime.reset();
  });

  describe('#create', () => {
    it('should create Identity from asset lock transaction, output index, proof and public keys', () => {
      const publicKeys = identity
        .getPublicKeys()
        .map((identityPublicKey) => ({
          ...identityPublicKey.toObject(),
          readonly: true,
        }));

      const result = factory.create(
        instantAssetLockProof,
        publicKeys,
      );

      expect(result).to.be.an.instanceOf(Identity);
      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#createFromObject', () => {
    it('should skip validation if options is set', () => {
      const identityObject = identity.toObject();
      identityObject.protocolVersion = 100;
      const result = factory.createFromObject(identityObject, { skipValidation: true });
      expect(result).to.exist();
    });

    it('should throw an error if validation have failed', () => {
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

    it('should throw InvalidIdentityError if the decoding fails with consensus error', () => {
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

    it('should throw an error if decoding fails with any other error', () => {
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
      const stateTransition = factory.createIdentityCreateTransition(identity);

      expect(stateTransition).to.be.instanceOf(IdentityCreateTransition);
      const keysToExpect = stateTransition.getPublicKeys()
        .map((key) => {
          const keyObject = key.toObject();
          delete keyObject.signature;
          return keyObject;
        });

      expect(keysToExpect)
        .to.deep.equal(identity.getPublicKeys().map((key) => key.toObject()));
      expect(stateTransition.getAssetLockProof().toObject())
        .to.deep.equal(instantAssetLockProof.toObject());
    });
  });

  describe('createChainAssetLockProof', () => {
    it('should create IdentityCreateTransition from Identity model', () => {
      const identityObject = getIdentityFixture().toObject();
      identityObject.id = chainAssetLockProof.createIdentifier();
      identity = new Identity(identityObject);
      identity.setAssetLockProof(chainAssetLockProof);
      identity.setBalance(0);

      const stateTransition = factory.createIdentityCreateTransition(identity);

      expect(stateTransition).to.be.instanceOf(IdentityCreateTransition);
      const keysToExpect = stateTransition.getPublicKeys()
        .map((key) => {
          const keyObject = key.toObject();
          delete keyObject.signature;
          return keyObject;
        });

      expect(keysToExpect)
        .to.deep.equal(identity.getPublicKeys().map((key) => key.toObject()));

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
      const addPublicKeys = [new IdentityPublicKeyCreateTransition({
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
        signature: Buffer.alloc(0),
      })];

      const stateTransition = factory
        .createIdentityUpdateTransition(
          identity,
          {
            add: addPublicKeys,
            disable: disablePublicKeys,
          },
        );

      expect(stateTransition).to.be.instanceOf(IdentityUpdateTransition);
      expect(stateTransition.getIdentityId().toBuffer()).to.deep.equal(identity.getId().toBuffer());
      expect(stateTransition.getRevision()).to.deep.equal(revision);
      expect(stateTransition.getPublicKeysToAdd().map((key) => key.toObject()))
        .to.deep.equal(addPublicKeys.map((key) => key.toObject()));
      expect(stateTransition.getPublicKeyIdsToDisable()).to.deep.equal([0]);
      expect(stateTransition.getPublicKeysDisabledAt()).to.deep.equal(new Date());
    });
  });
});
