const IdentityPublicKey = require('../../../../lib/identity/IdentityPublicKey');

const validatePublicKeysInIdentityCreateTransitionFactory = require('../../../../lib/identity/validation/validatePublicKeysInIdentityCreateTransitionFactory');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

const MissingMasterKeyError = require('../../../../lib/errors/consensus/basic/identity/MissingMasterKeyError');

describe('validateIdentityExistence', () => {
  let validatePublicKeysInIdentityCreateTransition;

  beforeEach(() => {
    validatePublicKeysInIdentityCreateTransition = (
      validatePublicKeysInIdentityCreateTransitionFactory()
    );
  });

  it('should return invalid result if identity is not found', async () => {
    const result = await validatePublicKeysInIdentityCreateTransition([{
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
    }]);

    expectValidationError(result, MissingMasterKeyError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1046);
  });

  it('should return valid result', async () => {
    const result = await validatePublicKeysInIdentityCreateTransition([{
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
    }]);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
