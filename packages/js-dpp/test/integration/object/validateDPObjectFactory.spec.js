const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../lib/validation/ValidationResult');

const DPObject = require('../../../lib/object/DPObject');
const validateDPObjectFactory = require('../../../lib/object/validateDPObjectFactory');
const enrichDPContractWithBaseDPObject = require('../../../lib/object/enrichDPContractWithBaseDPObject');

const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');
const getDPObjectsFixture = require('../../../lib/test/fixtures/getDPObjectsFixture');

const MissingDPObjectTypeError = require('../../../lib/errors/MissingDPObjectTypeError');
const MissingDPObjectActionError = require('../../../lib/errors/MissingDPObjectActionError');
const InvalidDPObjectTypeError = require('../../../lib/errors/InvalidDPObjectTypeError');
const InvalidDPObjectScopeIdError = require('../../../lib/errors/InvalidDPObjectScopeIdError');
const ConsensusError = require('../../../lib/errors/ConsensusError');
const JsonSchemaError = require('../../../lib/errors/JsonSchemaError');

const originalDPObjectBaseSchema = require('../../../schema/base/dp-object');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../lib/test/expect/expectError');

describe('validateDPObjectFactory', () => {
  let dpContract;
  let rawDPObjects;
  let rawDPObject;
  let validateDPObject;
  let validator;
  let dpObjectBaseSchema;

  beforeEach(function beforeEach() {
    const ajv = new Ajv();

    validator = new JsonSchemaValidator(ajv);
    this.sinonSandbox.spy(validator, 'validate');

    dpContract = getDPContractFixture();

    validateDPObject = validateDPObjectFactory(
      validator,
      enrichDPContractWithBaseDPObject,
    );

    rawDPObjects = getDPObjectsFixture().map(o => o.toJSON());
    [rawDPObject] = rawDPObjects;

    dpObjectBaseSchema = JSON.parse(
      JSON.stringify(originalDPObjectBaseSchema),
    );
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

        expect(error.getRawDPObject()).to.equal(rawDPObject);
      });

      it('should be defined in DP Contract', () => {
        rawDPObject.$type = 'undefinedObject';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(
          result,
          InvalidDPObjectTypeError,
        );

        const [error] = result.getErrors();

        expect(error.getType()).to.equal('undefinedObject');
      });

      it('should throw an error if getDPObjectSchemaRef throws error', function it() {
        const someError = new Error();

        this.sinonSandbox.stub(dpContract, 'getDPObjectSchemaRef').throws(someError);

        let error;
        try {
          validateDPObject(rawDPObject, dpContract);
        } catch (e) {
          error = e;
        }

        expect(error).to.equal(someError);

        expect(dpContract.getDPObjectSchemaRef).to.have.been.calledOnce();
      });
    });

    describe('$action', () => {
      it('should be present', () => {
        delete rawDPObject.$action;

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(
          result,
          MissingDPObjectActionError,
        );

        const [error] = result.getErrors();

        expect(error.getRawDPObject()).to.equal(rawDPObject);
      });

      it('should be a number', () => {
        rawDPObject.$action = 'string';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$action');
        expect(error.keyword).to.equal('type');
      });

      it('should be defined enum', () => {
        rawDPObject.$action = 3;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$action');
        expect(error.keyword).to.equal('enum');
      });
    });

    describe('$rev', () => {
      it('should return an error if $rev is not present', () => {
        delete rawDPObject.$rev;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$rev');
      });

      it('should be a number', () => {
        rawDPObject.$rev = 'string';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$rev');
        expect(error.keyword).to.equal('type');
      });

      it('should be an integer', () => {
        rawDPObject.$rev = 1.1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$rev');
        expect(error.keyword).to.equal('multipleOf');
      });

      it('should be greater or equal to zero', () => {
        rawDPObject.$rev = -1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$rev');
        expect(error.keyword).to.equal('minimum');
      });
    });

    describe('$scope', () => {
      it('should be present', () => {
        delete rawDPObject.$scope;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('');
        expect(error.keyword).to.equal('required');
        expect(error.params.missingProperty).to.equal('$scope');
      });

      it('should be a string', () => {
        rawDPObject.$scope = 1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$scope');
        expect(error.keyword).to.equal('type');
      });

      it('should be no less than 64 chars', () => {
        rawDPObject.$scope = '86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$scope');
        expect(error.keyword).to.equal('minLength');
      });

      it('should be no longer than 64 chars', () => {
        rawDPObject.$scope = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.equal('.$scope');
        expect(error.keyword).to.equal('maxLength');
      });
    });

    describe('$scopeId', () => {
      it('should be present', () => {
        delete rawDPObject.$scopeId;

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('');
        expect(jsonError.keyword).to.equal('required');
        expect(jsonError.params.missingProperty).to.equal('$scopeId');

        expect(scopeError).to.be.an.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.equal(rawDPObject);
      });

      it('should be a string', () => {
        rawDPObject.$scopeId = 1;

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('.$scopeId');
        expect(jsonError.keyword).to.equal('type');

        expect(scopeError).to.be.an.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.equal(rawDPObject);
      });

      it('should be no less than 34 chars', () => {
        rawDPObject.$scopeId = '86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('.$scopeId');
        expect(jsonError.keyword).to.equal('minLength');

        expect(scopeError).to.be.an.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.equal(rawDPObject);
      });

      it('should be no longer than 34 chars', () => {
        rawDPObject.$scopeId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, ConsensusError, 2);

        const [jsonError, scopeError] = result.getErrors();

        expect(jsonError).to.be.an.instanceOf(JsonSchemaError);
        expect(jsonError.dataPath).to.equal('.$scopeId');
        expect(jsonError.keyword).to.equal('maxLength');

        expect(scopeError).to.be.an.instanceOf(InvalidDPObjectScopeIdError);
        expect(scopeError.getRawDPObject()).to.equal(rawDPObject);
      });

      it('should be valid entropy', () => {
        rawDPObject.$scopeId = '86b273ff86b273ff86b273ff86b273ff86';

        const result = validateDPObject(rawDPObject, dpContract);

        expectValidationError(result, InvalidDPObjectScopeIdError);

        const [error] = result.getErrors();

        expect(error).to.be.an.instanceOf(InvalidDPObjectScopeIdError);
        expect(error.getRawDPObject()).to.equal(rawDPObject);
      });
    });
  });

  describe('DP Contract schema', () => {
    it('should return an error if the first object is not valid against DP Contract', () => {
      rawDPObjects[0].name = 1;

      const result = validateDPObject(rawDPObjects[0], dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('.name');
      expect(error.keyword).to.equal('type');
    });

    it('should return an error if the second object is not valid against DP Contract', () => {
      rawDPObjects[1].undefined = 1;

      const result = validateDPObject(rawDPObjects[1], dpContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.equal('');
      expect(error.keyword).to.equal('additionalProperties');
    });
  });

  it('should validate against base DP object schema if $action is DELETE', () => {
    delete rawDPObject.name;
    rawDPObject.$action = DPObject.ACTIONS.DELETE;

    const result = validateDPObject(rawDPObject, dpContract);

    expect(validator.validate).to.have.been.calledOnceWith(dpObjectBaseSchema, rawDPObject);
    expect(result.getErrors().length).to.equal(0);
  });

  it('should throw validation error if additional fields are defined and $action is DELETE', () => {
    rawDPObject.$action = DPObject.ACTIONS.DELETE;

    const result = validateDPObject(rawDPObject, dpContract);

    const [error] = result.getErrors();

    expect(error.dataPath).to.equal('');
    expect(error.keyword).to.equal('additionalProperties');
  });

  it('should return valid response is an object is valid', () => {
    const result = validateDPObject(rawDPObject, dpContract);

    expect(result).to.be.an.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
