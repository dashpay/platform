const createStateRepositoryMock = require('../../../lib/test/mocks/createStateRepositoryMock');
const getDocumentsFixture = require('../../../lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('../../../lib/test/fixtures/getDataContractFixture');

const { expectValidationError } = require('../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../dist');

let ValidationResult;
let MissingDataContractIdError;
let DataContractNotPresentError;

describe.skip('fetchAndValidateDataContractFactory', () => {
  let stateRepositoryMock;
  let fetchAndValidateDataContract;
  let rawDocument;

  beforeEach(async () => {
    ({
      fetchAndValidateDataContract,
      ValidationResult,
      MissingDataContractIdError,
      DataContractNotPresentError,
    } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    const dataContract = await getDataContractFixture();

    const [document] = await getDocumentsFixture(dataContract);
    rawDocument = document.toObject();

    stateRepositoryMock = createStateRepositoryMock(this.sinon);
    stateRepositoryMock.fetchDataContract.resolves(dataContract);
  });

  it('should return with invalid result if $dataContractId is not present', async () => {
    delete rawDocument.$dataContractId;

    const result = await fetchAndValidateDataContract(stateRepositoryMock, rawDocument);

    await expectValidationError(result, MissingDataContractIdError);

    const [error] = result.getErrors();
    expect(error.getCode()).to.equal(1025);
  });

  it('should return with invalid result if Data Contract is not present', async () => {
    stateRepositoryMock.fetchDataContract.resolves(null);

    const result = await fetchAndValidateDataContract(stateRepositoryMock, rawDocument);

    await expectValidationError(result, DataContractNotPresentError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1018);
    expect(error.getDataContractId()).to.deep.equal(rawDocument.$dataContractId);
  });

  it('should return valid result', async () => {
    const result = await fetchAndValidateDataContract(stateRepositoryMock, rawDocument);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
