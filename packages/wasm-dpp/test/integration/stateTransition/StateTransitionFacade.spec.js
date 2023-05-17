const { PrivateKey } = require('@dashevo/dashcore-lib');
const crypto = require('crypto');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const getIdentityFixture = require('../../../lib/test/fixtures/getIdentityFixture');
const getIdentityCreateTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreateTransitionFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../dist');
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
  let executionContext;

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
  let UnsupportedProtocolVersionError;
  let InvalidStateTransitionSignatureError;
  let DataContractAlreadyPresentError;
  let BalanceIsNotEnoughError;
  let StateTransitionExecutionContext;

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
      UnsupportedProtocolVersionError,
      InvalidStateTransitionSignatureError,
      DataContractAlreadyPresentError,
      BalanceIsNotEnoughError,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    this.timeout(20000);
    const privateKeyModel = new PrivateKey();
    const privateKey = privateKeyModel.toBuffer();
    const publicKey = privateKeyModel.toPublicKey().toBuffer();
    const publicKeyId = 1;

    executionContext = new StateTransitionExecutionContext();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

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

    const identityObject = (await getIdentityFixture()).toObject();
    identityObject.id = await generateRandomIdentifierAsync();
    identityObject.balance = 10000000;
    identity = new Identity(identityObject);
    identity.setPublicKeys([identityPublicKey]);

    const blockTime = Date.now();

    stateRepositoryMock.fetchIdentity.resolves(identity);
    stateRepositoryMock.fetchIdentityBalance.resolves(identity.getBalance());
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
    it('should create Data Contract State Transition from buffer', async () => {
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

      const result = await dpp.stateTransition.validate(rawStateTransition, executionContext);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(UnsupportedProtocolVersionError);
    });

    it('should return invalid result if State Transition signature is invalid', async () => {
      const rawStateTransition = dataContractCreateTransition.toObject();
      rawStateTransition.signature = Buffer.alloc(65).fill(1);

      const result = await dpp.stateTransition.validate(rawStateTransition, executionContext);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(InvalidStateTransitionSignatureError);
    });

    it('should return invalid result if not enough balance to pay fee for State Transition', async () => {
      identity.setBalance(0);
      stateRepositoryMock.fetchIdentityBalance.resolves(0);
      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition, executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(BalanceIsNotEnoughError);
    });

    it('should return invalid result if State Transition is invalid against state', async () => {
      stateRepositoryMock.fetchDataContract.resolves(dataContract);

      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition, executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
      const errors = result.getErrors();
      expect(errors).to.have.lengthOf(1);
      expect(errors[0]).to.be.instanceOf(DataContractAlreadyPresentError);
    });

    it('should validate DataContractCreateTransition', async () => {
      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition,
        executionContext,
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
        executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateBasic', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateBasic(
        dataContractCreateTransition.toObject(), executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateSignature', () => {
    it('should validate identity signed State Transition', async () => {
      const result = await dpp.stateTransition.validateSignature(
        dataContractCreateTransition, executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should validate key signed State Transition', async () => {
      const oneTimePrivateKey = new PrivateKey(
        'af432c476f65211f45f48f1d42c9c0b497e56696aa1736b40544ef1a496af837',
      );

      const identityCreateTransition = await getIdentityCreateTransitionFixture(oneTimePrivateKey);

      await identityCreateTransition.signByPrivateKey(
        oneTimePrivateKey.toBuffer(),
        IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      );

      const result = await dpp.stateTransition.validateSignature(
        identityCreateTransition, executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateFee', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateFee(
        dataContractCreateTransition,
        executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateState', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateState(
        dataContractCreateTransition,
        executionContext,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });
});
