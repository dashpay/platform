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
} = require('../../../..');

describe('validateStateTransitionStateFactory', () => {
  let validateDataContractSTDataMock;
  let stateTransition;
  let dpp;
  let stateRepositoryMock;

  beforeEach(async function beforeEach() {
    ({
      DashPlatformProtocol,
      InvalidStateTransitionTypeError,
      ValidationResult,
      DataContractAlreadyPresentError,
    } = await loadWasmDpp());
    const dataContract = await getDataContractFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.resolves();
    const blsMock = getBlsMock();

    dpp = new DashPlatformProtocol(blsMock, stateRepositoryMock);

    stateTransition = dpp.dataContract.createDataContractCreateTransition(dataContract);

    validateDataContractSTDataMock = this.sinonSandbox.stub();
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
      await dpp.stateTransition.validateState(stateTransition);

      expect.fail('should throw InvalidStateTransitionTypeError');
    } catch (e) {
      expect(e).to.be.instanceOf(InvalidStateTransitionTypeError);
      expect(e.getType()).to.equal(stateTransition.getType());

      expect(validateDataContractSTDataMock).to.not.be.called();
    }
  });

  it('should return invalid result if Data Contract State Transition is not valid', async () => {
    stateRepositoryMock.fetchDataContract.resolves(await getDataContractFixture());

    const result = await dpp.stateTransition.validateState(stateTransition);

    await expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.be.instanceOf(DataContractAlreadyPresentError);

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnce();
  });

  it('should return valid result', async () => {
    const result = await dpp.stateTransition.validateState(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
