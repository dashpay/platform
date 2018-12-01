const {
  validateSTPacket,
  DapContract,
  STPacket,
  DapObject,
} = require('../../../../lib/index');

const InvalidDapObjectTypeError = require('../../../../lib/dapContract/errors/InvalidDapObjectTypeError');

const getLovelyDapContract = require('../../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../../lib/test/fixtures/getLovelyDapObjects');

describe('validateSTPacketHeader', () => {
  let stPacket;
  let dapContract;

  beforeEach(() => {
    dapContract = DapContract.fromObject(getLovelyDapContract());
    stPacket = STPacket.fromObject({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
      objects: getLovelyDapObjects(),
    });
  });

  it('should validate structure too');

  describe('contractId', () => {
    it('should be a string', () => {
      stPacket.setDapContractId(1);

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be less than 64', () => {
      stPacket.setDapContractId('86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should not be bigger than 64', () => {
      stPacket.setDapContractId('86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.contractId');
      expect(error.keyword).to.be.equal('maxLength');
    });

    it('should be equal to Dap Contract ID');
  });

  describe('itemsMerkleRoot', () => {
    it('should be a string', () => {
      stPacket.setItemsMerkleRoot(1);

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be less than 64', () => {
      stPacket.setItemsMerkleRoot('86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should not be bigger than 64', () => {
      stPacket.setItemsMerkleRoot('86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.itemsMerkleRoot');
      expect(error.keyword).to.be.equal('maxLength');
    });

    it('should be merkle root of packet\'s items');
  });

  describe('itemsHash', () => {
    it('should be a string', () => {
      stPacket.setItemsHash(1);

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('type');
    });

    it('should not be less than 64', () => {
      stPacket.setItemsHash('86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('minLength');
    });

    it('should not be bigger than 64', () => {
      stPacket.setItemsHash('86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff');

      const errors = validateSTPacket(stPacket, dapContract);

      expect(errors).to.be.an('array').and.lengthOf(1);

      const [error] = errors;

      expect(error.dataPath).to.be.equal('.itemsHash');
      expect(error.keyword).to.be.equal('maxLength');
    });

    it('should be hash of packet\'s items');
  });

  it('should return error if packet is empty', () => {
    stPacket.setDapObjects([]);
    stPacket.setDapContract(null);

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.schemaPath).to.be.equal('#/oneOf');
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

  it('should return empty array if ST Packet with Dap Objects is correct', () => {
    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.empty();
  });

  it('should return empty array if ST Packet with Dap Contract is correct', () => {
    stPacket.setDapContractId(dapContract.getId());
    stPacket.setDapObjects([]);
    stPacket.setDapContract(dapContract);

    const errors = validateSTPacket(stPacket, dapContract);

    expect(errors).to.be.empty();
  });
});
