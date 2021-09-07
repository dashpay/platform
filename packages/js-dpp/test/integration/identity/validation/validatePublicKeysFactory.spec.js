const { default: getRE2Class } = require('@dashevo/re2-wasm');

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
  '../../../../lib/errors/consensus/basic/identity/DuplicatedIdentityPublicKeyError',
);
const DuplicatedIdentityPublicKeyIdError = require(
  '../../../../lib/errors/consensus/basic/identity/DuplicatedIdentityPublicKeyIdError',
);

const InvalidIdentityPublicKeyDataError = require(
  '../../../../lib/errors/consensus/basic/identity/InvalidIdentityPublicKeyDataError',
);

describe('validatePublicKeysFactory', () => {
  let rawPublicKeys;
  let validatePublicKeys;

  beforeEach(async () => {
    ({ publicKeys: rawPublicKeys } = getIdentityFixture().toObject());

    const RE2 = await getRE2Class();
    const ajv = createAjv(RE2);

    const validator = new JsonSchemaValidator(ajv);

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

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('id');
    });

    it('should be a number', () => {
      rawPublicKeys[1].id = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be an integer', () => {
      rawPublicKeys[1].id = 1.1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be greater or equal to one', () => {
      rawPublicKeys[1].id = -1;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id');
      expect(error.getKeyword()).to.equal('minimum');
    });
  });

  describe('type', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].type;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/data');
      expect(error.getKeyword()).to.equal('minItems');
    });

    it('should be a number', () => {
      rawPublicKeys[1].type = 'string';

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/type');
      expect(error.getKeyword()).to.equal('type');
    });
  });

  describe('data', () => {
    it('should be present', () => {
      delete rawPublicKeys[1].data;

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('data');
    });

    it('should be a byte array', () => {
      rawPublicKeys[1].data = new Array(33).fill('string');

      const result = validatePublicKeys(rawPublicKeys);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/data/0');
      expect(error.getKeyword()).to.equal('type');

      expect(byteArrayError.getKeyword()).to.equal('byteArray');
    });

    describe('ECDSA_SECP256K1', () => {
      it('should be no less than 33 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(32);

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 33 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(34);

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });

    describe('BLS12_381', () => {
      it('should be no less than 48 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(47);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('minItems');
      });

      it('should be no longer than 48 bytes', () => {
        rawPublicKeys[1].data = Buffer.alloc(49);
        rawPublicKeys[1].type = 1;

        const result = validatePublicKeys(rawPublicKeys);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.getInstancePath()).to.equal('/data');
        expect(error.getKeyword()).to.equal('maxItems');
      });
    });
  });

  it('should return invalid result if there are duplicate key ids', () => {
    rawPublicKeys[1].id = rawPublicKeys[0].id;

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyIdError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1030);
    expect(error.getDuplicatedIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if there are duplicate keys', () => {
    rawPublicKeys[1].data = rawPublicKeys[0].data;

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, DuplicatedIdentityPublicKeyError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1029);
    expect(error.getDuplicatedPublicKeysIds()).to.deep.equal([rawPublicKeys[1].id]);
  });

  it('should return invalid result if key data is not a valid DER', () => {
    rawPublicKeys[1].data = Buffer.alloc(33);

    const result = validatePublicKeys(rawPublicKeys);

    expectValidationError(result, InvalidIdentityPublicKeyDataError);

    const [error] = result.getErrors();

    expect(error.getCode()).to.equal(1040);
    expect(error.getPublicKeyId()).to.deep.equal(rawPublicKeys[1].id);
    expect(error.getValidationError()).to.be.instanceOf(TypeError);
    expect(error.getValidationError().message).to.equal('Invalid DER format public key');
  });

  it('should pass valid public keys', () => {
    const result = validatePublicKeys(rawPublicKeys);

    expect(result.isValid()).to.be.true();
  });
});
