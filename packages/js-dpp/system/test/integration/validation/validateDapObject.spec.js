const validateDapObject = require('../../../lib/validation/validateDapObject');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateDapObject', () => {
  let dapObject;
  let dapContract;

  beforeEach(() => {
    dapContract = getLovelyDapContract();
    [dapObject] = getLovelyDapObjects();
  });

  it('should return error if $$type is not present in object', () => {
    delete dapObject.$$type;

    const errors = validateDapObject(dapObject, dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('$$type');
  });

  it('should return error if $$type is not defined in contract', () => {
    dapObject.$$type = 'undefinedObject';

    const errors = validateDapObject(dapObject, dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].missingRef).to.be.equal('dap-contract#/dapObjectsDefinition/undefinedObject');
  });

  it('should return error if object is not valid against schema', () => {
    dapObject.name = 1;

    const errors = validateDapObject(dapObject, dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('.name');
    expect(errors[0].keyword).to.be.equal('type');
  });
});
