const { validateDapObject, DapObject, DapContract } = require('../../../lib/index');

const InvalidDapObjectTypeError = require('../../../lib/dapContract/errors/InvalidDapObjectTypeError');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateDapObject', () => {
  let dapContract;
  let dapObjects;
  let dapObject;

  beforeEach(() => {
    dapContract = DapContract.fromObject(getLovelyDapContract());
    dapObjects = getLovelyDapObjects().map(rawDapObject => DapObject.fromObject(rawDapObject));
    [dapObject] = dapObjects;
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

  describe('$$type', () => {
    it('should be defined in Dap Contract', () => {
      dapObject.setType('undefinedObject');

      const errors = validateDapObject(dapObjects[0], dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
      expect(error.getType()).to.be.equal('undefinedObject');
    });
  });

  describe('$$revision', () => {
    it('should be less than 0');
    it('should be a number');
    it('should be an integer');
  });

  describe('$$action', () => {
    it('should be a number');
    it('should have predefined value');
  });

  it('should return error if the first object is not valid against schema', () => {
    dapObject.name = 1;

    const errors = validateDapObject(dapObjects[0], dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('.name');
    expect(error.keyword).to.be.equal('type');
  });

  it('should return error if object has undefined properties', () => {
    dapObject.undefined = 1;

    const errors = validateDapObject(dapObjects[0], dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('additionalProperties');
  });

  it('should return error if the second object is not valid against schema', () => {
    dapObjects[1].lastName = 1;

    const errors = validateDapObject(dapObjects[1], dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('.lastName');
    expect(error.keyword).to.be.equal('type');
  });
});
