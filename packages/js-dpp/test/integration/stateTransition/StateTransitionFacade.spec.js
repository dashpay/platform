const DashPlatformProtocol = require('../../../lib/DashPlatformProtocol');

const DataContractStateTransition = require('../../../lib/dataContract/stateTransition/DataContractStateTransition');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const createDataProviderMock = require('../../../lib/test/mocks/createDataProviderMock');

describe('StateTransition', () => {
  let dpp;
  let dataContract;
  let stateTransition;

  beforeEach(function beforeEach() {
    dpp = new DashPlatformProtocol({
      dataProvider: createDataProviderMock(this.sinonSandbox),
    });

    dataContract = getDataContractFixture();
    stateTransition = new DataContractStateTransition(dataContract);
  });

  describe('createFromObject', () => {
    it('should create State Transition from plain object', () => {
      const result = dpp.stateTransition.createFromObject(stateTransition.toJSON());

      expect(result).to.be.an.instanceOf(DataContractStateTransition);

      expect(result.toJSON()).to.deep.equal(stateTransition.toJSON());
    });
  });

  describe('createFromSerialized', () => {
    it('should create State Transition from string', () => {
      const result = dpp.stateTransition.createFromSerialized(stateTransition.serialize());

      expect(result).to.be.an.instanceOf(DataContractStateTransition);

      expect(result.toJSON()).to.deep.equal(stateTransition.toJSON());
    });
  });

  describe('validate', () => {
    it('should return invalid result if State Transition structure is invalid', async function it() {
      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const rawStateTransiton = stateTransition.toJSON();
      delete rawStateTransiton.protocolVersion;

      const result = await dpp.stateTransition.validate(rawStateTransiton);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateDataSpy).to.not.be.called();
    });

    it('should validate structure and data', async function it() {
      const validateStructureSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateStructure',
      );

      const validateDataSpy = this.sinonSandbox.spy(
        dpp.stateTransition,
        'validateData',
      );

      const result = await dpp.stateTransition.validate(
        stateTransition,
        { skipStructureValidation: true },
      );

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();

      expect(validateStructureSpy).to.be.calledOnceWith(stateTransition);
      expect(validateDataSpy).to.be.called(stateTransition);
    });
  });

  describe('validateStructure', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateStructure(stateTransition.toJSON());

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });
  });

  describe('validateData', () => {
    it('should validate State Transition', async () => {
      const result = await dpp.stateTransition.validateData(stateTransition);

      expect(result).to.be.an.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.false();
    });
  });
});
