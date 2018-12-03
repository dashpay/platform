const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');
const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDapObjectFactory = require('../../../lib/dapObject/validateDapObjectFactory');
const validateDapContractFactory = require('../../../lib/dapContract/validateDapContractFactory');
const enrichDapContractWithBaseDapObject = require('../../../lib/dapObject/enrichDapContractWithBaseDapObject');

const DapContractFactory = require('../../../lib/dapContract/DapContractFactory');

const MissingDapObjectTypeError = require('../../../lib/consensusErrors/MissingDapObjectTypeError');
const InvalidDapObjectTypeError = require('../../../lib/consensusErrors/InvalidDapObjectTypeError');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

const {
  expectValidationError,
  expectJsonSchemaError,
} = require('../../../lib/test/expect/expectError');

describe('validateDapObject', () => {
  let dapContract;
  let rawDapObjects;
  let rawDapObject;
  let validateDapObject;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);
    const validateDapContract = validateDapContractFactory(validator);
    const dapContractFactory = new DapContractFactory(validateDapContract);
    dapContract = dapContractFactory.createFromObject(getLovelyDapContract());

    validateDapObject = validateDapObjectFactory(
      validator,
      enrichDapContractWithBaseDapObject,
    );

    rawDapObjects = getLovelyDapObjects();
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
      });

      it('should be string');

      it('should be defined in Dap Contract', () => {
        rawDapObject.$type = 'undefinedObject';

        const result = validateDapObject(rawDapObject, dapContract);

        expectValidationError(
          result,
          InvalidDapObjectTypeError,
        );

        const [error] = result.getErrors();

        expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
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

      it('should be a number');
      it('should be 0, 1 or 2');
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

      it('should be a number');
      it('should be an integer');
      it('should be greater or equal to zero');
    });

    describe('$scope', () => {
      it('should be present');
      it('should be a string');
      it('should be 64 chars long');
    });

    describe('$scopeId', () => {
      it('should be present');
      it('should be a string');
      it('should be 64 chars long');
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
