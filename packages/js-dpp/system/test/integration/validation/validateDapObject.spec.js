const validateDapObject = require('../../../lib/validation/validateDapObject');

const getLovelyContract = require('../../../lib/test/fixtures/getLovelyContract');
const getLovelyObjects = require('../../../lib/test/fixtures/getLovelyObjects');

describe('validateDapObject', () => {
  let object;
  let contract;

  beforeEach(() => {
    contract = getLovelyContract();
    [object] = getLovelyObjects();
  });

  it('should return error if $$type is not present in object', () => {
    delete object.$$type;

    const errors = validateDapObject(object, contract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('$$type');
  });

  it('should return error if $$type is not defined in contract', () => {
    object.$$type = 'undefinedObject';

    const errors = validateDapObject(object, contract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].missingRef).to.be.equal('dap-contract#/objectsDefinition/undefinedObject');
  });

  it('should return error if object is not valid against schema', () => {
    object.name = 1;

    const errors = validateDapObject(object, contract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('.name');
    expect(errors[0].keyword).to.be.equal('type');
  });
});
