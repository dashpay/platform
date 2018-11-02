const { validateSTPacket, DapContract, STPacket } = require('../../../lib');

const getLovelyDapContract = require('../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateSTPacket', () => {
  let stPacket;
  let dapContract;

  beforeEach(() => {
    dapContract = DapContract.fromObject(getLovelyDapContract());
    stPacket = STPacket.fromObject({
      dapContractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      dapContracts: [
        getLovelyDapContract(),
      ],
      dapObjects: getLovelyDapObjects(),
    });
  });

  describe('dapContractId', () => {
    it('should return error if packet doesn\'t contain `dapContractId`', () => {
      delete stPacket.dapContractId;

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('dapContractId');
    });

    it('should return error if `dapContractId` length is lesser 64', () => {
      stPacket.dapContractId = '86b273ff';

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].schemaPath).to.be.equal('#/properties/dapContractId/minLength');
    });

    it('should return error if `dapContractId` length is bigger 64', () => {
      stPacket.dapContractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].schemaPath).to.be.equal('#/properties/dapContractId/maxLength');
    });
  });

  it('should return error if packet contains 0 objects and 0 contracts', () => {
    stPacket.dapContracts = [];
    stPacket.dapObjects = [];

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/allOf/0/not');
  });

  it('should return error if packet contains the both objects and contracts');

  it('should return error if packet doesn\'t contain `dapObjects`', () => {
    delete stPacket.dapObjects;

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('dapObjects');
  });

  it('should return error if packet doesn\'t contain `dapContracts`', () => {
    delete stPacket.dapContracts;

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('dapContracts');
  });

  it('should return error if packet contains more than one contract', () => {
    stPacket.dapContracts.push(stPacket.dapContracts[0]);

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have more than 1 items');
  });

  it('should return error if there are additional properties in the packet', () => {
    stPacket.additionalStuff = {};

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have additional properties');
  });

  it('should validate dap contract if present');

  it('should validate dap objects if present');

  it('should return error if object type is undefined in contract', () => {
    stPacket.dapObjects.push({
      $$type: 'undefinedObject',
      name: 'Anonymous',
    });

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].missingRef).to.be.equal('dap-contract#/dapObjectsDefinition/undefinedObject');
  });

  it('should return null if packet structure is correct', () => {
    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.null();
  });
});
