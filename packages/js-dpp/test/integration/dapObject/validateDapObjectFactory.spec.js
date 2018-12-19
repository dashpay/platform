const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDapObjectFactory = require('../../../lib/dapObject/validateDapObjectFactory');
const enrichDapContractWithBaseDapObject = require('../../../lib/dapObject/enrichDapContractWithBaseDapObject');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');
const getDapObjectsFixture = require('../../../lib/test/fixtures/getDapObjectsFixture');

const MissingDapObjectTypeError = require('../../../lib/errors/MissingDapObjectTypeError');
const InvalidDapObjectTypeError = require('../../../lib/errors/InvalidDapObjectTypeError');
const InvalidDapObjectScopeIdError = require('../../../lib/errors/InvalidDapObjectScopeIdError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const JsonSchemaError = require('../../../lib/errors/JsonSchemaError');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../lib/test/expect/expectError');

describe('validateDapObjectFactory', () => {
  let dapContract;
  let rawDapObjects;
  let rawDapObject;
  let validateDapObject;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    dapContract = getDapContractFixture();

    validateDapObject = validateDapObjectFactory(
      validator,
      enrichDapContractWithBaseDapObject,
    );

    rawDapObjects = getDapObjectsFixture().map(o => o.toJSON());
    [rawDapObject] = rawDapObjects;
  });

  describe('Base schema', () => {
    describe('$type', () => {
      it('should be present', () => {
        delete rawDapObject.$type;

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(
          result,
          MissingDapObjectTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawDapObject()).to.be.equal(rawDapObject);
      });

      it('should be defined in Dap Contract', () => {
        rawDapObject.$type = 'undefinedObject';

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(
          result,
          InvalidDapObjectTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getType()).to.be.equal('undefinedObject');
      });
    });

    describe('$action', () => {
      it('should be present', () => {
        delete rawDapObject.$action;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('$action');
      });

      it('should be a number', () => {
        rawDapObject.$action = 'string';

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$action');
        expect(error.keyword).to.be.equal('type');
      });

      it('should be defined enum', () => {
        rawDapObject.$action = 3;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$action');
        expect(error.keyword).to.be.equal('enum');
      });
    });

    describe('$rev', () => {
      it('should return error if $rev is not present', () => {
        delete rawDapObject.$rev;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('$rev');
      });

      it('should be a number', () => {
        rawDapObject.$rev = 'string';

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$rev');
        expect(error.keyword).to.be.equal('type');
      });

      it('should be an integer', () => {
        rawDapObject.$rev = 1.1;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$rev');
        expect(error.keyword).to.be.equal('multipleOf');
      });

      it('should be greater or equal to zero', () => {
        rawDapObject.$rev = -1;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$rev');
        expect(error.keyword).to.be.equal('minimum');
      });
    });

    describe('$scope', () => {
      it('should be present', () => {
        delete rawDapObject.$scope;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('$scope');
      });

      it('should be a string', () => {
        rawDapObject.$scope = 1;

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$scope');
        expect(error.keyword).to.be.equal('type');
      });

      it('should not be less than 64 chars', () => {
        rawDapObject.$scope = '86b273ff';

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$scope');
        expect(error.keyword).to.be.equal('minLength');
      });

      it('should not be longer than 64 chars', () => {
        rawDapObject.$scope = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDapObject(rawDapObject, dapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$scope');
        expect(error.keyword).to.be.equal('maxLength');
      });
    });

    describe('$scopeId', () => {
      it('should be present', () => {
        delete rawDapObject.$scopeId;

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('');
        expect(jsonError.keyword).to.be.equal('required');
        expect(jsonError.params.missingProperty).to.be.equal('$scopeId');

        expect(scopeError).to.be.instanceOf(InvalidDapObjectScopeIdError);
        expect(scopeError.getRawDapObject()).to.be.equal(rawDapObject);
      });

      it('should be a string', () => {
        rawDapObject.$scopeId = 1;

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('.$scopeId');
        expect(jsonError.keyword).to.be.equal('type');

        expect(scopeError).to.be.instanceOf(InvalidDapObjectScopeIdError);
        expect(scopeError.getRawDapObject()).to.be.equal(rawDapObject);
      });

      it('should not be less than 34 chars', () => {
        rawDapObject.$scopeId = '86b273ff';

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('.$scopeId');
        expect(jsonError.keyword).to.be.equal('minLength');

        expect(scopeError).to.be.instanceOf(InvalidDapObjectScopeIdError);
        expect(scopeError.getRawDapObject()).to.be.equal(rawDapObject);
      });

      it('should not be longer than 34 chars', () => {
        rawDapObject.$scopeId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('.$scopeId');
        expect(jsonError.keyword).to.be.equal('maxLength');

        expect(scopeError).to.be.instanceOf(InvalidDapObjectScopeIdError);
        expect(scopeError.getRawDapObject()).to.be.equal(rawDapObject);
      });

      it('should be valid entropy', () => {
        rawDapObject.$scopeId = '86b273ff86b273ff86b273ff86b273ff86';

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(result, InvalidDapObjectScopeIdError);

        const [error] = result.getErrors();

        expect(error).to.be.instanceOf(InvalidDapObjectScopeIdError);
        expect(error.getRawDapObject()).to.be.equal(rawDapObject);
      });
    });
  });

  describe('Dap Contract schema', () => {
    it('should return error if the first object is not valid against Dap Contract', () => {
      rawDapObjects[0].name = 1;

      const result = validateDapObject(rawDapObjects[0], dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('type');
    });

    it('should return error if the second object is not valid against Dap Contract', () => {
      rawDapObjects[1].undefined = 1;

      const result = validateDapObject(rawDapObjects[1], dapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('additionalProperties');
    });
  });

  it('should return valid response is an object is valid', () => {
    const result = validateDapObject(rawDapObject, dapContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
