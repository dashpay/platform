const Ajv = require('ajv');

const SchemaValidator = require('../../../lib/validation/SchemaValidator');

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

  it('should return error if $schema is not present', () => {
    delete rawDapContract.$schema;

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('$schema');
  });

  it('should return error if $schema is not valid', () => {
    rawDapContract.$schema = 'wrong';

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].keyword).to.be.equal('const');
    expect(errors[0].dataPath).to.be.equal('.$schema');
  });

  it('should return error if contract name is not present', () => {
    delete rawDapContract.name;

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('name');
  });

  it('should not have empty definitions');

  it('should return error if contract name is not alphanumeric', () => {
    rawDapContract.name = '*(*&^';

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('.name');
    expect(errors[0].keyword).to.be.equal('pattern');
  });

  it('should return error if contract version is not present', () => {
    delete rawDapContract.version;

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('version');
  });

  it('should return error if contract version is not a number', () => {
    rawDapContract.version = 'wrong';

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('.version');
    expect(errors[0].keyword).to.be.equal('type');
  });

  it('should return error if contract has no `dapObjectsDefinition` property', () => {
    delete rawDapContract.dapObjectsDefinition;

    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('dapObjectsDefinition');
  });

  describe('definitions', () => {
    it('should return empty array if definitions property is not present');
    it('should return error if definition name is not valid');
    it('should return error if is is empty');
  });

  describe('objects', () => {
    it('should return error if object definition missing property `properties`', () => {
      delete rawDapContract.dapObjectsDefinition.niceObject.properties;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('properties');
    });

    it('should return error if object definition has no properties defined', () => {
      rawDapContract.dapObjectsDefinition.niceObject.properties = {};

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
      expect(errors[0].keyword).to.be.equal('minProperties');
    });

    it('should return error if object definition has a non-alphanumeric name', () => {
      rawDapContract.dapObjectsDefinition['(*&^'] = rawDapContract.dapObjectsDefinition.niceObject;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition');
      expect(errors[0].keyword).to.be.equal('additionalProperties');
    });

    it('should return error if object definition has a non-alphanumeric property name', () => {
      rawDapContract.dapObjectsDefinition.niceObject.properties['(*&^'] = {};

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(2);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
      expect(errors[0].keyword).to.be.equal('pattern');
      expect(errors[1].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].properties');
      expect(errors[1].keyword).to.be.equal('propertyNames');
    });

    it('should return error if object definition overwrite base object properties');

    it.skip('should return error if object definition has no \'additionalProperties\' property', () => {
      delete rawDapContract.dapObjectsDefinition.niceObject.additionalProperties;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('additionalProperties');
    });

    it.skip('should return error if object definition allows to create additional properties', () => {
      rawDapContract.dapObjectsDefinition.niceObject.additionalProperties = true;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].additionalProperties');
      expect(errors[0].keyword).to.be.equal('const');
    });

    it('should return error if object allOf directive is missing', () => {
      delete rawDapContract.dapObjectsDefinition.niceObject.allOf;

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\']');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('allOf');
    });

    it('should return error if object ref to base object is missing', () => {
      rawDapContract.dapObjectsDefinition.niceObject.allOf = [];

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('.dapObjectsDefinition[\'niceObject\'].allOf');
      expect(errors[0].keyword).to.be.equal('minItems');
    });

    it('should pass if object inherits base object and something else', () => {
      rawDapContract.dapObjectsDefinition.niceObject.allOf.push({
        $ref: 'something else',
      });

      const errors = validateDapContractStructure(rawDapContract);

      expect(errors).to.be.empty();
    });
  });

  it('should return empty array if contract is valid', () => {
    const errors = validateDapContractStructure(rawDapContract);

    expect(errors).to.be.empty();
  });
});
