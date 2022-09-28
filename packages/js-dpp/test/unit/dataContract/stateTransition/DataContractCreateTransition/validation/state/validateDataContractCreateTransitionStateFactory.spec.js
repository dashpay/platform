const validateDataContractCreateTransitionStateFactory = require('../../../../../../../lib/dataContract/stateTransition/DataContractCreateTransition/validation/state/validateDataContractCreateTransitionStateFactory');
const DataContractCreateTransition = require('../../../../../../../lib/dataContract/stateTransition/DataContractCreateTransition/DataContractCreateTransition');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../../../../../lib/errors/consensus/state/dataContract/DataContractAlreadyPresentError');
const StateTransitionExecutionContext = require('../../../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateDataContractCreateTransitionStateFactory', () => {
  let validateDataContractCreateTransitionState;
  let dataContract;
  let stateTransition;
  let stateRepositoryMock;
  let executionContext;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    dataContract = getDataContractFixture();
    stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    validateDataContractCreateTransitionState = validateDataContractCreateTransitionStateFactory(
      stateRepositoryMock,
    );
  });

  it('should return invalid result if Data Contract with specified contractId is already exist', async () => {
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    const result = await validateDataContractCreateTransitionState(stateTransition);

    expectValidationError(result, DataContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(4000);
    expect(Buffer.isBuffer(error.getDataContractId())).to.be.true();
    expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });

  it('should return valid result', async () => {
    const result = await validateDataContractCreateTransitionState(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });

  it('should return valid result on dry run', async () => {
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    executionContext.enableDryRun();
    const result = await validateDataContractCreateTransitionState(stateTransition);
    executionContext.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });
});
