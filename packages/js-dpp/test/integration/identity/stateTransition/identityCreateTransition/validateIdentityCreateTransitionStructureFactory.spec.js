const createAjv = require('../../../../../lib/ajv/createAjv');

const JsonSchemaValidator = require('../../../../../lib/validation/JsonSchemaValidator');

const getIdentityCreateTransitionFixture = require('../../../../../lib/test/fixtures/getIdentityCreateTransitionFixture');

const validateIdentityCreateTransitionStructureFactory = require(
  '../../../../../lib/identity/stateTransitions/identityCreateTransition/validateIdentityCreateTransitionStructureFactory',
);

const {
  expectJsonSchemaError,
  expectValidationError,
} = require('../../../../../lib/test/expect/expectError');

const ValidationResult = require('../../../../../lib/validation/ValidationResult');
const ConsensusError = require('../../../../../lib/errors/ConsensusError');

describe('validateIdentityCreateTransitionStructureFactory', () => {
  let validateIdentityCreateTransitionStructure;
  let rawStateTransition;
  let stateTransition;
  let validatePublicKeysMock;

  beforeEach(function beforeEach() {
    validatePublicKeysMock = this.sinonSandbox.stub().returns(new ValidationResult());

    const ajv = createAjv();
    const jsonSchemaValidator = new JsonSchemaValidator(ajv);

    validateIdentityCreateTransitionStructure = validateIdentityCreateTransitionStructureFactory(
      jsonSchemaValidator,
      validatePublicKeysMock,
    );

    stateTransition = getIdentityCreateTransitionFixture();

    const privateKey = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2';

    stateTransition.signByPrivateKey(privateKey);

    rawStateTransition = stateTransition.toObject();
  });

  describe('protocolVersion', () => {
    it('should be present', async () => {
      delete rawStateTransition.protocolVersion;

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('protocolVersion');
    });

    it('should be an integer', async () => {
      rawStateTransition.protocolVersion = '1';

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.protocolVersion');
      expect(error.keyword).to.equal('type');
    });

    it('should not be less than 0', async () => {
      rawStateTransition.protocolVersion = -1;

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minimum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });

    it('should not be greater than current version (0)', async () => {
      rawStateTransition.protocolVersion = 1;

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maximum');
      expect(error.dataPath).to.equal('.protocolVersion');
    });
  });

  describe('type', () => {
    it('should be present', async () => {
      delete rawStateTransition.type;

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('type');
    });

    it('should be equal to 2', async () => {
      rawStateTransition.type = 666;

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.type');
      expect(error.keyword).to.equal('const');
      expect(error.params.allowedValue).to.equal(2);
    });
  });

  describe('lockedOutPoint', () => {
    it('should be present', async () => {
      delete rawStateTransition.lockedOutPoint;

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('lockedOutPoint');
      expect(error.keyword).to.equal('required');
    });

    it('should be a byte array', async () => {
      rawStateTransition.lockedOutPoint = {};

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.lockedOutPoint');
      expect(error.keyword).to.equal('byteArray');
    });

    it('should not be less than 36 bytes', async () => {
      rawStateTransition.lockedOutPoint = Buffer.alloc(35);

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minBytesLength');
      expect(error.dataPath).to.equal('.lockedOutPoint');
    });

    it('should not be more than 36 bytes', async () => {
      rawStateTransition.lockedOutPoint = Buffer.alloc(37);

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maxBytesLength');
      expect(error.dataPath).to.equal('.lockedOutPoint');
    });
  });

  describe('publicKeys', () => {
    it('should be present', async () => {
      rawStateTransition.publicKeys = undefined;

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.params.missingProperty).to.equal('publicKeys');
      expect(error.keyword).to.equal('required');
    });

    it('should not be empty', async () => {
      rawStateTransition.publicKeys = [];

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('minItems');
      expect(error.dataPath).to.equal('.publicKeys');
    });

    it('should not have more than 10 items', async () => {
      const [key] = rawStateTransition.publicKeys;

      for (let i = 0; i < 10; i++) {
        rawStateTransition.publicKeys.push(key);
      }

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('maxItems');
      expect(error.dataPath).to.equal('.publicKeys');
    });

    it('should be unique', async () => {
      rawStateTransition.publicKeys.push(rawStateTransition.publicKeys[0]);

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.equal('uniqueItems');
      expect(error.dataPath).to.equal('.publicKeys');
    });

    it('should be valid', async () => {
      const publicKeysError = new ConsensusError('test');
      const publicKeysResult = new ValidationResult([
        publicKeysError,
      ]);

      validatePublicKeysMock.returns(publicKeysResult);

      const result = await validateIdentityCreateTransitionStructure(
        rawStateTransition,
      );

      expectValidationError(result);

      const [error] = result.getErrors();

      expect(error).to.equal(publicKeysError);
    });
  });

  describe('signature', () => {
    it('should be present', async () => {
      delete rawStateTransition.signature;

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('required');
      expect(error.params.missingProperty).to.equal('signature');
    });

    it('should be a byte array', async () => {
      rawStateTransition.signature = { };

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('byteArray');
    });

    it('should be not shorter than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(64);

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('minBytesLength');
    });

    it('should be not longer than 65 bytes', async () => {
      rawStateTransition.signature = Buffer.alloc(66);

      const result = await validateIdentityCreateTransitionStructure(rawStateTransition);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.signature');
      expect(error.keyword).to.equal('maxBytesLength');
    });
  });

  it('should return valid result', () => {
    const result = validateIdentityCreateTransitionStructure(rawStateTransition);

    expect(result.isValid()).to.be.true();

    expect(validatePublicKeysMock).to.be.calledOnceWithExactly(
      rawStateTransition.publicKeys,
    );
  });
});
