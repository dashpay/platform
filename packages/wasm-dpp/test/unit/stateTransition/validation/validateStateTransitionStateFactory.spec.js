const createStateRepositoryMock = require('../../../../lib/test/mocks/createStateRepositoryMock');
const getBlsMock = require('../../../../lib/test/mocks/getBlsAdapterMock');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');
const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../..');
let {
  DashPlatformProtocol,
  InvalidStateTransitionTypeError,
  ValidationResult,
  DataContractAlreadyPresentError,
  StateTransitionExecutionContext,
} = require('../../../..');

describe.skip('validateStateTransitionStateFactory', () => {
  let stateTransition;
  let dpp;
  let stateRepositoryMock;
  let executionContext;

  beforeEach(async function beforeEach() {
    ({
      DashPlatformProtocol,
      InvalidStateTransitionTypeError,
      ValidationResult,
      DataContractAlreadyPresentError,
      StateTransitionExecutionContext,
    } = await loadWasmDpp());
    const dataContract = await getDataContractFixture();
    executionContext = new StateTransitionExecutionContext();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves();
    const blsMock = getBlsMock();

    dpp = new DashPlatformProtocol(blsMock, stateRepositoryMock);

    stateTransition = dpp.dataContract.createDataContractCreateTransition(dataContract);
  });

  it('should return invalid result if State Transition type is invalid', async () => {
    const rawStateTransition = {};
    stateTransition = {
      getType() {
        return 234;
      },
      toObject() {
        return rawStateTransition;
      },
    };

    try {
      await dpp.stateTransition.validateState(stateTransition, executionContext);

      expect.fail('should throw InvalidStateTransitionTypeError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidStateTransitionTypeError);
      expect(e.getType()).to.equal(stateTransition.getType());
    }
  });

  it('should return invalid result if Data Contract State Transition is not valid', async () => {
    stateRepositoryMock.fetchDataContract.resolves(await getDataContractFixture());

    const result = await dpp.stateTransition.validateState(stateTransition, executionContext);

    await expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(DataContractAlreadyPresentError);

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnce();
  });

  it('should return valid result', async () => {
    const result = await dpp.stateTransition.validateState(stateTransition, executionContext);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
