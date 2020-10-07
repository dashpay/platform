const Ajv = require('ajv');

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
    const ajv = new Ajv();
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

    it('should be a binary (encoded string)', async () => {
      rawStateTransition.lockedOutPoint = 1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.lockedOutPoint');
      expect(error.keyword).to.equal('type');
      expect(error.params.type).to.equal('string');
    });

    it('should not be less than 36 bytes (48 characters) in length', async () => {
      rawStateTransition.lockedOutPoint = '1';

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minLength');
      expect(error.dataPath).to.equal('.lockedOutPoint');
    });

    it('should not be more than 36 bytes (48 characters) in length', async () => {
      rawStateTransition.lockedOutPoint = Buffer.alloc(48).toString('base64');

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maxLength');
      expect(error.dataPath).to.equal('.lockedOutPoint');
    });

    it('should be base64 encoded', async () => {
      rawStateTransition.lockedOutPoint = '&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&&';

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('pattern');
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

    it('should be a binary (encoded string)', async () => {
      rawStateTransition.identityId = 1;

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.identityId');
      expect(error.keyword).to.equal('type');
      expect(error.params.type).to.equal('string');
    });

    it('should be no less than 42 chars', async () => {
      rawStateTransition.identityId = '1'.repeat(41);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.identityId');
      expect(error.keyword).to.equal('minLength');
    });

    it('should be no longer than 44 chars', async () => {
      rawStateTransition.identityId = '1'.repeat(45);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.identityId');
      expect(error.keyword).to.equal('maxLength');
    });

    it('should be base58 encoded', async () => {
      rawStateTransition.identityId = '&'.repeat(44);

      const result = await validateIdentityTopUpTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('pattern');
      expect(error.dataPath).to.equal('.identityId');
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

    it('should be a binary (encoded string)', async () => {
      rawStateTransition.signature = 1;

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('type');
      expect(error.params.type).to.equal('string');
    });

    it('should have length of 65 bytes (87 chars)', async () => {
      rawStateTransition.signature = Buffer.alloc(10);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('minLength');
      expect(error.params.limit).to.equal(87);
    });

    it('should be base64 encoded', async () => {
      rawStateTransition.signature = '&'.repeat(87);

      const result = await validateIdentityTopUpTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('pattern');
    });
  });

  it('should return valid result', () => {
    const result = validateIdentityTopUpTransitionStructure(rawStateTransition);

    expect(result.isValid()).to.be.true();
  });
});
