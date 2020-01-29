const { PrivateKey } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');
const DocumentsStateTransition = require('../../../lib/document/stateTransition/DocumentsStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const Identity = require('../../../lib/identity/Identity');
const IdentityPublicKey = require('../../../lib/identity/IdentityPublicKey');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractStateTransition;
  let documentsStateTransition;
  let dataProviderMock;
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

    dataContract = getDataContractFixture();
    dataContractStateTransition = new DataContractStateTransition(dataContract);
    dataContractStateTransition.sign(identityPublicKey, privateKey);

    const documents = getDocumentsFixture();
    documentsStateTransition = new DocumentsStateTransition(documents);
    documentsStateTransition.sign(identityPublicKey, privateKey);

    const getPublicKeyById = this.sinonSandbox.stub().returns(identityPublicKey);

    const identity = {
      getPublicKeyById,
      type: 2,
    };

    dataProviderMock = createDataProviderMock(this.sinonSandbox);
    dataProviderMock.fetchIdentity.resolves(identity);

    dpp = new DashPlatformProtocol({
      dataProvider: dataProviderMock,
    });
  });

  describe('createFromObject', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.createFromObject(
          dataContractStateTransition.toJSON(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should skip checking for data provider if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();

      await dpp.stateTransition.createFromObject(
        dataContractStateTransition.toJSON(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from plain object', async () => {
      const result = await dpp.stateTransition.createFromObject(
        dataContractStateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(DataContractStateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractStateTransition.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.createFromSerialized(
          dataContractStateTransition.serialize(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should skip checking for data provider if skipValidation is set', async () => {
      dpp = new DashPlatformProtocol();

      await dpp.stateTransition.createFromSerialized(
        dataContractStateTransition.serialize(),
        { skipValidation: true },
      );
    });

    it('should create State Transition from string', async () => {
      const result = await dpp.stateTransition.createFromSerialized(
        dataContractStateTransition.serialize(),
      );

      expect(result).to.be.an.instanceOf(DataContractStateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractStateTransition.toJSON());
    });
  });

  describe('validate', async () => {
    it('should return invalid result if State Transition structure is invalid', async function it() {
      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const rawStateTransition = dataContractStateTransition.toJSON();
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
        dataContractStateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateStructureSpy).to.be.calledOnceWith(dataContractStateTransition);
      expect(validateDataSpy).to.be.calledOnceWith(dataContractStateTransition);
    });

    it('should validate Documents ST structure and data', async function it() {
      dataProviderMock.fetchDocuments.resolves([]);

      dataProviderMock.fetchDataContract.resolves(dataContract);
      dataProviderMock.fetchIdentity.resolves({
        type: Identity.TYPES.USER,
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
        documentsStateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();

      expect(validateStructureSpy).to.be.calledOnceWith(documentsStateTransition);
      expect(validateDataSpy).to.be.calledOnceWith(documentsStateTransition);
    });
  });

  describe('validateStructure', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateStructure(
          dataContractStateTransition.toJSON(),
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateStructure(
        dataContractStateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateData', () => {
    it('should throw MissingOption if dataProvider is not set', async () => {
      dpp = new DashPlatformProtocol();

      try {
        await dpp.stateTransition.validateData(
          dataContractStateTransition,
        );

        expect.fail('MissingOption should be thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(MissingOptionError);
        expect(e.getOptionName()).to.equal('dataProvider');
      }
    });

    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateData(
        dataContractStateTransition,
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });
});
