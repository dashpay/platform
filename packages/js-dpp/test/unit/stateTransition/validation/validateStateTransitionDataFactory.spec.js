const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const validateStateTransitionDataFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionDataFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const DataContractFactory = require('../../../../lib/dataContract/DataContractFactory');

const InvalidStateTransitionTypeError = require('../../../../lib/errors/InvalidStateTransitionTypeError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');
const createDPPMock = require('../../../../lib/test/mocks/createDPPMock');

describe('validateStateTransitionDataFactory', () => {
  let validateDataContractSTDataMock;
  let validateStateTransitionData;
  let stateTransition;

  beforeEach(function beforeEach() {
    validateDataContractSTDataMock = this.sinonSandbox.stub();

    const dataContractFactory = new DataContractFactory(createDPPMock(), undefined);

    const dataContract = getDataContractFixture();
    stateTransition = dataContractFactory.createStateTransition(dataContract);

    validateStateTransitionData = validateStateTransitionDataFactory({
      [stateTransitionTypes.DATA_CONTRACT_CREATE]: validateDataContractSTDataMock,
    });
  });

  it('should return invalid result if State Transition type is invalid', async () => {
    const rawStateTransition = {};
    stateTransition = {
      getType() {
        return 4343;
      },
      toObject() {
        return rawStateTransition;
      },
    };

    const result = await validateStateTransitionData(stateTransition);

    expectValidationError(result, InvalidStateTransitionTypeError);

    const [error] = result.getErrors();

    expect(error.getRawStateTransition()).to.equal(rawStateTransition);

    expect(validateDataContractSTDataMock).to.not.be.called();
  });

  it('should return invalid result if Data Contract State Transition is not valid', async () => {
    const dataContractError = new ConsensusError('test');
    const dataContractResult = new ValidationResult([
      dataContractError,
    ]);

    validateDataContractSTDataMock.resolves(dataContractResult);

    const result = await validateStateTransitionData(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(validateDataContractSTDataMock).to.be.calledOnceWith(stateTransition);
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContractSTDataMock.resolves(dataContractResult);

    const result = await validateStateTransitionData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContractSTDataMock).to.be.calledOnceWith(stateTransition);
  });
});
