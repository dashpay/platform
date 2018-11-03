const { validateDapObject, DapObject, DapContract } = require('../../../lib');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateDapObject', () => {
  let dapObject;
  let dapContract;

  beforeEach(() => {
    dapContract = DapContract.fromObject(getLovelyDapContract());
  });


  for (const rawDapObject of getLovelyDapObjects()) {
    describe(`${rawDapObject.$$type}`, () => {
      beforeEach(() => {
        dapObject = DapObject.fromObject(rawDapObject);
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
        dapObject.setType('undefinedObject');

        const errors = validateDapObject(dapObject, dapContract);
        expect(errors).to.be.an('array').and.lengthOf(1);
        expect(errors[0].name).to.be.equal('InvalidDapObjectTypeError');
        expect(errors[0].type).to.be.equal('undefinedObject');
      });

      it('should return error if $$action is not present');
      it('should return error if $$action is not valid');

      it('should return error if object is not valid against schema', () => {
        dapObject.name = 1;

        const errors = validateDapObject(dapObject, dapContract);
        expect(errors).to.be.an('array').and.lengthOf(1);
        expect(errors[0].dataPath).to.be.equal('.name');
        expect(errors[0].keyword).to.be.equal('type');
      });
    });
  }
});
