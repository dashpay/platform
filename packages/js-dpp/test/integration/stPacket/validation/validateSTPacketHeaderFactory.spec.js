const Ajv = require('ajv');

const SchemaValidator = require('../../../../lib/validation/SchemaValidator');

const validateSTPacketHeaderFactory = require('../../../../lib/stPacket/validation/validateSTPacketHeaderFactory');

describe('validateSTPacketHeader', () => {
  let rawStPacket;
  let validateSTPacketHeader;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new SchemaValidator(ajv);

    validateSTPacketHeader = validateSTPacketHeaderFactory(validator);

    rawStPacket = {
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
    };
  });

  it('should return error if packet doesn\'t contain `contractId`', () => {
    delete rawStPacket.contractId;

    const errors = validateSTPacketHeader(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('contractId');
  });

  it('should return error if packet doesn\'t contain `itemsMerkleRoot`', () => {
    delete rawStPacket.itemsMerkleRoot;

    const errors = validateSTPacketHeader(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('itemsMerkleRoot');
  });

  it('should return error if packet doesn\'t contain `itemsHash`', () => {
    delete rawStPacket.itemsHash;

    const errors = validateSTPacketHeader(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('itemsHash');
  });

  it('should return empty array if packet structure is correct', () => {
    const errors = validateSTPacketHeader(rawStPacket);

    expect(errors).to.be.empty();
  });
});
