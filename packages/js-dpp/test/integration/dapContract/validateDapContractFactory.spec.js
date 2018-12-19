const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDapContractFactory = require('../../../lib/dapContract/validateDapContractFactory');

const getDapContractFixture = require('../../../lib/test/fixtures/getDapContractFixture');

const { expectJsonSchemaError } = require('../../../lib/test/expect/expectError');

describe('validateDapContractFactory', () => {
  let rawDapContract;
  let validateDapContract;

  beforeEach(() => {
    rawDapContract = getDapContractFixture().toJSON();

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateDapContract = validateDapContractFactory(validator);
  });

  describe('$schema', () => {
    it('should be present', () => {
      delete rawDapContract.$schema;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('$schema');
    });

    it('should be a string', () => {
      rawDapContract.$schema = 1;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.$schema');
      expect(error.keyword).to.be.equal('type');
    });

    it('should be particular url', () => {
      rawDapContract.$schema = 'wrong';

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.be.equal('const');
      expect(error.dataPath).to.be.equal('.$schema');
    });
  });

  describe('name', () => {
    it('should be present', () => {
      delete rawDapContract.name;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('name');
    });

    it('should be a string', () => {
      rawDapContract.name = 1;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('type');
    });

    it('should be greater or equal to 3', () => {
      rawDapContract.name = 'a'.repeat(2);

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should be less or equal to 24', () => {
      rawDapContract.name = 'a'.repeat(25);

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('maxLength');
    });

    it('should be an alphanumeric string', () => {
      rawDapContract.name = '*(*&^';

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('pattern');
    });
  });

  describe('version', () => {
    it('should be present', () => {
      delete rawDapContract.version;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('version');
    });

    it('should be a number', () => {
      rawDapContract.version = 'wrong';

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.version');
      expect(error.keyword).to.be.equal('type');
    });

    it('should be an integer', () => {
      rawDapContract.version = 1.2;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.version');
      expect(error.keyword).to.be.equal('multipleOf');
    });

    it('should be greater or equal to one', () => {
      rawDapContract.version = 0;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.version');
      expect(error.keyword).to.be.equal('minimum');
    });
  });

  describe('definitions', () => {
    it('may not be present', () => {
      delete rawDapContract.definitions;

      const result = validateDapContract(rawDapContract);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should be an object', () => {
      rawDapContract.definitions = 1;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.definitions');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be empty', () => {
      rawDapContract.definitions = {};

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.definitions');
      expect(error.keyword).to.be.equal('minProperties');
    });

    it('should have no a non-alphanumeric properties', () => {
      rawDapContract.definitions = {
        $subSchema: {},
      };

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result, 2);

      const [patternError, propertyNamesError] = result.getErrors();

      expect(patternError.dataPath).to.be.equal('.definitions');
      expect(patternError.keyword).to.be.equal('pattern');

      expect(propertyNamesError.dataPath).to.be.equal('.definitions');
      expect(propertyNamesError.keyword).to.be.equal('propertyNames');
    });

    it('should have no more than 100 properties', () => {
      rawDapContract.definitions = {};

      Array(101).fill({}).forEach((item, i) => {
        rawDapContract.definitions[i] = item;
      });

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.definitions');
      expect(error.keyword).to.be.equal('maxProperties');
    });
  });

  describe('dapObjectsDefinition', () => {
    it('should be present', () => {
      delete rawDapContract.dapObjectsDefinition;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('dapObjectsDefinition');
    });

    it('should be an object', () => {
      rawDapContract.dapObjectsDefinition = 1;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dapObjectsDefinition');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be empty', () => {
      rawDapContract.dapObjectsDefinition = {};

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dapObjectsDefinition');
      expect(error.keyword).to.be.equal('minProperties');
    });

    it('should have no a non-alphanumeric properties', () => {
      rawDapContract.dapObjectsDefinition['(*&^'] = rawDapContract.dapObjectsDefinition.niceObject;

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dapObjectsDefinition');
      expect(error.keyword).to.be.equal('additionalProperties');
    });

    it('should have no more than 100 properties', () => {
      const niceObjectDefinition = rawDapContract.dapObjectsDefinition.niceObject;

      rawDapContract.dapObjectsDefinition = {};

      Array(101).fill(niceObjectDefinition).forEach((item, i) => {
        rawDapContract.dapObjectsDefinition[i] = item;
      });

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dapObjectsDefinition');
      expect(error.keyword).to.be.equal('maxProperties');
    });

    describe('Dap Object schema', () => {
      it('should not be empty', () => {
        rawDapContract.dapObjectsDefinition.niceObject.properties = {};

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
        expect(error.keyword).to.be.equal('minProperties');
      });

      it('should have type "object" if defined', () => {
        delete rawDapContract.dapObjectsDefinition.niceObject.properties;

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('properties');
      });

      it('should have "properties"', () => {
        delete rawDapContract.dapObjectsDefinition.niceObject.properties;

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('properties');
      });

      it('should have no non-alphanumeric properties', () => {
        rawDapContract.dapObjectsDefinition.niceObject.properties['(*&^'] = {};

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result, 2);

        const errors = result.getErrors();

        expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
        expect(errors[0].keyword).to.be.equal('pattern');
        expect(errors[1].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
        expect(errors[1].keyword).to.be.equal('propertyNames');
      });

      it('should have "additionalProperties" defined', () => {
        delete rawDapContract.dapObjectsDefinition.niceObject.additionalProperties;

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('additionalProperties');
      });

      it('should have "additionalProperties" defined to false', () => {
        rawDapContract.dapObjectsDefinition.niceObject.additionalProperties = true;

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].additionalProperties');
        expect(error.keyword).to.be.equal('const');
      });

      it('should have no more than 100 properties', () => {
        const propertyDefinition = { };

        rawDapContract.dapObjectsDefinition.niceObject.properties = {};

        Array(101).fill(propertyDefinition).forEach((item, i) => {
          rawDapContract.dapObjectsDefinition.niceObject.properties[i] = item;
        });

        const result = validateDapContract(rawDapContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
        expect(error.keyword).to.be.equal('maxProperties');
      });
    });
  });

  it('should return invalid result if there are additional properties', () => {
    rawDapContract.additionalProperty = { };

    const result = validateDapContract(rawDapContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('additionalProperties');
  });

  it('should return valid result if contract is valid', () => {
    const result = validateDapContract(rawDapContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
