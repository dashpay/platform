const { PublicKey } = require('@dashevo/dashcore-lib');
const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const Identity = require('../../../lib/identity/Identity');
const IdentityCreateTransition = require('../../../lib/identity/stateTransition/IdentityCreateTransition/IdentityCreateTransition');
const IdentityTopUpTransition = require('../../../lib/identity/stateTransition/IdentityTopUpTransition/IdentityTopUpTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');
const InstantAssetLockProof = require('../../../lib/identity/stateTransition/assetLockProof/instant/InstantAssetLockProof');
const getInstantAssetLockProofFixture = require('../../../lib/test/fixtures/getInstantAssetLockProofFixture');
const getChainAssetLockProofFixture = require('../../../lib/test/fixtures/getChainAssetLockProofFixture');
const ChainAssetLockProof = require('../../../lib/identity/stateTransition/assetLockProof/chain/ChainAssetLockProof');
const IdentityUpdateTransition = require('../../../lib/identity/stateTransition/IdentityUpdateTransition/IdentityUpdateTransition');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

describe('IdentityFacade', () => {
  let dpp;
  let identity;
  let stateRepositoryMock;
  let instantAssetLockProof;
  let chainAssetLockProof;

  beforeEach(async function beforeEach() {
    const rawTransaction = '030000000137feb5676d0851337ea3c9a992496aab7a0b3eee60aeeb9774000b7f4bababa5000000006b483045022100d91557de37645c641b948c6cd03b4ae3791a63a650db3e2fee1dcf5185d1b10402200e8bd410bf516ca61715867666d31e44495428ce5c1090bf2294a829ebcfa4ef0121025c3cc7fbfc52f710c941497fd01876c189171ea227458f501afcb38a297d65b4ffffffff021027000000000000166a14152073ca2300a86b510fa2f123d3ea7da3af68dcf77cb0090a0000001976a914152073ca2300a86b510fa2f123d3ea7da3af68dc88ac00000000';

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchTransaction.resolves(rawTransaction);

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
    });
    await dpp.initialize();

    chainAssetLockProof = getChainAssetLockProofFixture();
    instantAssetLockProof = getInstantAssetLockProofFixture();
    identity = getIdentityFixture();
    identity.id = instantAssetLockProof.createIdentifier();
    identity.setAssetLockProof(instantAssetLockProof);
    identity.setBalance(0);
  });

  describe('#create', () => {
    it('should create Identity', () => {
      const publicKeys = identity.getPublicKeys()
        .map((identityPublicKey) => ({
          ...identityPublicKey.toObject(),
          key: new PublicKey(identityPublicKey.getData()),
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
      expect(result.getTransaction().toObject()).to.deep.equal(assetLockTransaction.toObject());
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
      expect(stateTransition.getPublicKeys()).to.deep.equal(identity.getPublicKeys());
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
      expect(stateTransition.getIdentityId()).to.be.deep.equal(identity.getId());
      expect(stateTransition.getAssetLockProof().toObject()).to.deep.equal(
        instantAssetLockProof.toObject(),
      );
    });
  });

  describe('#createIdentityUpdateTransition', () => {
    it('should create IdentityUpdateTransition from identity id and public keys', () => {
      const publicKeys = {
        add: [new IdentityPublicKey({
          id: 3,
          type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
          data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
          purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
          securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
          readOnly: false,
        })],
      };

      const stateTransition = dpp.identity
        .createIdentityUpdateTransition(
          identity,
          publicKeys,
        );

      expect(stateTransition).to.be.instanceOf(IdentityUpdateTransition);
      expect(stateTransition.getIdentityId()).to.be.deep.equal(identity.getId());
      expect(stateTransition.getRevision()).to.equal(
        identity.getRevision() + 1,
      );
      expect(
        stateTransition.getPublicKeysToAdd().map((pk) => pk.toObject()),
      ).to.deep.equal(publicKeys.add);
      expect(stateTransition.getPublicKeyIdsToDisable()).to.equal(undefined);
      expect(stateTransition.getPublicKeysDisabledAt()).to.equal(undefined);
    });
  });
});
