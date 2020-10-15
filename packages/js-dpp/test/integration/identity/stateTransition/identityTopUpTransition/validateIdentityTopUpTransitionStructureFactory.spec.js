const createAjv = require('../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../lib/validation/JsonSchemaValidator');

const getIdentityTopUpTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityTopUpTransitionFixture');

const validateIdentityTopUpTransitionStructureFactory = require(
  '../../../../../lib/identity/stateTransitions/identityTopUpTransition/validateIdentityTopUpTransitionStructureFactory',
);

const {
  expectJsonSchemaError,
} = require('../../../../../lib/test/expect/expectError');

describe('validateIdentityTopUpTransitionStructureFactory', () => {
  let rawStateTransition;
  let stateTransition;
  let validateIdentityTopUpTransitionStructure;

  beforeEach(() => {
    const ajv = createAjv();
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateIdentityTopUpTransitionStructure = validateIdentityTopUpTransitionStructureFactory(
      jsonSchemaValidator,
    );

    stateTransition = getIdentityTopUpTransitionFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    stateTransition.signByPrivateKey(privateKey);

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal to 3', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(3);
    });
  });

  describe('lockedOutPoint', () => {
    it('should be present', async () => {
      delete rawStateTransition.lockedOutPoint;

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('lockedOutPoint');
      expect(error.keyword).to.equal('required');
    });

    it('should be a byte array', async () => {
      rawStateTransition.lockedOutPoint = new Array(36).fill('string');

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('.lockedOutPoint[0]');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should not be less than 36 bytes', async () => {
      rawStateTransition.lockedOutPoint = Buffer.alloc(35);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minItems');
      expect(error.dataPath).to.equal('.lockedOutPoint');
    });

    it('should not be more than 36 bytes', async () => {
      rawStateTransition.lockedOutPoint = Buffer.alloc(37);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maxItems');
      expect(error.dataPath).to.equal('.lockedOutPoint');
    });
  });

  describe('identityId', () => {
    it('should be present', async () => {
      delete rawStateTransition.identityId;

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('identityId');
    });

    it('should be a byte array', async () => {
      rawStateTransition.identityId = new Array(32).fill('string');

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('.identityId[0]');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be no less than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(31);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.identityId');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be no longer than 32 bytes', async () => {
      rawStateTransition.identityId = Buffer.alloc(33);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.identityId');
      expect(error.keyword).to.equal('maxItems');
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = new Array(65).fill('string');

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result, 2);

      const [error, byteArrayError] = result.getErrors();

      expect(error.dataPath).to.equal('.signature[0]');
      expect(error.keyword).to.equal('type');

      expect(byteArrayError.keyword).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('minItems');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('maxItems');
    });
  });

  it('should return valid result', () => {
    const result = validateIdentityTopUpTransitionStructure(rawStateTransition);

    expect(result.isValid()).to.be.true();
  });
});
