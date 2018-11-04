const { validateDapObject, DapObject, DapContract } = require('../../../lib');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateDapObject', () => {
  let dapContract;
  let dapObjects;

  beforeEach(() => {
    dapContract = DapContract.fromObject(getLovelyDapContract());
    dapObjects = getLovelyDapObjects().map(rawDapObject => DapObject.fromObject(rawDapObject));
  });

  it('should return error if $$type is not present in object', () => {
    delete dapObjects[0].$$type;

    const errors = validateDapObject(dapObjects[0], dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('$$type');
  });

  it('should return error if $$type is not defined in contract', () => {
    dapObjects[0].setType('undefinedObject');

    const errors = validateDapObject(dapObjects[0], dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].name).to.be.equal('InvalidDapObjectTypeError');
    expect(errors[0].type).to.be.equal('undefinedObject');
  });

  it('should return error if $$revision is not present');
  it('should return error if $$revision is not valid');

  it('should return error if $$action is not present');
  it('should return error if $$action is not valid');

  it('should return error if the first object is not valid against schema', () => {
    dapObjects[0].name = 1;

    const errors = validateDapObject(dapObjects[0], dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('.name');
    expect(errors[0].keyword).to.be.equal('type');
  });

  it('should return error if the second object is not valid against schema', () => {
    dapObjects[1].lastName = 1;

    const errors = validateDapObject(dapObjects[1], dapContract);
    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('.lastName');
    expect(errors[0].keyword).to.be.equal('type');
  });
});
