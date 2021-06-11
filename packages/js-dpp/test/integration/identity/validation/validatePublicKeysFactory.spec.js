const createAjv = require('../../../../lib/ajv/createAjv');

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

  beforeEach(async () => {
    ({ publicKeys: rawPublicKeys } = getIdentityFixture().toObject());

    const validator = new JsonSchemaValidator(await createAjv());

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

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('id');
    });

    it('should be a number', () => {
      rawPublicKeys[1].id = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/id');
      expect(error.keyword).to.equal('type');
    });

    it('should be an integer', () => {
      rawPublicKeys[1].id = 1.1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/id');
      expect(error.keyword).to.equal('type');
    });

    it('should be greater or equal to one', () => {
      rawPublicKeys[1].id = -1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/id');
      expect(error.keyword).to.equal('minimum');
    });
  });

  describe('type', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].type;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/data');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be a number', () => {
      rawPublicKeys[1].type = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('/type');
      expect(error.keyword).to.equal('type');
    });
  });

  describe('data', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].data;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.instancePath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('data');
    });

    it('should be a byte array', () => {
      rawPublicKeys[1].data = new Array(33).fill('string');

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.instancePath).to.equal('/data/0');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    describe('ECDSA_SECP256K1', () => {
      it('should be no less than 33 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(32);

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/data');
        expect(error.keyword).to.equal('minItems');
      });

      it('should be no longer than 33 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(34);

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/data');
        expect(error.keyword).to.equal('maxItems');
      });
    });

    describe('BLS12_381', () => {
      it('should be no less than 48 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(47);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/data');
        expect(error.keyword).to.equal('minItems');
      });

      it('should be no longer than 48 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(49);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.instancePath).to.equal('/data');
        expect(error.keyword).to.equal('maxItems');
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
    rawPublicKeys[1].data = Buffer.alloc(33);

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
