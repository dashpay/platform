const Ajv = require('ajv');

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');

const JsonSchemaValidator = require(
  '../../../../lib/validation/JsonSchemaValidator',
);

const { expectValidationError, expectJsonSchemaError } = require(
  '../../../../lib/test/expect/expectError',
);

const validateIdentityFactory = require(
  '../../../../lib/identity/validation/validateIdentityFactory',
);

const Identity = require('../../../../lib/identity/Identity');

const JsonSchemaError = require(
  '../../../../lib/errors/JsonSchemaError',
);

const ConsensusError = require('../../../../lib/errors/ConsensusError');
const ValidationResult = require('../../../../lib/validation/ValidationResult');

describe('validateIdentityFactory', () => {
  let rawIdentity;
  let validateIdentity;
  let identity;
  let validateIdentityTypeMock;
  let validatePublicKeysMock;

  beforeEach(function beforeEach() {
    const schemaValidator = new JsonSchemaValidator(new Ajv());

    validateIdentityTypeMock = this.sinonSandbox.stub().returns(new ValidationResult());
    validatePublicKeysMock = this.sinonSandbox.stub().returns(new ValidationResult());

    validateIdentity = validateIdentityFactory(
      schemaValidator,
      validateIdentityTypeMock,
      validatePublicKeysMock,
    );

    identity = getIdentityFixture();

    rawIdentity = identity.toJSON();
  });

  describe('id', () => {
    it('should be present', () => {
      rawIdentity.id = undefined;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('id');
      expect(error.keyword).to.equal('required');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should be a string', () => {
      rawIdentity.id = 1;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('type');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should not be less than 42 characters', () => {
      rawIdentity.id = '1'.repeat(41);

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minLength');
      expect(error.dataPath).to.equal('.id');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should not be more than 44 characters', () => {
      rawIdentity.id = '1'.repeat(45);

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maxLength');
      expect(error.dataPath).to.equal('.id');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should be base58 encoded', () => {
      rawIdentity.id = '&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&';

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('pattern');
      expect(error.dataPath).to.equal('.id');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });
  });

  describe('type', () => {
    it('should be present', () => {
      rawIdentity.type = undefined;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('type');
      expect(error.keyword).to.equal('required');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should be an integer', () => {
      rawIdentity.type = 1.2;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('multipleOf');
      expect(error.dataPath).to.equal('.type');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should be greater than 0', () => {
      rawIdentity.type = -1;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('.type');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should be less than 65535', () => {
      rawIdentity.type = 77777;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('.type');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });
  });

  describe('publicKeys', () => {
    it('should be present', () => {
      rawIdentity.publicKeys = undefined;

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('publicKeys');
      expect(error.keyword).to.equal('required');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should be an array', () => {
      rawIdentity.publicKeys = 1;

      const result = validateIdentity(rawIdentity);

      expectValidationError(result, JsonSchemaError, 1);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.publicKeys');
      expect(error.keyword).to.equal('type');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should not be empty', () => {
      rawIdentity.publicKeys = [];

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minItems');
      expect(error.dataPath).to.equal('.publicKeys');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });

    it('should throw an error if publicKeys have more than 100 keys', () => {
      const [key] = rawIdentity.publicKeys;

      rawIdentity.publicKeys = [];
      for (let i = 0; i < 101; i++) {
        rawIdentity.publicKeys.push(key);
      }

      const result = validateIdentity(rawIdentity);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maxItems');
      expect(error.dataPath).to.equal('.publicKeys');

      expect(validateIdentityTypeMock).to.not.be.called();
      expect(validatePublicKeysMock).to.not.be.called();
    });
  });

  it('should return invalid result if there are duplicate keys', () => {
    const consensusError = new ConsensusError('error');

    validateIdentityTypeMock.returns(new ValidationResult([consensusError]));

    const result = validateIdentity(rawIdentity);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(consensusError);

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawIdentity.type);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(rawIdentity.publicKeys);
  });

  it('should return invalid result if identity type is unknown', () => {
    const consensusError = new ConsensusError('error');

    validatePublicKeysMock.returns(new ValidationResult([consensusError]));

    const result = validateIdentity(rawIdentity);

    expectValidationError(result);

    const [error] = result.getErrors();

    expect(error).to.equal(consensusError);

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawIdentity.type);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(rawIdentity.publicKeys);
  });

  it('should return valid result if a raw identity is valid', () => {
    const result = validateIdentity(rawIdentity);

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawIdentity.type);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(rawIdentity.publicKeys);

    expect(result.isValid()).to.be.true();
  });

  it('should return valid result if an identity model is valid', () => {
    const result = validateIdentity(new Identity(rawIdentity));

    expect(validateIdentityTypeMock).to.be.calledOnceWithExactly(rawIdentity.type);
    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(rawIdentity.publicKeys);

    expect(result.isValid()).to.be.true();
  });
});
