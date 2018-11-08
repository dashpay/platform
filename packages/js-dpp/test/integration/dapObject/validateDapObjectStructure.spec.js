const Ajv = require('ajv');

const SchemaValidator = require('../../../lib/SchemaValidator');

const validateDapObjectStructureFactory = require('../../../lib/dapObject/validateDapObjectStructureFactory');

const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateDapObjectStructure', () => {
  let rawDapObject;
  let validateDapObjectStructure;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new SchemaValidator(ajv);

    validateDapObjectStructure = validateDapObjectStructureFactory(validator);

    [rawDapObject] = getLovelyDapObjects();
  });

  it('should return error if $$type is not present', () => {
    delete rawDapObject.$$type;

    const errors = validateDapObjectStructure(rawDapObject);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('$$type');
  });

  it('should return error if $$revision is not present', () => {
    delete rawDapObject.$$revision;

    const errors = validateDapObjectStructure(rawDapObject);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('$$revision');
  });

  it('should return error if $$action is not present', () => {
    delete rawDapObject.$$action;

    const errors = validateDapObjectStructure(rawDapObject);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('$$action');
  });

  it('should return empty array if Dap Object base structure is valid', () => {
    delete rawDapObject.$$action;

    const errors = validateDapObjectStructure(rawDapObject);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('$$action');
  });
});
