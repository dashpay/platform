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

  it('should return error if $$type is not defined in contract', () => {
    dapObject.setType('undefinedObject');

    const errors = validateDapObject(dapObjects[0], dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
    expect(error.getType()).to.be.equal('undefinedObject');
  });

  describe('$$revision', () => {
    it('should return error if it is less than 0');
    it('should return error if it is not a number');
    it('should return error if it is not an integer');
  });

  describe('$$action', () => {
    it('should return error if it is not a number');
    it('should return error if it is not valid');
  });

  it('should return error if the first object is not valid against schema', () => {
    dapObject.name = 1;

    const errors = validateDapObject(dapObjects[0], dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('.name');
    expect(error.keyword).to.be.equal('type');
  });

  it('should return error if object has undefined properties');

  it('should return error if the second object is not valid against schema', () => {
    dapObjects[1].lastName = 1;

    const errors = validateDapObject(dapObjects[1], dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('.lastName');
    expect(error.keyword).to.be.equal('type');
  });
});
