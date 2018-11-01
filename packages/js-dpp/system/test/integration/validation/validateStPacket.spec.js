const validateStPacket = require('../../../lib/validation/validateStPacket');

describe('validateStPacket', () => {
  it('should return error if packet contains 0 objects and 0 contracts', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/allOf/0/not');
  });

  it('should return error if packet doesn\'t contain `contractId`', () => {
    const errors = validateStPacket({
      contracts: [],
      objects: [{
        _type: 'niceObject',
      }],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal("should have required property 'contractId'");
  });

  it('should return error if `contractId` length is lesser 64', () => {
    const errors = validateStPacket({
      contractId: '86b273ff',
      contracts: [],
      objects: [{
        _type: 'niceObject',
      }],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/properties/contractId/minLength');
  });

  it('should return error if `contractId` length is bigger 64', () => {
    const errors = validateStPacket({
      contractId: '86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff86b273ff',
      contracts: [],
      objects: [{
        _type: 'niceObject',
      }],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/properties/contractId/maxLength');
  });

  it('should return error if packet doesn\'t contain `objects`', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal("should have required property 'objects'");
  });

  it('should return error if packet doesn\'t contain `contracts`', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      objects: [{
        _type: 'niceObject',
      }],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal("should have required property 'contracts'");
  });

  it('should return error if contract object is empty', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{}],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal("should have required property 'name'");
  });

  it('should return error if contract has no `version` property', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal("should have required property 'version'");
  });

  it('should return error if contract has no `objects` property', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal("should have required property 'objects'");
  });

  it('should return error if contract object definition missing property `properties`', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: {} },
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/properties/objects/patternProperties/%5E%5Ba-zA-Z0-9-_%5D%7B1%2C255%7D%24/required');
  });

  it('should return error if contract object definition has no properties defined', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: {} } },
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have fewer than 1 properties');
  });

  it('should return error if contract object definition has a non-alphanumeric property name', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { '$&?': {} } } },
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(2);

    expect(errors[0].message).to.be.equal('should match pattern "^[a-zA-Z0-9-_]{1,255}$"');
    expect(errors[1].message).to.be.equal('property name \'$&?\' is invalid');
  });

  it('should return error if contract object has a non-alphanumeric name', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { 'hello$&?': { properties: { '$&?': {} } } },
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].schemaPath).to.be.equal('#/properties/objects/additionalProperties');
  });

  it('should return error if contract object definition has no \'additionalProperties\' property', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { hello: {} } } },
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should have required property \'additionalProperties\'');
  });

  it('should return error if contract object definition allows to create additional properties', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { hello: {} }, additionalProperties: true } },
      }],
      objects: [],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should be equal to constant');
  });

  it('should return error if there are additional properties in the packet', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { hello: {} }, additionalProperties: false } },
      }],
      objects: [],
      additionalStuff: {},
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have additional properties');
  });

  it('should return error if packet contains more than one contract', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      objects: [{
        _type: 'niceObject',
      }],
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { hello: {} }, additionalProperties: false } },
      }, {
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { hello: {} }, additionalProperties: false } },
      }],
    }, {});

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('should NOT have more than 1 items');
  });

  it('should return error if object type is undefined in contract', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      objects: [{
        _type: 'lovelyObject',
      }],
      contracts: [],
    }, {
      objects: {
        niceObject: {
          properties: {
            name: {
              type: 'string',
            },
          },
        },
      },
    });

    expect(errors).to.be.an('array').and.lengthOf(1);
    expect(errors[0].message).to.be.equal('can\'t resolve reference dap-contract#/objects/lovelyObject from id #');
  });


  it('should return null if packet structure is correct', () => {
    const errors = validateStPacket({
      contractId: '6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b',
      objects: [{
        _type: 'niceObject',
        name: 'Сutie',
      }],
      contracts: [{
        name: 'lovelyContract',
        version: 1,
        objects: { helloObject: { properties: { hello: {} }, additionalProperties: false } },
      }],
    }, {
      objects: {
        niceObject: {
          properties: {
            name: {
              type: 'string',
            },
          },
        },
      },
    });

    expect(errors).to.be.null();
  });
});
