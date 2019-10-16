const validateDataContractSTDataFactory = require('../../../../lib/dataContract/stateTransition/validateDataContractSTDataFactory');
const DataContractStateTransition = require('../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const createDataProviderMock = require('../../../../lib/test/mocks/createDataProviderMock');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const DataContractIdentityNotFoundError = require('../../../../lib/errors/DataContractIdentityNotFoundError');
const UnconfirmedUserError = require('../../../../lib/errors/UnconfirmedUserError');
const DataContractAlreadyPresentError = require('../../../../lib/errors/DataContractAlreadyPresentError');

describe('validateDataContractSTDataFactory', () => {
  let validateDataContractSTData;
  let dataContract;
  let stateTransition;
  let dataProviderMock;
  let registrationTransaction;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    registrationTransaction = {
      confirmations: 6,
    };

    dataContract = getDataContractFixture();
    stateTransition = new DataContractStateTransition(dataContract);

    validateDataContractSTData = validateDataContractSTDataFactory(dataProviderMock);
  });

  it('should return invalid result if contractId is wrong', async () => {
    const result = await validateDataContractSTData(stateTransition);

    expectValidationError(result, DataContractIdentityNotFoundError);

    const [error] = result.getErrors();

    expect(error.getDataContractId()).to.equal(dataContract.getId());

    expect(dataProviderMock.fetchTransaction).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.not.be.called();
  });

  it('should return invalid result if Data Contract Identity is not confirmed', async () => {
    registrationTransaction.confirmations = 2;

    dataProviderMock.fetchTransaction.resolves(registrationTransaction);

    const result = await validateDataContractSTData(stateTransition);

    expectValidationError(result, UnconfirmedUserError);

    const [error] = result.getErrors();

    expect(error.getRegistrationTransaction()).to.equal(registrationTransaction);

    expect(dataProviderMock.fetchTransaction).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.not.be.called();
  });

  it('should return invalid result if Data Contract with specified contractId is already exist', async () => {
    dataProviderMock.fetchTransaction.resolves(registrationTransaction);
    dataProviderMock.fetchDataContract.resolves(dataContract);

    const result = await validateDataContractSTData(stateTransition);

    expectValidationError(result, DataContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContract()).to.equal(dataContract);

    expect(dataProviderMock.fetchTransaction).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.be.calledOnceWith(dataContract.getId());
  });

  it('should return valid result', async () => {
    dataProviderMock.fetchTransaction.resolves(registrationTransaction);

    const result = await validateDataContractSTData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(dataProviderMock.fetchTransaction).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.be.calledOnceWith(dataContract.getId());
  });
});
