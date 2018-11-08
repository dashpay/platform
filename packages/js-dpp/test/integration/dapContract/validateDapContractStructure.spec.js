const Ajv = require('ajv');

const SchemaValidator = require('../../../lib/SchemaValidator');

const validateDapContractStructureFactory = require('../../../lib/dapContract/validateDapContractStructureFactory');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');

describe('validateDapContractStructure', () => {
  let rawDapContract;
  let validateDapContractStructure;

  beforeEach(() => {
    rawDapContract = getLovelyDapContract();

    const ajv = new Ajv();
    const validator = new SchemaValidator(ajv);

    validateDapContractStructure = validateDapContractStructureFactory(validator);
  });

  describe('$schema', () => {
    it('should be specified', () => {
      delete rawDapContract.$schema;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('$schema');
    });

    it('should be hardcoded url', () => {
      rawDapContract.$schema = 'wrong';

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].keyword).to.be.equal('const');
      expect(errors[0].dataPath).to.be.equal('.$schema');
    });
  });

  describe('name', () => {
    it('should be specified', () => {
      delete rawDapContract.name;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('name');
    });

    it('should return error if contract name is not alphanumeric', () => {
      rawDapContract.name = '*(*&^';

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.name');
      expect(errors[0].keyword).to.be.equal('pattern');
    });
  });

  describe('version', () => {
    it('should be specified', () => {
      delete rawDapContract.version;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('version');
    });

    it('should be a number', () => {
      rawDapContract.version = 'wrong';

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.version');
      expect(errors[0].keyword).to.be.equal('type');
    });

    it('should be an integer');
  });

  describe('definitions', () => {
    it('should return empty array if definitions property is not present');
    it('should return error if definition name is not valid');
    it('should return error if is is empty');
  });

  describe('dapObjectsDefinition', () => {
    it('should be specified', () => {
      delete rawDapContract.dapObjectsDefinition;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('dapObjectsDefinition');
    });

    it('should not be empty', () => {
      rawDapContract.dapObjectsDefinition.niceObject.properties = {};

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
      expect(errors[0].keyword).to.be.equal('minProperties');
    });

    it('should have no a non-alphanumeric properties', () => {
      rawDapContract.dapObjectsDefinition['(*&^'] = rawDapContract.dapObjectsDefinition.niceObject;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition');
      expect(errors[0].keyword).to.be.equal('additionalProperties');
    });

    describe('Dap Object schema', () => {
      it('should have type "object" if defined', () => {
        delete rawDapContract.dapObjectsDefinition.niceObject.properties;

        const errors = validateDapContractStructure(rawDapContract);

        expect(errors).to.be.an('array').and.lengthOf(1);
        expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
        expect(errors[0].keyword).to.be.equal('required');
        expect(errors[0].params.missingProperty).to.be.equal('properties');
      });

      it('should have "properties"', () => {
        delete rawDapContract.dapObjectsDefinition.niceObject.properties;

        const errors = validateDapContractStructure(rawDapContract);

        expect(errors).to.be.an('array').and.lengthOf(1);
        expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
        expect(errors[0].keyword).to.be.equal('required');
        expect(errors[0].params.missingProperty).to.be.equal('properties');
      });

      it('should have no non-alphanumeric properties', () => {
        rawDapContract.dapObjectsDefinition.niceObject.properties['(*&^'] = {};

        const errors = validateDapContractStructure(rawDapContract);

        expect(errors).to.be.an('array').and.lengthOf(2);
        expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
        expect(errors[0].keyword).to.be.equal('pattern');
        expect(errors[1].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
        expect(errors[1].keyword).to.be.equal('propertyNames');
      });

      it('should not overwrite base object properties');

      it('should have "additionalProperties" defined', () => {
        delete rawDapContract.dapObjectsDefinition.niceObject.additionalProperties;

        const errors = validateDapContractStructure(rawDapContract);

        expect(errors).to.be.an('array').and.lengthOf(1);
        expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
        expect(errors[0].keyword).to.be.equal('required');
        expect(errors[0].params.missingProperty).to.be.equal('additionalProperties');
      });

      it('should have "additionalProperties" defined to false', () => {
        rawDapContract.dapObjectsDefinition.niceObject.additionalProperties = true;

        const errors = validateDapContractStructure(rawDapContract);

        expect(errors).to.be.an('array').and.lengthOf(1);
        expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].additionalProperties');
        expect(errors[0].keyword).to.be.equal('const');
      });
    });
  });

  it('should return empty array if contract is valid', () => {
    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.empty();
  });
});
