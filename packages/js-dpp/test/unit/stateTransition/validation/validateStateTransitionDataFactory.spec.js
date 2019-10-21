const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const validateStateTransitionDataFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionDataFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const DataContractStateTransition = require('../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const InvalidStateTransitionTypeError = require('../../../../lib/errors/InvalidStateTransitionTypeError');
const ConsensusError = require('../../../../lib/errors/ConsensusError');

describe('validateStateTransitionDataFactory', () => {
  let validateDataContractSTDataMock;
  let validateStateTransitionData;

  beforeEach(function beforeEach() {
    validateDataContractSTDataMock = this.sinonSandbox.stub();

    validateStateTransitionData = validateStateTransitionDataFactory({
      [stateTransitionTypes.DATA_CONTRACT]: validateDataContractSTDataMock,
    });
  });

  it('should return invalid result if State Transition type is invalid', async () => {
    const rawStateTransition = {};
    const stateTransition = {
      getType() {
        return 4343;
      },
      toJSON() {
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

    const dataContract = getDataContractFixture();
    const stateTransition = new DataContractStateTransition(dataContract);

    const result = await validateStateTransitionData(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(validateDataContractSTDataMock).to.be.calledOnceWith(stateTransition);
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContractSTDataMock.resolves(dataContractResult);

    const dataContract = getDataContractFixture();
    const stateTransition = new DataContractStateTransition(dataContract);

    const result = await validateStateTransitionData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContractSTDataMock).to.be.calledOnceWith(stateTransition);
  });
});
