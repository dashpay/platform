const {
  validateSTPacket,
  DapContract,
  STPacket,
  DapObject,
} = require('../../../lib');

const InvalidDapObjectTypeError = require('../../../lib/errors/InvalidDapObjectTypeError');

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

  it('should validate structure too');

  describe('dapContractId', () => {
    it('should return error if `dapContractId` length is lesser 64', () => {
      stPacket.setDapContractId('86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].schemaPath).to.be.equal('#/properties/dapContractId/minLength');
    });

    it('should return error if `dapContractId` length is bigger 64', () => {
      stPacket.setDapContractId('86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].schemaPath).to.be.equal('#/properties/dapContractId/maxLength');
    });
  });

  it('should return error if packet is empty', () => {
    // TODO How to remove contract from ST Packet?
    stPacket.dapContracts = [];
    stPacket.setDapObjects([]);

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/allOf/0/not');
  });

  it('should validate dap contract if present');

  it('should validate dap objects if present');

  it('should return error if object type is undefined in contract', () => {
    const wrongType = 'undefinedObject';

    stPacket.setDapObjects([
      new DapObject(wrongType, { name: 'Anonymous' }),
    ]);

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
    expect(error.getType()).to.be.equal(wrongType);
  });

  it('should return empty array if packet data is correct', () => {
    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.empty();
  });
});
