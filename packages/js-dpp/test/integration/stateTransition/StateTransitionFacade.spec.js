const { Transaction } = require('@dashevo/dashcore-lib');

const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');
const DocumentsStateTransition = require('../../../lib/document/stateTransition/DocumentsStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

const MissingOptionError = require('../../../lib/errors/MissingOptionError');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractStateTransition;
  let documentsStateTransition;
  let dataProviderMock;
  let dataContract;

  beforeEach(function beforeEach() {
    dataContract = getDataContractFixture();
    dataContractStateTransition = new DataContractStateTransition(dataContract);

    const documents = getDocumentsFixture();
    documentsStateTransition = new DocumentsStateTransition(documents);

    dataProviderMock = createDataProviderMock(this.sinonSandbox);

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
      expect(result.isValid()).to.be.false();

      expect(validateStructureSpy).to.be.calledOnceWith(dataContractStateTransition);
      expect(validateDataSpy).to.be.calledOnceWith(dataContractStateTransition);
    });

    it('should validate Documents ST structure and data', async function it() {
      dataProviderMock.fetchTransaction.resolves({
        type: Transaction.TYPES.TRANSACTION_SUBTX_REGISTER,
        confirmations: 6,
      });

      dataProviderMock.fetchDocuments.resolves([]);


      dataProviderMock.fetchDataContract.resolves(dataContract);

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
      expect(result.isValid()).to.be.false();
    });
  });
});
