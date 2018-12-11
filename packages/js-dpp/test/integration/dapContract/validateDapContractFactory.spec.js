const Ajv = require('ajv');

const JsonSchemaValidator = require('../../../lib/validation/JsonSchemaValidator');

const ValidationResult = require('../../../lib/validation/ValidationResult');

const validateDapContractFactory = require('../../../lib/dapContract/validateDapContractFactory');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');

const { expectJsonSchemaError } = require('../../../lib/test/expect/expectError');

describe('validateDapContractFactory', () => {
  let rawDapContract;
  let validateDapContract;

  beforeEach(() => {
    rawDapContract = getLovelyDapContract().toJSON();

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

    it('should be greater or equal to three');

    it('should be less or equal to 23');

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

    it('should be an integer');

    it('should be greater or equal to one');
  });

  describe('definitions', () => {
    it('may not be present');
    it('should be object');
    it('should not be empty');
    it('should have no a non-alphanumeric properties');
    it('should have no more than 100 properties');
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

    it('should not be empty', () => {
      rawDapContract.dapObjectsDefinition.niceObject.properties = {};

      const result = validateDapContract(rawDapContract);

      expectJsonSchemaError(result);

      const [error] = result.getErrors();

      expect(error.dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
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

    it('should have no more than 100 properties');

    describe('Dap Object schema', () => {
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

      it('should not overwrite base object properties');

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

      describe('primaryKey', () => {
        it('can be omitted');
        it('should have "composite" property if defined');
        it('should have "includes" if "composite" property is true');
        it('should not have "includes if "composite" property is false');

        describe('includes', () => {
          it('should be array');
          it('should have string items');
          it('should have alphanumeric items');
          it('should have at least one item');
          it('should have unique items');
          it('can have many items');
          it('should have only allowed items: "buid" and properties');
          it('should contains only "string" and "number" typed properties');
        });
      });
    });
  });

  it('should return empty array if contract is valid', () => {
    const result = validateDapContract(rawDapContract);

    expect(result).to.be.instanceOf(ValidationResult);
    expect(result.isValid()).to.be.true();
  });
});
