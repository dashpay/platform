const { PrivateKey } = require('@dashevo/dashcore-lib');
const crypto = require('crypto');

const getIdentityCreateTransitionFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityCreateTransitionFixture');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const getIdentityFixture = require('@dashevo/dpp/lib/test/fixtures/getIdentityFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../..');
const getBlsAdapterMock = require('../../../lib/test/mocks/getBlsAdapterMock');
const generateRandomIdentifierAsync = require('../../../lib/test/utils/generateRandomIdentifierAsync');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractCreateTransition;
  let documentsBatchTransition;
  let stateRepositoryMock;
  let dataContract;
  let identityPublicKey;
  let identity;

  let DashPlatformProtocol;
  let DataContractFactory;
  let DataContractValidator;
  let DataContractCreateTransition;
  let Identity;
  let ValidationResult;
  let IdentityPublicKey;
  let DocumentFactory;
  let DocumentValidator;
  let ProtocolVersionValidator;
  let IdentityCreateTransition;
  let UnsupportedProtocolVersionError;
  let InvalidStateTransitionSignatureError;
  let DataContractAlreadyPresentError;
  let BalanceIsNotEnoughError;

  before(async () => {
    ({
      DashPlatformProtocol,
      ValidationResult,
      DataContractCreateTransition,
      Identity,
      DataContractValidator,
      DataContractFactory,
      IdentityPublicKey,
      DocumentFactory,
      DocumentValidator,
      ProtocolVersionValidator,
      IdentityCreateTransition,
      UnsupportedProtocolVersionError,
      InvalidStateTransitionSignatureError,
      DataContractAlreadyPresentError,
      BalanceIsNotEnoughError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const privateKeyModel = new PrivateKey();
    const privateKey = privateKeyModel.toBuffer();
    const publicKey = privateKeyModel.toPublicKey().toBuffer();
    const publicKeyId = 1;

    identityPublicKey = new IdentityPublicKey({
      id: publicKeyId,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: publicKey,
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
      readOnly: false,
    });

    dataContract = await getDataContractFixture();

    const dataContractValidator = new DataContractValidator();
    const dataContractFactory = new DataContractFactory(1, dataContractValidator);

    dataContractCreateTransition = await dataContractFactory.createDataContractCreateTransition(
      dataContract,
    );
    await dataContractCreateTransition.sign(
      identityPublicKey,
      privateKey,
    );

    const documentValidator = new DocumentValidator(new ProtocolVersionValidator());
    const documentFactory = new DocumentFactory(1, documentValidator, stateRepositoryMock);

    documentsBatchTransition = documentFactory.createStateTransition({
      create: await getDocumentsFixture(dataContract),
    });
    await documentsBatchTransition.sign(identityPublicKey, privateKey);

    const identityObjectJS = getIdentityFixture().toObject();
    identityObjectJS.id = await generateRandomIdentifierAsync();
    identityObjectJS.balance = 10000000;
    identity = new Identity(identityObjectJS);
    identity.setPublicKeys([identityPublicKey]);

    const blockTime = Date.now();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);
    stateRepositoryMock.fetchLatestPlatformBlockTime.resolves(blockTime);
    stateRepositoryMock.fetchDataContract.resolves(null);

    const blsAdapter = await getBlsAdapterMock();

    dpp = new DashPlatformProtocol(
      blsAdapter,
      stateRepositoryMock,
      { generate: () => crypto.randomBytes(32) },
      1,
    );
  });

  describe('createFromObject', () => {
    it('should create State Transition from plain object', async () => {
      const object = dataContractCreateTransition.toObject();
      const result = await dpp.stateTransition.createFromObject(
        object,
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(dataContractCreateTransition.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should create State Transition from string', async () => {
      const result = await dpp.stateTransition.createFromBuffer(
        dataContractCreateTransition.toBuffer(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(dataContractCreateTransition.toObject());
    });
  });

  describe('validate', () => {
    it('should return invalid result if State Transition structure is invalid', async () => {
      const rawStateTransition = dataContractCreateTransition.toObject();
      rawStateTransition.protocolVersion = 100;

      const result = await dpp.stateTransition.validate(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(UnsupportedProtocolVersionError);
    });

    it('should return invalid result if State Transition signature is invalid', async () => {
      const rawStateTransition = dataContractCreateTransition.toObject();
      rawStateTransition.signature = Buffer.alloc(65).fill(1);

      const result = await dpp.stateTransition.validate(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(InvalidStateTransitionSignatureError);
    });

    it('should return invalid result if not enough balance to pay fee for State Transition', async () => {
      identity.setBalance(0);
      const result = await dpp.stateTransition.validate(dataContractCreateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(BalanceIsNotEnoughError);
    });

    it('should return invalid result if State Transition is invalid against state', async () => {
      stateRepositoryMock.fetchDataContract.resolves(dataContract);

      const result = await dpp.stateTransition.validate(dataContractCreateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(DataContractAlreadyPresentError);
    });

    it('should validate DataContractCreateTransition', async () => {
      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should validate DocumentsBatchTransition', async () => {
      stateRepositoryMock.fetchDocuments.resolves([]);
      stateRepositoryMock.fetchExtendedDocuments.resolves([]);

      stateRepositoryMock.fetchDataContract.resolves(dataContract);
      const result = await dpp.stateTransition.validate(
        documentsBatchTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateBasic', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateBasic(
        dataContractCreateTransition.toObject(),
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateSignature', () => {
    it('should validate identity signed State Transition', async () => {
      const result = await dpp.stateTransition.validateSignature(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should validate key signed State Transition', async () => {
      const oneTimePrivateKey = new PrivateKey(
        'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837',
      );

      const identityCreateTransitionJS = getIdentityCreateTransitionFixture(oneTimePrivateKey);
      const identityCreateTransition = new IdentityCreateTransition(
        identityCreateTransitionJS.toObject(),
      );

      await identityCreateTransition.signByPrivateKey(
        oneTimePrivateKey.toBuffer(),
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      const result = await dpp.stateTransition.validateSignature(
        identityCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateFee', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateFee(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateState', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateState(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });
});
