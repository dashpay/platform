const fetchAndValidateDataContractFactory = require('../../../lib/document/fetchAndValidateDataContractFactory');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const MissingDataContractIdError = require('../../../lib/errors/MissingDataContractIdError');
const DataContractNotPresentError = require('../../../lib/errors/DataContractNotPresentError');

const { expectValidationError } = require('../../../lib/test/expect/expectError');

describe('fetchAndValidateDataContractFactory', () => {
  let stateRepositoryMock;
  let fetchAndValidateDataContract;
  let rawDocument;

  beforeEach(function beforeEach() {
    const dataContract = getDataContractFixture();

    const [document] = getDocumentsFixture(dataContract);
    rawDocument = document.toObject();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    fetchAndValidateDataContract = fetchAndValidateDataContractFactory(
      stateRepositoryMock,
    );
  });

  it('should return with invalid result if $dataContractId is not present', async () => {
    delete rawDocument.$dataContractId;

    const result = await fetchAndValidateDataContract(rawDocument);

    expectValidationError(result, MissingDataContractIdError);

    const [error] = result.getErrors();

    expect(error.getRawDocument()).to.equal(rawDocument);
  });

  it('should return with invalid result if Data Contract is not present', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    const result = await fetchAndValidateDataContract(rawDocument);

    expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContractId()).to.deep.equal(rawDocument.$dataContractId);
  });

  it('should return valid result', async () => {
    const result = await fetchAndValidateDataContract(rawDocument);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
