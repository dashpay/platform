const validateDataContractUpdateTransitionStateFactory = require('../../../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/validation/state/validateDataContractUpdateTransitionStateFactory');
const DataContractUpdateTransition = require('../../../../../../../lib/dataContract/stateTransition/DataContractUpdateTransition/DataContractUpdateTransition');

const createStateRepositoryMock = require('../../../../../../../lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('../../../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../../../lib/validation/ValidationResult');

const DataContractNotPresentError = require('../../../../../../../lib/errors/consensus/basic/document/DataContractNotPresentError');
const InvalidDataContractVersionError = require('../../../../../../../lib/errors/consensus/basic/dataContract/InvalidDataContractVersionError');
const StateTransitionExecutionContext = require('../../../../../../../lib/stateTransition/StateTransitionExecutionContext');

describe('validateDataContractUpdateTransitionStateFactory', () => {
  let validateDataContractUpdateTransitionState;
  let dataContract;
  let stateTransition;
  let stateRepositoryMock;
  let executionContext;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    dataContract = getDataContractFixture();

    const updatedRawDataContract = dataContract.toObject();

    updatedRawDataContract.version += 1;

    stateTransition = new DataContractUpdateTransition({
      dataContract: updatedRawDataContract,
    });

    executionContext = new StateTransitionExecutionContext();

    stateTransition.setExecutionContext(executionContext);

    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    validateDataContractUpdateTransitionState = validateDataContractUpdateTransitionStateFactory(
      stateRepositoryMock,
    );
  });

  it('should return invalid result if Data Contract with specified contractId was not found', async () => {
    stateRepositoryMock.fetchDataContract.resolves(undefined);

    const result = await validateDataContractUpdateTransitionState(stateTransition);

    expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1018);
    expect(Buffer.isBuffer(error.getDataContractId())).to.be.true();
    expect(error.getDataContractId()).to.deep.equal(dataContract.getId().toBuffer());

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });

  it('should return invalid result if Data Contract version is not larger by 1', async () => {
    dataContract.version -= 1;

    const result = await validateDataContractUpdateTransitionState(stateTransition);

    expectValidationError(result, InvalidDataContractVersionError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1050);

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });

  it('should return valid result', async () => {
    const result = await validateDataContractUpdateTransitionState(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });

  it('should return valid result on dry run', async () => {
    stateRepositoryMock.fetchDataContract.resolves(undefined);

    executionContext.enableDryRun();

    const result = await validateDataContractUpdateTransitionState(stateTransition);

    executionContext.disableDryRun();

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(
      dataContract.getId(),
      executionContext,
    );
  });
});
