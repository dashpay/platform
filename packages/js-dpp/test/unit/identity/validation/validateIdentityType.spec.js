const validateIdentityType = require('../../../../lib/identity/validation/validateIdentityType');

const Identity = require('../../../../lib/identity/Identity');

const { expectValidationError } = require('../../../../lib/test/expect/expectError');

const UnknownIdentityTypeError = require('../../../../lib/errors/UnknownIdentityTypeError');

const ValidationResult = require('../../../../lib/validation/ValidationResult');

describe('validateIdentityType', () => {
  it('should return invalid result if type is unknown', () => {
    const type = 42;

    const result = validateIdentityType(type);

    expectValidationError(result, UnknownIdentityTypeError);

    const [error] = result.getErrors();

    expect(error.getType()).to.equal(type);
  });

  it('should return valid result if type is defined in protocol', () => {
    const type = 1;

    const result = validateIdentityType(type);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if type is not reserved for protocol', () => {
    const type = Identity.MAX_RESERVED_TYPE + 1;

    const result = validateIdentityType(type);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
