const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../../dist');

describe('InvalidDataContractError', () => {
  let rawDataContract;
  let error;
  let InvalidDataContractError;

  before(async () => {
    ({
      InvalidDataContractError,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    error = new Error('Some error');

    const dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();
  });

  it('should return errors', () => {
    const errors = [error];

    const invalidDataContractError = new InvalidDataContractError(errors, rawDataContract);

    expect(invalidDataContractError.getErrors()).to.deep.equal(errors);
  });

  it('should return Data Contract', async () => {
    const errors = [error];

    const invalidDataContractError = new InvalidDataContractError(errors, rawDataContract);

    expect(invalidDataContractError.getRawDataContract()).to.deep.equal(rawDataContract);
  });

  it('should contain message for 1 error', async () => {
    const errors = [error];

    const invalidDataContractError = new InvalidDataContractError(errors, rawDataContract);

    expect(invalidDataContractError.getMessage()).to.equal(`Data contract decode error: "${error.message}"`);
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const invalidDataContractError = new InvalidDataContractError(errors, rawDataContract);

    expect(invalidDataContractError.getMessage()).to.equal(`Data contract decode error: "${error.message}" and 1 more`);
  });
});
