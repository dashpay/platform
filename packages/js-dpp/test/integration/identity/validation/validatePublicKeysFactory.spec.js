const Ajv = require('ajv');

const JsonSchemaValidator = require(
  '../../../../lib/validation/JsonSchemaValidator',
);

const validatePublicKeysFactory = require(
  '../../../../lib/identity/validation/validatePublicKeysFactory',
);

const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../../lib/test/expect/expectError');

const DuplicatedIdentityPublicKeyError = require(
  '../../../../lib/errors/DuplicatedIdentityPublicKeyError',
);
const DuplicatedIdentityPublicKeyIdError = require(
  '../../../../lib/errors/DuplicatedIdentityPublicKeyIdError',
);

const InvalidIdentityPublicKeyDataError = require(
  '../../../../lib/errors/InvalidIdentityPublicKeyDataError',
);

describe('validatePublicKeysFactory', () => {
  let publicKeys;
  let validatePublicKeys;

  beforeEach(() => {
    ({ publicKeys } = getIdentityFixture().toJSON());

    const validator = new JsonSchemaValidator(new Ajv());

    validatePublicKeys = validatePublicKeysFactory(
      validator,
    );
  });

  describe('id', () => {
    it('should be present', () => {
      delete publicKeys[1].id;

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('id');
    });

    it('should be a number', () => {
      publicKeys[1].id = 'string';

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('type');
    });

    it('should be an integer', () => {
      publicKeys[1].id = 1.1;

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('type');
    });

    it('should be greater or equal to one', () => {
      publicKeys[1].id = -1;

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('minimum');
    });
  });

  describe('type', () => {
    it('should be present', () => {
      delete publicKeys[1].type;

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be a number', () => {
      publicKeys[1].type = 'string';

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('type');
    });
  });

  describe('data', () => {
    it('should be present', () => {
      delete publicKeys[1].data;

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('data');
    });

    it('should be a string', () => {
      publicKeys[1].data = 1;

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.data');
      expect(error.keyword).to.equal('type');
    });

    it('should be base64 encoded string without padding', () => {
      publicKeys[1].data = '&'.repeat(44);

      const result = validatePublicKeys(publicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.data');
      expect(error.keyword).to.equal('pattern');
    });

    describe('ECDSA_SECP256K1', () => {
      it('should be no less than 44 character', () => {
        publicKeys[1].data = Buffer.alloc(33).toString('base64').slice(1);

        const result = validatePublicKeys(publicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 44 character', () => {
        publicKeys[1].data = `${Buffer.alloc(33).toString('base64')}a`;

        const result = validatePublicKeys(publicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('maxLength');
      });
    });

    describe('BLS12_381', () => {
      it('should be no less than 64 character', () => {
        publicKeys[1].data = Buffer.alloc(48).toString('base64').slice(1);
        publicKeys[1].type = 1;

        const result = validatePublicKeys(publicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 64 character', () => {
        publicKeys[1].data = `${Buffer.alloc(48).toString('base64')}a`;
        publicKeys[1].type = 1;

        const result = validatePublicKeys(publicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('maxLength');
      });
    });
  });

  it('should return invalid result if there are duplicate key ids', () => {
    publicKeys[1].id = publicKeys[0].id;

    const result = validatePublicKeys(publicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyIdError);

    const [error] = result.getErrors();

    expect(error.getRawPublicKeys()).to.equal(publicKeys);
  });

  it('should return invalid result if there are duplicate keys', () => {
    publicKeys[1].data = publicKeys[0].data;

    const result = validatePublicKeys(publicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyError);

    const [error] = result.getErrors();

    expect(error.getRawPublicKeys()).to.equal(publicKeys);
  });

  it('should return invalid result if key data is not a valid DER', () => {
    publicKeys[1].data = Buffer.alloc(33).toString('base64');

    const result = validatePublicKeys(publicKeys);

    expectValidationError(result, InvalidIdentityPublicKeyDataError);

    const [error] = result.getErrors();

    expect(error.getPublicKey()).to.deep.equal(publicKeys[1]);
    expect(error.getValidationError()).to.be.an.instanceOf(TypeError);
  });

  it('should pass valid public keys', () => {
    const result = validatePublicKeys(publicKeys);

    expect(result.isValid()).to.be.true();
  });
});
