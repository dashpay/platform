const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');
const DocumentsStateTransition = require('../../../lib/document/stateTransition/DocumentsStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

describe('StateTransitionFacade', () => {
  let dpp;
  let dataContractStateTransition;
  let documentsStateTransition;
  let dataProviderMock;
  let dataContract;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    dpp = new DashPlatformProtocol({
      dataProvider: dataProviderMock,
    });

    dataContract = getDataContractFixture();
    dataContractStateTransition = new DataContractStateTransition(dataContract);

    const documents = getDocumentsFixture();
    documentsStateTransition = new DocumentsStateTransition(documents);
  });

  describe('createFromObject', () => {
    it('should create State Transition from plain object', async () => {
      const result = await dpp.stateTransition.createFromObject(
        dataContractStateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(DataContractStateTransition);

      expect(result.toJSON()).to.deep.equal(dataContractStateTransition.toJSON());
    });
  });

  describe('createFromSerialized', () => {
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
      expect(result.isValid()).to.be.false();

      expect(validateStructureSpy).to.be.calledOnceWith(documentsStateTransition);
      expect(validateDataSpy).to.be.calledOnceWith(documentsStateTransition);
    });
  });

  describe('validateStructure', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateStructure(
        dataContractStateTransition.toJSON(),
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateData', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateData(dataContractStateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
    });
  });
});
