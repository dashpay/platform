const { PrivateKey } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractCreateTransition = require('../../../lib/dataContract/stateTransition/DataContractCreateTransition');
const DocumentsBatchTransition = require('../../../lib/document/stateTransition/DocumentsBatchTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDocumentTransitionsFixture = require('../../../lib/test/fixtures/getDocumentTransitionsFixture');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractCreateTransition;
  let documentsBatchTransition;
  let stateRepositoryMock;
  let dataContract;
  let identityPublicKey;

  beforeEach(function beforeEach() {
    const privateKeyModel = new PrivateKey();
    const privateKey = privateKeyModel.toBuffer();
    const publicKey = privateKeyModel.toPublicKey().toBuffer().toString('base64');
    const publicKeyId = 1;

    identityPublicKey = new IdentityPublicKey()
      .setId(publicKeyId)
      .setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1)
      .setData(publicKey);

    dataContract = getDocumentsFixture.dataContract;

    dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toJSON(),
      entropy: dataContract.getEntropy(),
    });
    dataContractCreateTransition.sign(identityPublicKey, privateKey);

    const documentTransitions = getDocumentTransitionsFixture({
      create: getDocumentsFixture(),
    });

    documentsBatchTransition = new DocumentsBatchTransition({
      ownerId: getDocumentsFixture.ownerId,
      contractId: dataContract.getId(),
      transitions: documentTransitions.map((t) => t.toJSON()),
    });
    documentsBatchTransition.sign(identityPublicKey, privateKey);

    const getPublicKeyById = this.sinonSandbox.stub().returns(identityPublicKey);
    const getBalance = this.sinonSandbox.stub().returns(10000);

    const identity = {
      getPublicKeyById,
      type: 2,
      getBalance,
    };

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchIdentity.resolves(identity);

    dpp = new DashPlatformProtocol({
      stateRepository: stateRepositoryMock,
    });
  });

  describe('createFromObject', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.createFromObject(
          dataContractCreateTransition.toJSON(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should skip checking for state repository if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();

      await dpp.stateTransition.createFromObject(
        dataContractCreateTransition.toJSON(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from plain object', async () => {
      const result = await dpp.stateTransition.createFromObject(
        dataContractCreateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractCreateTransition.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.createFromSerialized(
          dataContractCreateTransition.serialize(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should skip checking for state repository if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();

      await dpp.stateTransition.createFromSerialized(
        dataContractCreateTransition.serialize(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from string', async () => {
      const result = await dpp.stateTransition.createFromSerialized(
        dataContractCreateTransition.serialize(),
      );

      expect(result).to.be.an.instanceOf(DataContractCreateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractCreateTransition.toJSON());
    });
  });

  describe('validate', async () => {
    it('should return invalid result if State Transition structure is invalid', async function it() {
      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const rawStateTransition = dataContractCreateTransition.toJSON();
      delete rawStateTransition.protocolVersion;

      const result = await dpp.stateTransition.validate(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateDataSpy).to.not.be.called();
    });

    it('should validate Data Contract ST structure and data', async function it() {
      const validateStructureSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateStructure',
      );

      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const result = await dpp.stateTransition.validate(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateStructureSpy).to.be.calledOnceWith(dataContractCreateTransition);
      expect(validateDataSpy).to.be.calledOnceWith(dataContractCreateTransition);
    });

    it('should validate Documents ST structure and data', async function it() {
      stateRepositoryMock.fetchDocuments.resolves([]);

      stateRepositoryMock.fetchDataContract.resolves(dataContract);
      stateRepositoryMock.fetchIdentity.resolves({
        getPublicKeyById: this.sinonSandbox.stub().returns(identityPublicKey),
      });

      const validateStructureSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateStructure',
      );

      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const result = await dpp.stateTransition.validate(
        documentsBatchTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateStructureSpy).to.be.calledOnceWith(documentsBatchTransition);
      expect(validateDataSpy).to.be.calledOnceWith(documentsBatchTransition);
    });
  });

  describe('validateStructure', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateStructure(
          dataContractCreateTransition.toJSON(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateStructure(
        dataContractCreateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateData', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateData(
          dataContractCreateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('stateRepository');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateData(
        dataContractCreateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should validate raw state transition data', async () => {
      const rawStateTransition = dataContractCreateTransition.toJSON();

      const result = await dpp.stateTransition.validateData(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateFee', () => {
    it('should throw MissingOption if stateRepository is not set', async () => {
      dpp = new DashPlatformProtocol();

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

    it('should validate raw state transition data', async () => {
      const rawStateTransition = dataContractCreateTransition.toJSON();

      const result = await dpp.stateTransition.validateFee(rawStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });
});
