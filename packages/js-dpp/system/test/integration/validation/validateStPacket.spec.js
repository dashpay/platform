const validateStPacket = require('../../../lib/validation/validateStPacket');
const getLovelyContract = require('../../../lib/test/fixtures/getLovelyContract');
const getLovelyObjects = require('../../../lib/test/fixtures/getLovelyObjects');

describe('validateStPacket', () => {
  let packet;
  let contract;

  beforeEach(() => {
    contract = getLovelyContract();
    packet = {
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [
        contract,
      ],
      objects: getLovelyObjects(),
    };
  });

  describe('contractId', () => {
    it('should return error if packet doesn\'t contain `contractId`', () => {
      delete packet.contractId;

      const errors = validateStPacket(packet, contract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].dataPath).to.be.equal('');
      expect(errors[0].keyword).to.be.equal('required');
      expect(errors[0].params.missingProperty).to.be.equal('contractId');
    });

    it('should return error if `contractId` length is lesser 64', () => {
      packet.contractId = '86b273ff';

      const errors = validateStPacket(packet, contract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].schemaPath).to.be.equal('#/properties/contractId/minLength');
    });

    it('should return error if `contractId` length is bigger 64', () => {
      packet.contractId = '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff';

      const errors = validateStPacket(packet, contract);

      expect(errors).to.be.an('array').and.lengthOf(1);
      expect(errors[0].schemaPath).to.be.equal('#/properties/contractId/maxLength');
    });
  });

  it('should return error if packet contains 0 objects and 0 contracts', () => {
    packet.contracts = [];
    packet.objects = [];

    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/allOf/0/not');
  });


  it('should return error if packet doesn\'t contain `objects`', () => {
    delete packet.objects;

    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('objects');
  });

  it('should return error if packet doesn\'t contain `contracts`', () => {
    delete packet.contracts;

    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].dataPath).to.be.equal('');
    expect(errors[0].keyword).to.be.equal('required');
    expect(errors[0].params.missingProperty).to.be.equal('contracts');
  });

  it('should return error if packet contains more than one contract', () => {
    packet.contracts.push(packet.contracts[0]);

    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have more than 1 items');
  });

  it('should return error if there are additional properties in the packet', () => {
    packet.additionalStuff = {};

    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have additional properties');
  });

  it('should validate dap contract');

  it('should validate dap objects');

  it('should return error if object type is undefined in contract', () => {
    packet.objects.push({
      $$type: 'undefinedObject',
      name: 'Anonymous',
    });

    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].missingRef).to.be.equal('dap-contract#/objectsDefinition/undefinedObject');
  });

  it('should return null if packet structure is correct', () => {
    const errors = validateStPacket(packet, contract);

    expect(errors).to.be.null();
  });
});
