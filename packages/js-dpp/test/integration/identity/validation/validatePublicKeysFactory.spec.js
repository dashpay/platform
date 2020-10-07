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
  let rawPublicKeys;
  let validatePublicKeys;

  beforeEach(() => {
    ({ publicKeys: rawPublicKeys } = getIdentityFixture().toObject());

    const validator = new JsonSchemaValidator(new Ajv());

    validatePublicKeys = validatePublicKeysFactory(
      validator,
    );
  });

  describe('id', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].id;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('id');
    });

    it('should be a number', () => {
      rawPublicKeys[1].id = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('type');
    });

    it('should be an integer', () => {
      rawPublicKeys[1].id = 1.1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('type');
    });

    it('should be greater or equal to one', () => {
      rawPublicKeys[1].id = -1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.id');
      expect(error.keyword).to.equal('minimum');
    });
  });

  describe('type', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].type;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be a number', () => {
      rawPublicKeys[1].type = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('type');
    });
  });

  describe('data', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].data;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('data');
    });

    it('should be a binary (encoded string)', () => {
      rawPublicKeys[1].data = 1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.data');
      expect(error.keyword).to.equal('type');
    });

    it('should be base64 encoded string without padding', () => {
      rawPublicKeys[1].data = '&'.repeat(44);

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.data');
      expect(error.keyword).to.equal('pattern');
    });

    describe('ECDSA_SECP256K1', () => {
      it('should be no less than 44 character', () => {
        rawPublicKeys[1].data = Buffer.alloc(33).toString('base64').slice(1);

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 44 character', () => {
        rawPublicKeys[1].data = `${Buffer.alloc(33).toString('base64')}a`;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('maxLength');
      });
    });

    describe('BLS12_381', () => {
      it('should be no less than 64 character', () => {
        rawPublicKeys[1].data = Buffer.alloc(48).toString('base64').slice(1);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 64 character', () => {
        rawPublicKeys[1].data = `${Buffer.alloc(48).toString('base64')}a`;
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.data');
        expect(error.keyword).to.equal('maxLength');
      });
    });
  });

  it('should return invalid result if there are duplicate key ids', () => {
    rawPublicKeys[1].id = rawPublicKeys[0].id;

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyIdError);

    const [error] = result.getErrors();

    expect(error.getRawPublicKeys()).to.equal(rawPublicKeys);
  });

  it('should return invalid result if there are duplicate keys', () => {
    rawPublicKeys[1].data = rawPublicKeys[0].data;

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyError);

    const [error] = result.getErrors();

    expect(error.getRawPublicKeys()).to.equal(rawPublicKeys);
  });

  it('should return invalid result if key data is not a valid DER', () => {
    rawPublicKeys[1].data = Buffer.alloc(33).toString('base64');

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, InvalidIdentityPublicKeyDataError);

    const [error] = result.getErrors();

    expect(error.getPublicKey()).to.deep.equal(rawPublicKeys[1]);
    expect(error.getValidationError()).to.be.an.instanceOf(TypeError);
  });

  it('should pass valid public keys', () => {
    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });
});
