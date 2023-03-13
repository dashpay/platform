const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');
const getInstantAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getInstantAssetLockProofFixture');
const getChainAssetLockProofFixture = require('@dashevo/dpp/lib/test/fixtures/getChainAssetLockProofFixture');
const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const { default: loadWasmDpp } = require('../../../dist');

describe('IdentityFacade', () => {
  let dpp;
  let stateRepositoryMock;

  let identity;
  let instantAssetLockProof;
  let chainAssetLockProof;

  let Identity;
  let InstantAssetLockProof;
  let IdentityCreateTransition;
  let IdentityTopUpTransition;
  let IdentityUpdateTransition;
  let IdentityPublicKeyCreateTransition;
  let ChainAssetLockProof;
  let DashPlatformProtocol;
  let ValidationResult;

  before(async () => {
    ({
      Identity, InstantAssetLockProof, ChainAssetLockProof, IdentityUpdateTransition,
      IdentityCreateTransition, IdentityTopUpTransition, IdentityPublicKeyCreateTransition,
      DashPlatformProtocol, ValidationResult,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchTransaction.resolves({
      data: Buffer.from(rawTransaction, 'hex'),
      height: 42,
    });

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
    });

    const chainAssetLockProofJS = getChainAssetLockProofFixture();
    const instantAssetLockProofJS = getInstantAssetLockProofFixture();
    instantAssetLockProof = new InstantAssetLockProof(instantAssetLockProofJS.toObject());
    chainAssetLockProof = new ChainAssetLockProof(chainAssetLockProofJS.toObject());

    const identityObject = getIdentityFixture().toObject();
    identityObject.id = instantAssetLockProof.createIdentifier();
    identity = new Identity(identityObject);
    identity.setAssetLockProof(instantAssetLockProof);
    identity.setBalance(0);
  });

  describe('#create', () => {
    it('should create Identity', () => {
      const publicKeys = identity.getPublicKeys()
        .map((identityPublicKey) => ({
          ...identityPublicKey.toObject(),
        }));

      const result = dpp.identity.create(
        instantAssetLockProof,
        publicKeys,
      );

      expect(result).to.be.an.instanceOf(Identity);
      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#createFromObject', () => {
    it('should create Identity from plain object', () => {
      const result = dpp.identity.createFromObject(identity.toObject());

      expect(result).to.be.an.instanceOf(Identity);

      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#createFromBuffer', () => {
    it('should create Identity from string', () => {
      const result = dpp.identity.createFromBuffer(identity.toBuffer());

      expect(result).to.be.an.instanceOf(Identity);

      expect(result.toObject()).to.deep.equal(identity.toObject());
    });
  });

  describe('#validate', () => {
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
      const stateTransition = dpp.identity.createIdentityCreateTransition(identity);

      expect(stateTransition).to.be.instanceOf(IdentityCreateTransition);
      const keysToExpect = stateTransition.getPublicKeys()
        .map((key) => {
          const keyObject = key.toObject();
          delete keyObject.signature;
          return keyObject;
        });

      expect(keysToExpect)
        .to.deep.equal(identity.getPublicKeys().map((key) => key.toObject()));
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
      const publicKeys = {
        add: [new IdentityPublicKeyCreateTransition({
          id: 3,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
          readOnly: false,
          signature: Buffer.alloc(0),
        })],
      };

      const stateTransition = dpp.identity
        .createIdentityUpdateTransition(
          identity,
          publicKeys,
        );

      expect(stateTransition).to.be.instanceOf(IdentityUpdateTransition);
      expect(stateTransition.getIdentityId().toBuffer())
        .to.be.deep.equal(identity.getId().toBuffer());
      expect(stateTransition.getRevision()).to.equal(
        identity.getRevision() + 1n,
      );
      expect(
        stateTransition.getPublicKeysToAdd().map((pk) => pk.toObject()),
      ).to.deep.equal(publicKeys.add.map((key) => key.toObject()));
      expect(stateTransition.getPublicKeyIdsToDisable()).to.deep.equal([]);
      expect(stateTransition.getPublicKeysDisabledAt()).to.equal(undefined);
    });
  });
});
