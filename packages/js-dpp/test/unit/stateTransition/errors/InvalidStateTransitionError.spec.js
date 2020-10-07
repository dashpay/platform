const InvalidStateTransitionError = require('../../../../lib/stateTransition/errors/InvalidStateTransitionError');
const DataContractCreateTransition = require('../../../../lib/dataContract/stateTransition/DataContractCreateTransition');
const getDataContractFixture = require('../../../../lib/test/fixtures/getDataContractFixture');

describe('InvalidStateTransitionError', () => {
  let rawStateTransition;
  let error;

  beforeEach(() => {
    error = new Error('Some error');

    const dataContract = getDataContractFixture();
    const dataContractCreateTransition = new DataContractCreateTransition({
      dataContract: dataContract.toObject(),
      entropy: dataContract.getEntropy(),
    });
    rawStateTransition = dataContractCreateTransition.toObject();
  });

  it('should return errors', () => {
    const errors = [error];

    const invalidStateTransitionError = new InvalidStateTransitionError(errors, rawStateTransition);

    expect(invalidStateTransitionError.getErrors()).to.deep.equal(errors);
  });

  it('should return State Transition', async () => {
    const errors = [error];

    const invalidStateTransitionError = new InvalidStateTransitionError(errors, rawStateTransition);

    expect(invalidStateTransitionError.getRawStateTransition()).to.deep.equal(rawStateTransition);
  });

  it('should contain message for 1 error', async () => {
    const errors = [error];

    const invalidStateTransitionError = new InvalidStateTransitionError(errors, rawStateTransition);

    expect(invalidStateTransitionError.message).to.equal(`Invalid State Transition: "${error.message}"`);
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const invalidStateTransitionError = new InvalidStateTransitionError(errors, rawStateTransition);

    expect(invalidStateTransitionError.message).to.equal(`Invalid State Transition: "${error.message}" and 1 more`);
  });
});
