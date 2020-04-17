const fetchAndValidateDataContractFactory = require('../../../lib/document/fetchAndValidateDataContractFactory');

const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');

const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const MissingDocumentContractIdError = require('../../../lib/errors/MissingDocumentContractIdError');
const DataContractNotPresentError = require('../../../lib/errors/DataContractNotPresentError');

const { expectValidationError } = require('../../../lib/test/expect/expectError');

describe('fetchAndValidateDataContractFactory', () => {
  let stateRepositoryMock;
  let fetchAndValidateDataContract;
  let document;

  beforeEach(function beforeEach() {
    [document] = getDocumentsFixture();

    const dataContract = getDataContractFixture();

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);

    stateRepositoryMock.fetchDataContract.resolves(dataContract);

    fetchAndValidateDataContract = fetchAndValidateDataContractFactory(
      stateRepositoryMock,
    );
  });

  it('should return with invalid result if $dataContractId is not present', async () => {
    const rawDocument = document.toJSON();

    delete rawDocument.$dataContractId;

    const result = await fetchAndValidateDataContract(rawDocument);

    expectValidationError(result, MissingDocumentContractIdError);

    const [error] = result.getErrors();

    expect(error.getRawDocument()).to.equal(rawDocument);
  });

  it('should return with invalid result if Data Contract is not present', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    const result = await fetchAndValidateDataContract(document);

    expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getDataContractId()).to.equal(document.getDataContractId());
  });

  it('should return valid result', async () => {
    const result = await fetchAndValidateDataContract(document);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
