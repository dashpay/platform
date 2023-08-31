const getIdentityFixture = require('../../../../lib/test/fixtures/getIdentityFixture');
const { expectValidationError, expectJsonSchemaError } = require('../../../../lib/test/expect/expectError');

const { default: loadWasmDpp } = require('../../../../dist');
const getBlsAdapterMock = require('../../../../lib/test/mocks/getBlsAdapterMock');
const generateRandomIdentifierAsync = require('../../../../lib/test/utils/generateRandomIdentifierAsync');

describe.skip('validateIdentityFactory', () => {
  let rawIdentity;
  let validateIdentity;
  let identity;

  let IdentityValidator;
  let Identity;
  let UnsupportedProtocolVersionError;

  before(async () => {
    ({ IdentityValidator, Identity, UnsupportedProtocolVersionError } = await loadWasmDpp());
  });

  beforeEach(async function beforeEach() {
    this.timeout(20000);
    const blsAdapter = await getBlsAdapterMock();

    const validator = new IdentityValidator(blsAdapter);

    validateIdentity = (value) => validator.validate(value);

    const identityObject = (await getIdentityFixture()).toObject();
    identityObject.id = await generateRandomIdentifierAsync();
    identity = new Identity(identityObject);

    rawIdentity = identity.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawIdentity.protocolVersion;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getKeyword()).to.equal('required');
      expect(error.getParams().missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawIdentity.protocolVersion = '1';

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/protocolVersion');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should be valid', async () => {
      rawIdentity.protocolVersion = 100;

      const result = validateIdentity(rawIdentity);

      await expectValidationError(result, UnsupportedProtocolVersionError);
    });
  });

  describe('id', () => {
    it('should be present', async () => {
      rawIdentity.id = undefined;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('id');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be a byte array', async () => {
      rawIdentity.id = new Array(32).fill('string');

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result, 32);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('/id/0');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be less than 32 bytes', async () => {
      rawIdentity.id = Buffer.alloc(31);

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getInstancePath()).to.equal('/id');
    });

    it('should not be more than 32 bytes', async () => {
      rawIdentity.id = Buffer.alloc(33);

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/id');
    });
  });

  describe('balance', () => {
    it('should be present', async () => {
      rawIdentity.balance = undefined;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('balance');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an integer', async () => {
      rawIdentity.balance = 1.2;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('type');
      expect(error.getInstancePath()).to.equal('/balance');
    });

    it('should be greater or equal 0', async () => {
      rawIdentity.balance = -1;

      let result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minimum');
      expect(error.getInstancePath()).to.equal('/balance');

      rawIdentity.balance = 0;

      result = validateIdentity(rawIdentity);

      expect(result.isValid()).to.be.true();
    });
  });

  describe('publicKeys', () => {
    it('should be present', async () => {
      rawIdentity.publicKeys = undefined;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('publicKeys');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an array', async () => {
      rawIdentity.publicKeys = 1;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getCode()).to.equal(1005);
      expect(error.getInstancePath()).to.equal('/publicKeys');
      expect(error.getKeyword()).to.equal('type');
    });

    it('should not be empty', async () => {
      rawIdentity.publicKeys = [];

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should be unique', async () => {
      rawIdentity.publicKeys.push(rawIdentity.publicKeys[0]);

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('uniqueItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });

    it('should throw an error if publicKeys have more than 100 keys', async () => {
      const [key] = rawIdentity.publicKeys;

      rawIdentity.publicKeys = [];
      for (let i = 0; i < 101; i++) {
        rawIdentity.publicKeys.push({ ...key, id: i });
      }

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('maxItems');
      expect(error.getInstancePath()).to.equal('/publicKeys');
    });
  });

  describe('revision', () => {
    it('should be present', async () => {
      rawIdentity.revision = undefined;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getInstancePath()).to.equal('');
      expect(error.getParams().missingProperty).to.equal('revision');
      expect(error.getKeyword()).to.equal('required');
    });

    it('should be an integer', async () => {
      rawIdentity.revision = 1.2;

      const result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('type');
      expect(error.getInstancePath()).to.equal('/revision');
    });

    it('should be greater or equal 0', async () => {
      rawIdentity.revision = -1;

      let result = validateIdentity(rawIdentity);

      await expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.getKeyword()).to.equal('minimum');
      expect(error.getInstancePath()).to.equal('/revision');

      rawIdentity.revision = 0;

      result = validateIdentity(rawIdentity);

      expect(result.isValid()).to.be.true();
    });
  });

  it('should return valid result if a raw identity is valid', () => {
    const result = validateIdentity(rawIdentity);

    expect(result.isValid()).to.be.true();
  });
});
