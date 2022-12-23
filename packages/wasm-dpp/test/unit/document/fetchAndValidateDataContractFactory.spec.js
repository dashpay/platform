const fetchAndValidateDataContractFactory = require('@dashevo/dpp/lib/document/fetchAndValidateDataContractFactory');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const MissingDataContractIdError = require('@dashevo/dpp/lib/errors/consensus/basic/document/MissingDataContractIdError');
const DataContractNotPresentError = require('@dashevo/dpp/lib/errors/consensus/basic/document/DataContractNotPresentError');

const { expectValidationError } = require('@dashevo/dpp/lib/test/expect/expectError');

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

    expect(error.getCode()).to.equal(1025);
  });

  it('should return with invalid result if Data Contract is not present', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    const result = await fetchAndValidateDataContract(rawDocument);

    expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1018);
    expect(error.getDataContractId()).to.deep.equal(rawDocument.$dataContractId);
  });

  it('should return valid result', async () => {
    const result = await fetchAndValidateDataContract(rawDocument);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
