const fetchAndValidateDataContractFactory = require('@dashevo/dpp/lib/document/fetchAndValidateDataContractFactory');

const createStateRepositoryMock = require('@dashevo/dpp/lib/test/mocks/createStateRepositoryMock');

const getDocumentsFixture = require('@dashevo/dpp/lib/test/fixtures/getDocumentsFixture');
const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const ValidationResultJs = require('@dashevo/dpp/lib/validation/ValidationResult');

const MissingDataContractIdErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/MissingDataContractIdError');
const DataContractNotPresentErrorJs = require('@dashevo/dpp/lib/errors/consensus/basic/document/DataContractNotPresentError');

const { expectValidationError } = require('@dashevo/dpp/lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../dist');

let ValidationResult;
let DataContract;

describe('fetchAndValidateDataContractFactory', () => {
  let stateRepositoryMockJs;
  let stateRepositoryMock;
  let fetchAndValidateDataContractJs;
  let fetchAndValidateDataContract;
  let rawDocument;

  beforeEach(async () => {
    ({
      DataContract,
      fetchAndValidateDataContract,
      ValidationResult,
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    const dataContractJs = getDataContractFixture();
    const dataContract = DataContract.fromBuffer(dataContractJs.toBuffer());

    const [document] = getDocumentsFixture(dataContractJs);
    rawDocument = document.toObject();

    stateRepositoryMockJs = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMockJs.fetchDataContract.resolves(dataContractJs);

    stateRepositoryMock = createStateRepositoryMock(this.sinonSandbox);
    stateRepositoryMock.fetchDataContract.returns(dataContract);

    fetchAndValidateDataContractJs = fetchAndValidateDataContractFactory(
      stateRepositoryMockJs,
    );
  });

  it('should return with invalid result if $dataContractId is not present', async () => {
    delete rawDocument.$dataContractId;

    const result = await fetchAndValidateDataContractJs(rawDocument);

    expectValidationError(result, MissingDataContractIdErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1025);
  });

  it('should return with invalid result if $dataContractId is not present - Rust', async () => {
    delete rawDocument.$dataContractId;

    const result = await fetchAndValidateDataContract(stateRepositoryMock, rawDocument);

    const [error] = result.getErrors();
    expect(error.getCode()).to.equal(1025);
  });

  it('should return with invalid result if Data Contract is not present', async () => {
    stateRepositoryMockJs.fetchDataContract.resolves(null);

    const result = await fetchAndValidateDataContractJs(rawDocument);

    expectValidationError(result, DataContractNotPresentErrorJs);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1018);
    expect(error.getDataContractId()).to.deep.equal(rawDocument.$dataContractId);
  });

  it('should return with invalid result if Data Contract is not present - Rust', async () => {
    stateRepositoryMock.fetchDataContract.returns(null);

    const result = await fetchAndValidateDataContract(stateRepositoryMock, rawDocument);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1018);
    expect(error.getDataContractId()).to.deep.equal(rawDocument.$dataContractId);
  });

  it('should return valid result', async () => {
    const result = await fetchAndValidateDataContractJs(rawDocument);

    expect(result).to.be.an.instanceOf(ValidationResultJs);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result - Rust', async () => {
    const result = await fetchAndValidateDataContract(stateRepositoryMock, rawDocument);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
