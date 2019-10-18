const validateDataContractSTDataFactory = require('../../../../../lib/dataContract/stateTransition/validation/validateDataContractSTDataFactory');
const DataContractStateTransition = require('../../../../../lib/dataContract/stateTransition/DataContractStateTransition');

const createDataProviderMock = require('../../../../../lib/test/mocks/createDataProviderMock');
const getDataContractFixture = require('../../../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../../../lib/errors/DataContractAlreadyPresentError');
const ConsensusError = require('../../../../../lib/errors/ConsensusError');

describe('validateDataContractSTDataFactory', () => {
  let validateDataContractSTData;
  let dataContract;
  let stateTransition;
  let dataProviderMock;
  let registrationTransaction;
  let validateBlockchainUserMock;

  beforeEach(function beforeEach() {
    dataProviderMock = createDataProviderMock(this.sinonSandbox);

    validateBlockchainUserMock = this.sinonSandbox.stub().resolves(new ValidationResult());

    registrationTransaction = {
      confirmations: 6,
    };

    dataContract = getDataContractFixture();
    stateTransition = new DataContractStateTransition(dataContract);

    validateDataContractSTData = validateDataContractSTDataFactory(
      dataProviderMock,
      validateBlockchainUserMock,
    );
  });

  it('should return invalid result if Data Contract Identity is invalid', async () => {
    const blockchainUserError = new ConsensusError('error');

    validateBlockchainUserMock.resolves(
      new ValidationResult([blockchainUserError]),
    );

    const result = await validateDataContractSTData(stateTransition);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(blockchainUserError);

    expect(validateBlockchainUserMock).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.not.be.called();
  });

  it('should return invalid result if Data Contract with specified contractId is already exist', async () => {
    dataProviderMock.fetchTransaction.resolves(registrationTransaction);
    dataProviderMock.fetchDataContract.resolves(dataContract);

    const result = await validateDataContractSTData(stateTransition);

    expectValidationError(result, DataContractAlreadyPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContract()).to.equal(dataContract);

    expect(validateBlockchainUserMock).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.be.calledOnceWith(dataContract.getId());
  });

  it('should return valid result', async () => {
    dataProviderMock.fetchTransaction.resolves(registrationTransaction);

    const result = await validateDataContractSTData(stateTransition);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();

    expect(validateBlockchainUserMock).to.be.calledOnceWith(dataContract.getId());
    expect(dataProviderMock.fetchDataContract).to.be.calledOnceWith(dataContract.getId());
  });
});
