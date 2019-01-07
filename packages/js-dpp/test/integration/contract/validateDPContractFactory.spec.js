const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDPContractFactory = require('../../../lib/contract/validateDPContractFactory');

const getDPContractFixture = require('../../../lib/test/fixtures/getDPContractFixture');

const { expectJsonSchemaError } = require('../../../lib/test/expect/expectError');

describe('validateDPContractFactory', () => {
  let rawDPContract;
  let validateDPContract;

  beforeEach(() => {
    rawDPContract = getDPContractFixture().toJSON();

    const ajv = new Ajv();
    const validator = new JsonSchemaValidator(ajv);

    validateDPContract = validateDPContractFactory(validator);
  });

  describe('$schema', () => {
    it('should be present', () => {
      delete rawDPContract.$schema;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('$schema');
    });

    it('should be a string', () => {
      rawDPContract.$schema = 1;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.$schema');
      expect(error.keyword).to.be.equal('type');
    });

    it('should be particular url', () => {
      rawDPContract.$schema = 'wrong';

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.keyword).to.be.equal('const');
      expect(error.dataPath).to.be.equal('.$schema');
    });
  });

  describe('name', () => {
    it('should be present', () => {
      delete rawDPContract.name;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('name');
    });

    it('should be a string', () => {
      rawDPContract.name = 1;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('type');
    });

    it('should be greater or equal to 3', () => {
      rawDPContract.name = 'a'.repeat(2);

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should be less or equal to 24', () => {
      rawDPContract.name = 'a'.repeat(25);

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('maxLength');
    });

    it('should be an alphanumeric string', () => {
      rawDPContract.name = '*(*&^';

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.name');
      expect(error.keyword).to.be.equal('pattern');
    });
  });

  describe('version', () => {
    it('should be present', () => {
      delete rawDPContract.version;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('version');
    });

    it('should be a number', () => {
      rawDPContract.version = 'wrong';

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.version');
      expect(error.keyword).to.be.equal('type');
    });

    it('should be an integer', () => {
      rawDPContract.version = 1.2;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.version');
      expect(error.keyword).to.be.equal('multipleOf');
    });

    it('should be greater or equal to one', () => {
      rawDPContract.version = 0;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.version');
      expect(error.keyword).to.be.equal('minimum');
    });
  });

  describe('definitions', () => {
    it('may not be present', () => {
      delete rawDPContract.definitions;

      const result = validateDPContract(rawDPContract);

      expect(result).to.be.instanceOf(ValidationResult);
      expect(result.isValid()).to.be.true();
    });

    it('should be an object', () => {
      rawDPContract.definitions = 1;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.definitions');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be empty', () => {
      rawDPContract.definitions = {};

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.definitions');
      expect(error.keyword).to.be.equal('minProperties');
    });

    it('should have no a non-alphanumeric properties', () => {
      rawDPContract.definitions = {
        $subSchema: {},
      };

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result, 2);

      const [patternError, propertyNamesError] = result.getErrors();

      expect(patternError.dataPath).to.be.equal('.definitions');
      expect(patternError.keyword).to.be.equal('pattern');

      expect(propertyNamesError.dataPath).to.be.equal('.definitions');
      expect(propertyNamesError.keyword).to.be.equal('propertyNames');
    });

    it('should have no more than 100 properties', () => {
      rawDPContract.definitions = {};

      Array(101).fill({}).forEach((item, i) => {
        rawDPContract.definitions[i] = item;
      });

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.definitions');
      expect(error.keyword).to.be.equal('maxProperties');
    });
  });

  describe('dpObjectsDefinition', () => {
    it('should be present', () => {
      delete rawDPContract.dpObjectsDefinition;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('');
      expect(error.keyword).to.be.equal('required');
      expect(error.params.missingProperty).to.be.equal('dpObjectsDefinition');
    });

    it('should be an object', () => {
      rawDPContract.dpObjectsDefinition = 1;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dpObjectsDefinition');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be empty', () => {
      rawDPContract.dpObjectsDefinition = {};

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dpObjectsDefinition');
      expect(error.keyword).to.be.equal('minProperties');
    });

    it('should have no a non-alphanumeric properties', () => {
      rawDPContract.dpObjectsDefinition['(*&^'] = rawDPContract.dpObjectsDefinition.niceObject;

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dpObjectsDefinition');
      expect(error.keyword).to.be.equal('additionalProperties');
    });

    it('should have no more than 100 properties', () => {
      const niceObjectDefinition = rawDPContract.dpObjectsDefinition.niceObject;

      rawDPContract.dpObjectsDefinition = {};

      Array(101).fill(niceObjectDefinition).forEach((item, i) => {
        rawDPContract.dpObjectsDefinition[i] = item;
      });

      const result = validateDPContract(rawDPContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dpObjectsDefinition');
      expect(error.keyword).to.be.equal('maxProperties');
    });

    describe('DPObject schema', () => {
      it('should not be empty', () => {
        rawDPContract.dpObjectsDefinition.niceObject.properties = {};

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\'].properties');
        expect(error.keyword).to.be.equal('minProperties');
      });

      it('should have type "object" if defined', () => {
        delete rawDPContract.dpObjectsDefinition.niceObject.properties;

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\']');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('properties');
      });

      it('should have "properties"', () => {
        delete rawDPContract.dpObjectsDefinition.niceObject.properties;

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\']');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('properties');
      });

      it('should have no non-alphanumeric properties', () => {
        rawDPContract.dpObjectsDefinition.niceObject.properties['(*&^'] = {};

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result, 2);

        const errors = result.getErrors();

        expect(errors[0].dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\'].properties');
        expect(errors[0].keyword).to.be.equal('pattern');
        expect(errors[1].dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\'].properties');
        expect(errors[1].keyword).to.be.equal('propertyNames');
      });

      it('should have "additionalProperties" defined', () => {
        delete rawDPContract.dpObjectsDefinition.niceObject.additionalProperties;

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\']');
        expect(error.keyword).to.be.equal('required');
        expect(error.params.missingProperty).to.be.equal('additionalProperties');
      });

      it('should have "additionalProperties" defined to false', () => {
        rawDPContract.dpObjectsDefinition.niceObject.additionalProperties = true;

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\'].additionalProperties');
        expect(error.keyword).to.be.equal('const');
      });

      it('should have no more than 100 properties', () => {
        const propertyDefinition = { };

        rawDPContract.dpObjectsDefinition.niceObject.properties = {};

        Array(101).fill(propertyDefinition).forEach((item, i) => {
          rawDPContract.dpObjectsDefinition.niceObject.properties[i] = item;
        });

        const result = validateDPContract(rawDPContract);

        expectJsonSchemaError(result);

        const [error] = result.getErrors();

        expect(error.dataPath).to.be.equal('.dpObjectsDefinition[\'niceObject\'].properties');
        expect(error.keyword).to.be.equal('maxProperties');
      });
    });
  });

  it('should return invalid result if there are additional properties', () => {
    rawDPContract.additionalProperty = { };

    const result = validateDPContract(rawDPContract);

    expectJsonSchemaError(result);

    const [error] = result.getErrors();

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('additionalProperties');
  });

  it('should return valid result if contract is valid', () => {
    const result = validateDPContract(rawDPContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
