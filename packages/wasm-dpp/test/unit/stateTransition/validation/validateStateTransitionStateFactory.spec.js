const stateTransitionTypes = require('../../../../lib/stateTransition/stateTransitionTypes');

const validateStateTransitionStateFactory = require('../../../../lib/stateTransition/validation/validateStateTransitionStateFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const DataContractFactory = require('../../../../lib/dataContract/DataContractFactory');

const InvalidStateTransitionTypeError = require('../../../../lib/stateTransition/errors/InvalidStateTransitionTypeError');
const createDPPMock = require('../../../../lib/test/mocks/createDPPMock');
const SomeConsensusError = require('../../../../lib/test/mocks/SomeConsensusError');

describe('validateStateTransitionStateFactory', () => {
  let validateDataContractSTDataMock;
  let validateStateTransitionState;
  let stateTransition;

  beforeEach(function beforeEach() {
    validateDataContractSTDataMock = this.sinonSandbox.stub();

    const dataContractFactory = new DataContractFactory(createDPPMock(), undefined);

    const dataContract = getDataContractFixture();
    stateTransition = dataContractFactory.createDataContractCreateTransition(dataContract);

    validateStateTransitionState = validateStateTransitionStateFactory({
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

    try {
      await validateStateTransitionState(stateTransition);

      expect.fail('should throw InvalidStateTransitionTypeError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidStateTransitionTypeError);
      expect(e.getType()).to.equal(stateTransition.getType());

      expect(validateDataContractSTDataMock).to.not.be.called();
    }
  });

  it('should return invalid result if Data Contract State Transition is not valid', async () => {
    const dataContractError = new SomeConsensusError('test');
    const dataContractResult = new ValidationResult([
      dataContractError,
    ]);

    validateDataContractSTDataMock.resolves(dataContractResult);

    const result = await validateStateTransitionState(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(dataContractError);

    expect(validateDataContractSTDataMock).to.be.calledOnceWith(stateTransition);
  });

  it('should return valid result', async () => {
    const dataContractResult = new ValidationResult();

    validateDataContractSTDataMock.resolves(dataContractResult);

    const result = await validateStateTransitionState(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateDataContractSTDataMock).to.be.calledOnceWith(stateTransition);
  });
});
