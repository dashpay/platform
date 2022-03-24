const validatePublicKeysAreEnabled = require(
  '../../../../lib/identity/validation/validatePublicKeysAreEnabled',
);
const { expectValidationError } = require('../../../../lib/test/expect/expectError');
const InvalidIdentityPublicKeyDisabledError = require('../../../../lib/errors/consensus/state/identity/InvalidIdentityPublicKeyDisabledError');
const getIdentityCreateTransitionFixture = require('../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

describe('validatePublicKeysAreEnabled', () => {
  let rawPublicKeys;

  beforeEach(() => {
    const stateTransition = getIdentityCreateTransitionFixture();
    const rawStateTransition = stateTransition.toObject();

    rawPublicKeys = rawStateTransition.publicKeys;
  });

  it('should return valid result', () => {
    const result = validatePublicKeysAreEnabled(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should return InvalidIdentityPublicKeyDisabledError', () => {
    rawPublicKeys[0].disabledAt = new Date();

    const result = validatePublicKeysAreEnabled(rawPublicKeys);

    expectValidationError(result, InvalidIdentityPublicKeyDisabledError);

    const [error] = result.getErrors();
    expect(error.getId()).to.equal(0);
  });
});
