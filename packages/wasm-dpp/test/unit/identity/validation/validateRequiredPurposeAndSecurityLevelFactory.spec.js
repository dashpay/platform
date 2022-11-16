const IdentityPublicKey = require('@dashevo/dpp/lib/identity/IdentityPublicKey');

const validateRequiredPurposeAndSecurityLevelFactory = require('@dashevo/dpp/lib/identity/validation/validateRequiredPurposeAndSecurityLevelFactory');

const { expectValidationError } = require('@dashevo/dpp/lib/test/expect/expectError');

const ValidationResult = require('@dashevo/dpp/lib/validation/ValidationResult');

const MissingMasterPublicKeyError = require('@dashevo/dpp/lib/errors/consensus/basic/identity/MissingMasterPublicKeyError');

describe('validateRequiredPurposeAndSecurityLevel', () => {
  let validateRequiredPurposeAndSecurityLevel;

  beforeEach(() => {
    validateRequiredPurposeAndSecurityLevel = (
      validateRequiredPurposeAndSecurityLevelFactory()
    );
  });

  it('should return invalid result if the state transition does not contain master key', async () => {
    const result = await validateRequiredPurposeAndSecurityLevel([{
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.CRITICAL,
    }, {
      // this key must be filtered out
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      disabledAt: 42,
    }]);

    expectValidationError(result, MissingMasterPublicKeyError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1046);
  });

  it('should return valid result', async () => {
    const result = await validateRequiredPurposeAndSecurityLevel([{
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
    }]);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
