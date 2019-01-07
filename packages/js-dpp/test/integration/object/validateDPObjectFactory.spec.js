const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDPObjectFactory = require('../../../lib/object/validateDPObjectFactory');
const enrichDPContractWithBaseDPObject = require('../../../lib/object/enrichDPContractWithBaseDPObject');

const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');
const getDPObjectsFixture = require('../../../lib/test/fixtures/getDPObjectsFixture');

const MissingDPObjectTypeError = require('../../../lib/errors/MissingDPObjectTypeError');
const InvalidDPObjectTypeError = require('../../../lib/errors/InvalidDPObjectTypeError');
const InvalidDPObjectScopeIdError = require('../../../lib/errors/InvalidDPObjectScopeIdError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const JsonSchemaError = require('../../../lib/errors/JsonSchemaError');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../lib/test/expect/expectError');

describe('validateDPObjectFactory', () => {
  let dpContract;
  let rawDPObjects;
  let rawDPObject;
  let validateDPObject;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    dpContract = getDPContractFixture();

    validateDPObject = validateDPObjectFactory(
      validator,
      enrichDPContractWithBaseDPObject,
    );

    rawDPObjects = getDPObjectsFixture().map(o => o.toJSON());
    [rawDPObject] = rawDPObjects;
  });

  describe('Base schema', () => {
    describe('$type', () => {
      it('should be present', () => {
        delete rawDPObject.$type;

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(
          result,
          MissingDPObjectTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getRawDPObject()).to.be.equal(rawDPObject);
      });

      it('should be defined in DP Contract', () => {
        rawDPObject.$type = 'undefinedObject';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(
          result,
          InvalidDPObjectTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getType()).to.be.equal('undefinedObject');
      });

      it('should throw error if getDPObjectSchemaRef throws error', function it() {
        const someError = new Error();

        this.sinonSandbox.stub(dpContract, 'getDPObjectSchemaRef').throws(someError);

        let error;
        try {
          validateDPObject(rawDPObject, dpContract);
        } catch (e) {
          error = e;
        }

        expect(error).to.be.equal(someError);

        expect(dpContract.getDPObjectSchemaRef).to.be.calledOnce();
      });
    });

    describe('$action', () => {
      it('should be present', () => {
        delete rawDPObject.$action;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('$action');
      });

      it('should be a number', () => {
        rawDPObject.$action = 'string';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$action');
        expect(error.keyword).to.be.equal('type');
      });

      it('should be defined enum', () => {
        rawDPObject.$action = 3;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$action');
        expect(error.keyword).to.be.equal('enum');
      });
    });

    describe('$rev', () => {
      it('should return error if $rev is not present', () => {
        delete rawDPObject.$rev;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('$rev');
      });

      it('should be a number', () => {
        rawDPObject.$rev = 'string';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$rev');
        expect(error.keyword).to.be.equal('type');
      });

      it('should be an integer', () => {
        rawDPObject.$rev = 1.1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$rev');
        expect(error.keyword).to.be.equal('multipleOf');
      });

      it('should be greater or equal to zero', () => {
        rawDPObject.$rev = -1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$rev');
        expect(error.keyword).to.be.equal('minimum');
      });
    });

    describe('$scope', () => {
      it('should be present', () => {
        delete rawDPObject.$scope;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('$scope');
      });

      it('should be a string', () => {
        rawDPObject.$scope = 1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$scope');
        expect(error.keyword).to.be.equal('type');
      });

      it('should not be less than 64 chars', () => {
        rawDPObject.$scope = '86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$scope');
        expect(error.keyword).to.be.equal('minLength');
      });

      it('should not be longer than 64 chars', () => {
        rawDPObject.$scope = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.$scope');
        expect(error.keyword).to.be.equal('maxLength');
      });
    });

    describe('$scopeId', () => {
      it('should be present', () => {
        delete rawDPObject.$scopeId;

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('');
        expect(jsonError.keyword).to.be.equal('required');
        expect(jsonError.params.missingProperty).to.be.equal('$scopeId');

        expect(scopeError).to.be.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.be.equal(rawDPObject);
      });

      it('should be a string', () => {
        rawDPObject.$scopeId = 1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('.$scopeId');
        expect(jsonError.keyword).to.be.equal('type');

        expect(scopeError).to.be.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.be.equal(rawDPObject);
      });

      it('should not be less than 34 chars', () => {
        rawDPObject.$scopeId = '86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('.$scopeId');
        expect(jsonError.keyword).to.be.equal('minLength');

        expect(scopeError).to.be.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.be.equal(rawDPObject);
      });

      it('should not be longer than 34 chars', () => {
        rawDPObject.$scopeId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.be.equal('.$scopeId');
        expect(jsonError.keyword).to.be.equal('maxLength');

        expect(scopeError).to.be.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.be.equal(rawDPObject);
      });

      it('should be valid entropy', () => {
        rawDPObject.$scopeId = '86b273ff86b273ff86b273ff86b273ff86';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, InvalidDPObjectScopeIdError);

        const [error] = result.getErrors();

        expect(error).to.be.instanceOf(InvalidDPObjectScopeIdError);
        expect(error.getRawDPObject()).to.be.equal(rawDPObject);
      });
    });
  });

  describe('DP Contract schema', () => {
    it('should return error if the first object is not valid against DP Contract', () => {
      rawDPObjects[0].name = 1;

      const result = validateDPObject(rawDPObjects[0], dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('type');
    });

    it('should return error if the second object is not valid against DP Contract', () => {
      rawDPObjects[1].undefined = 1;

      const result = validateDPObject(rawDPObjects[1], dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('additionalProperties');
    });
  });

  it('should return valid response is an object is valid', () => {
    const result = validateDPObject(rawDPObject, dpContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
