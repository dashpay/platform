const getDataContractFixture = require('@dashevo/dpp/lib/test/fixtures/getDataContractFixture');

const { default: loadWasmDpp } = require('../../../../dist');

describe('DataContractDecodeError', () => {
  let rawDataContract;
  let error;
  let DataContractDecodeError;

  before(async () => {
    ({
      DataContractDecodeError,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    error = new Error('Some error');

    const dataContract = getDataContractFixture();
    rawDataContract = dataContract.toObject();
  });

  it('should return errors', () => {
    const errors = [error];

    const dataContractDecodeError = new DataContractDecodeError(errors, rawDataContract);

    expect(dataContractDecodeError.getErrors()).to.deep.equal(errors);
  });

  it('should return Data Contract', async () => {
    const errors = [error];

    const dataContractDecodeError = new DataContractDecodeError(errors, rawDataContract);

    expect(dataContractDecodeError.getRawDataContract()).to.deep.equal(rawDataContract);
  });

  it('should contain message for 1 error', async () => {
    const errors = [error];

    const dataContractDecodeError = new DataContractDecodeError(errors, rawDataContract);

    expect(dataContractDecodeError.getMessage()).to.equal(`Data contract decode error: "${error.message}"`);
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const dataContractDecodeError = new DataContractDecodeError(errors, rawDataContract);

    expect(dataContractDecodeError.getMessage()).to.equal(`Data contract decode error: "${error.message}" and 1 more`);
  });
});
