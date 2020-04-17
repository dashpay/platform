const InvalidIdentityError = require('../../../../lib/identity/errors/InvalidIdentityError');
const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');

describe('InvalidIdentityError', () => {
  let rawIdentity;
  let error;

  beforeEach(() => {
    error = new Error('Some error');

    const identity = getIdentityFixture();
    rawIdentity = identity.toJSON();
  });

  it('should return errors', () => {
    const errors = [error];

    const invalidIdentityError = new InvalidIdentityError(errors, rawIdentity);

    expect(invalidIdentityError.getErrors()).to.deep.equal(errors);
  });

  it('should return Identity', async () => {
    const errors = [error];

    const invalidIdentityError = new InvalidIdentityError(errors, rawIdentity);

    expect(invalidIdentityError.getRawIdentity()).to.deep.equal(rawIdentity);
  });

  it('should contain message for 1 error', async () => {
    const errors = [error];

    const invalidIdentityError = new InvalidIdentityError(errors, rawIdentity);

    expect(invalidIdentityError.message).to.equal(`Invalid Identity: "${error.message}"`);
  });

  it('should contain message for multiple errors', async () => {
    const errors = [error, error];

    const invalidIdentityError = new InvalidIdentityError(errors, rawIdentity);

    expect(invalidIdentityError.message).to.equal(`Invalid Identity: "${error.message}" and 1 more`);
  });
});
