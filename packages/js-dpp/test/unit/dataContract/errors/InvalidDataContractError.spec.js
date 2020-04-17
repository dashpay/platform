const InvalidDataContractError = require('../../../../lib/dataContract/errors/InvalidDataContractError');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

describe('InvalidDataContractError', () => {
  let rawDataContract;
  let error;

  beforeEach(() => {
    error = new Error('Some error');

    const dataContract = getDataContractFixture();
    rawDataContract = dataContract.toJSON();
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

    expect(invalidDataContractError.message).to.equal(`Invalid Data Contract: "${error.message}"`);
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const invalidDataContractError = new InvalidDataContractError(errors, rawDataContract);

    expect(invalidDataContractError.message).to.equal(`Invalid Data Contract: "${error.message}" and 1 more`);
  });
});
