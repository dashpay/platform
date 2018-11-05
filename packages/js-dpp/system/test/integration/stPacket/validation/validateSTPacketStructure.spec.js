const Ajv = require('ajv');

const SchemaValidator = require('../../../../lib/validation/SchemaValidator');

const getLovelyDapContract = require('../../../../lib/test/fixtures/getLovelyDapContract');
const getLovelyDapObjects = require('../../../../lib/test/fixtures/getLovelyDapObjects');

const validateSTPacketStructureFactory = require('../../../../lib/stPacket/validation/validateSTPacketStructureFactory');

describe('validateSTPacketStructure', () => {
  let rawStPacket;
  let rawDapContract;
  let rawDapObjects;
  let validateSTPacketStructure;

  beforeEach(() => {
    const ajv = new Ajv();
    const validator = new SchemaValidator(ajv);

    validateSTPacketStructure = validateSTPacketStructureFactory(validator);

    rawDapContract = getLovelyDapContract();
    rawDapObjects = getLovelyDapObjects();
    rawStPacket = {
      dapContractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsMerkleRoot: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      itemsHash: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      dapContracts: [
      ],
      dapObjects: rawDapObjects,
    };
  });

  it('should return error if packet doesn\'t contain `dapContractId`', () => {
    delete rawStPacket.dapContractId;

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('dapContractId');
  });

  it('should return error if packet doesn\'t contain `itemsMerkleRoot`', () => {
    delete rawStPacket.itemsMerkleRoot;

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('itemsMerkleRoot');
  });

  it('should return error if packet doesn\'t contain `itemsHash`', () => {
    delete rawStPacket.itemsHash;

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('itemsHash');
  });

  it('should return error if packet contains 0 objects and 0 contracts', () => {
    rawStPacket.dapContracts = [];
    rawStPacket.dapObjects = [];

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.keyword).to.be.equal('oneOf');
    expect(error.params.passingSchemas).to.be.deep.equal([0, 1]);
  });

  it('should return error if packet contains the both objects and contracts', () => {
    rawStPacket.dapContracts.push(rawDapContract);

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(3);

    expect(errors[0].dataPath).to.be.equal('.dapObjects');
    expect(errors[0].keyword).to.be.equal('maxItems');

    expect(errors[1].dataPath).to.be.equal('.dapContracts');
    expect(errors[1].keyword).to.be.equal('maxItems');

    expect(errors[2].dataPath).to.be.equal('');
    expect(errors[2].keyword).to.be.equal('oneOf');
    expect(errors[2].params.passingSchemas).to.be.null();
  });

  it('should return error if packet `dapObjects` is not present', () => {
    delete rawStPacket.dapObjects;

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('required');
    expect(error.params.missingProperty).to.be.equal('dapObjects');
  });

  it('should return error if `dapObjects` contains more than 1000 items', () => {
    const thousandDapObjects = (new Array(1001)).fill(rawDapObjects[0]);
    rawStPacket.dapObjects.push(...thousandDapObjects);

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(3);

    expect(errors[0].dataPath).to.be.equal('.dapObjects');
    expect(errors[0].keyword).to.be.equal('maxItems');

    expect(errors[1].dataPath).to.be.equal('.dapObjects');
    expect(errors[1].keyword).to.be.equal('maxItems');

    expect(errors[2].dataPath).to.be.equal('');
    expect(errors[2].keyword).to.be.equal('oneOf');
    expect(errors[2].params.passingSchemas).to.be.null();
  });

  it('should return error if `dapObjects` is not array', () => {
    rawStPacket.dapObjects = 1;

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('.dapObjects');
    expect(error.keyword).to.be.equal('type');
  });

  it('should return error if `dapContracts` is not array', () => {
    rawStPacket.dapContracts = 1;

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('.dapContracts');
    expect(error.keyword).to.be.equal('type');
  });

  it('should return error if packet contains more than one contract', () => {
    rawStPacket.dapContracts.push(rawDapContract, rawDapContract);

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(3);

    expect(errors[0].dataPath).to.be.equal('.dapObjects');
    expect(errors[0].keyword).to.be.equal('maxItems');

    expect(errors[1].dataPath).to.be.equal('.dapContracts');
    expect(errors[1].keyword).to.be.equal('maxItems');

    expect(errors[2].dataPath).to.be.equal('');
    expect(errors[2].keyword).to.be.equal('oneOf');
    expect(errors[2].params.passingSchemas).to.be.null();
  });

  it('should return error if there are additional properties in the packet', () => {
    const additionalProperty = 'additionalStuff';

    rawStPacket[additionalProperty] = {};

    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.an('array').and.lengthOf(1);

    const [error] = errors;

    expect(error.dataPath).to.be.equal('');
    expect(error.keyword).to.be.equal('additionalProperties');
    expect(error.params.additionalProperty).to.be.equal(additionalProperty);
  });

  it('should return error if Dap Contract structure is wrong');
  it('should return error if Dap Object structure is wrong');

  it('should return empty array if packet structure is correct', () => {
    const errors = validateSTPacketStructure(rawStPacket);

    expect(errors).to.be.empty();
  });
});
