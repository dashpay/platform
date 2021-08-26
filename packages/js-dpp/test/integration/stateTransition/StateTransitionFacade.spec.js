const { PrivateKey } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getIdentityCreateTransitionFixture = require('../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const DataContractFactory = require('../../../lib/dataContract/DataContractFactory');
const DocumentFactory = require('../../../lib/document/DocumentFactory');

const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');
const createDPPMock = require('../../../lib/test/mocks/createDPPMock');
const ConsensusError = require('../../../lib/errors/consensus/ConsensusError');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractCreateTransition;
  let documentsBatchTransition;
  let stateRepositoryMock;
  let dataContract;
  let identityPublicKey;

  beforeEach(async function beforeEach() {
    const privateKeyModel = new PrivateKey();
    const privateKey = privateKeyModel.toBuffer();
    const publicKey = privateKeyModel.toPublicKey().toBuffer();
    const publicKeyId = 1;

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey);

    dataContract = getDataContractFixture();

    const dataContractFactory = new DataContractFactory(createDPPMock(), undefined);

    dataContractCreateTransition = dataContractFactory.createStateTransition(dataContract);
    dataContractCreateTransition.sign(identityPublicKey, privateKey);

    const documentFactory = new DocumentFactory(createDPPMock(), undefined, undefined);

    documentsBatchTransition = documentFactory.createStateTransition({
      create: getDocumentsFixture(dataContract),
    });
    documentsBatchTransition.sign(identityPublicKey, privateKey);

    const getPublicKeyById = this.sinonSandbox.stub().returns(identityPublicKey);
    const getBalance = this.sinonSandbox.stub().returns(10000);

    const identity = {
      getPublicKeyById,
      type: 2,
      getBalance,
    };

    const timeInSeconds = Math.ceil(new Date().getTime() / 1000);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);
    stateRepositoryMock.fetchLatestPlatformBlockHeader.resolves({
      time: {
        seconds: timeInSeconds,
      },
    });

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
    });

    await dpp.initialize();
  });

  describe('createFromObject', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      try {
        await dpp.stateTransition.createFromObject(
          dataContractCreateTransition.toObject(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should skip checking for state repository if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      await dpp.stateTransition.createFromObject(
        dataContractCreateTransition.toObject(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from plain object', async () => {
      const result = await dpp.stateTransition.createFromObject(
        dataContractCreateTransition.toObject(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(dataContractCreateTransition.toObject());
    });
  });

  describe('createFromBuffer', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      try {
        await dpp.stateTransition.createFromBuffer(
          dataContractCreateTransition.toBuffer(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should skip checking for state repository if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      await dpp.stateTransition.createFromBuffer(
        dataContractCreateTransition.toBuffer(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from string', async () => {
      const result = await dpp.stateTransition.createFromBuffer(
        dataContractCreateTransition.toBuffer(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toObject()).to.deep.equal(dataContractCreateTransition.toObject());
    });
  });

  describe('validate', () => {
    let validateBasicSpy;
    let validateSignatureSpy;
    let validateFeeSpy;
    let validateStateSpy;

    beforeEach(function beforeEach() {
      validateBasicSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateBasic',
      );

      validateSignatureSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateSignature',
      );

      validateFeeSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateFee',
      );

      validateStateSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateState',
      );
    });

    it('should return invalid result if State Transition structure is invalid', async () => {
      const rawStateTransition = dataContractCreateTransition.toObject();
      delete rawStateTransition.protocolVersion;

      const result = await dpp.stateTransition.validate(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateBasicSpy).to.be.calledOnceWithExactly(rawStateTransition);
      expect(validateSignatureSpy).to.not.be.called();
      expect(validateFeeSpy).to.not.be.called();
      expect(validateStateSpy).to.not.be.called();
    });

    it('should return invalid result if State Transition signature is invalid', async () => {
      dataContractCreateTransition.signature = Buffer.alloc(65).fill(1);

      const result = await dpp.stateTransition.validate(dataContractCreateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateBasicSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateSignatureSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateFeeSpy).to.not.be.called();
      expect(validateStateSpy).to.not.be.called();
    });

    it('should return invalid result if not enough balance to pay fee for State Transition', async () => {
      const consensusError = new ConsensusError('error');

      dpp.stateTransition.validateStateTransitionFee = () => new ValidationResult([consensusError]);

      const result = await dpp.stateTransition.validate(dataContractCreateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateBasicSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateSignatureSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateFeeSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateStateSpy).to.not.be.called();
    });

    it('should return invalid result if State Transition is invalid against state', async () => {
      const consensusError = new ConsensusError('error');

      dpp.stateTransition.validateStateTransitionState = () => (
        new ValidationResult([consensusError])
      );

      const result = await dpp.stateTransition.validate(dataContractCreateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateBasicSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateSignatureSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateFeeSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateStateSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
    });

    it('should validate DataContractCreateTransition', async () => {
      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateBasicSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateSignatureSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateFeeSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
      expect(validateStateSpy).to.be.calledOnceWithExactly(dataContractCreateTransition);
    });

    it('should validate DocumentsBatchTransition', async function it() {
      stateRepositoryMock.fetchDocuments.resolves([]);

      stateRepositoryMock.fetchDataContract.resolves(dataContract);
      stateRepositoryMock.fetchIdentity.resolves({
        getPublicKeyById: this.sinonSandbox.stub().returns(identityPublicKey),
        getBalance: this.sinonSandbox.stub().returns(10000),
      });

      const result = await dpp.stateTransition.validate(
        documentsBatchTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateBasicSpy).to.be.calledOnceWithExactly(documentsBatchTransition);
      expect(validateSignatureSpy).to.be.calledOnceWithExactly(documentsBatchTransition);
      expect(validateFeeSpy).to.be.calledOnceWithExactly(documentsBatchTransition);
      expect(validateStateSpy).to.be.calledOnceWithExactly(documentsBatchTransition);
    });
  });

  describe('validateBasic', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      try {
        await dpp.stateTransition.validateBasic(
          dataContractCreateTransition.toObject(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateBasic(
        dataContractCreateTransition.toObject(),
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateSignature', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      try {
        await dpp.stateTransition.validateSignature(
          dataContractCreateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

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

      const identityCreateTransition = getIdentityCreateTransitionFixture(oneTimePrivateKey);

      identityCreateTransition.signByPrivateKey(oneTimePrivateKey);

      const result = await dpp.stateTransition.validateSignature(
        identityCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateFee', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      try {
        await dpp.stateTransition.validateFee(
          dataContractCreateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateFee(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateState', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();
      await dpp.initialize();

      try {
        await dpp.stateTransition.validateState(
          dataContractCreateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateState(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });
});
