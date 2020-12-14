const validateDataContractCreateTransitionDataFactory = require('../../../../../lib/dataContract/stateTransition/validation/validateDataContractCreateTransitionDataFactory');
const DataContractCreateTransition = require('../../../../../lib/dataContract/stateTransition/DataContractCreateTransition');

const createStateRepositoryMock = require('../../../../../lib/test/mocks/createStateRepositoryMock');
const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../../../lib/errors/DataContractAlreadyPresentError');

describe('validateDataContractCreateTransitionDataFactory', () => {
  let validateDataContractCreateTransitionData;
  let dataContract;
  let stateTransition;
  let stateRepositoryMock;

  beforeEach(function beforeEach() {
    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    dataContract = getDataContractFixture();
    stateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });

    validateDataContractCreateTransitionData = validateDataContractCreateTransitionDataFactory(
      stateRepositoryMock,
    );
  });

  it('should return invalid result if Data Contract with specified contractId is already exist', async () => {
    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    const result = await validateDataContractCreateTransitionData(stateTransition);

    expectValidationError(result, DataContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContract().toObject()).to.deep.equal(dataContract.toObject());

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(dataContract.getId());
  });

  it('should return valid result', async () => {
    const result = await validateDataContractCreateTransitionData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(stateRepositoryMock.fetchDataContract).to.be.calledOnceWithExactly(dataContract.getId());
  });
});
