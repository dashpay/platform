const rewiremock = require('rewiremock/node');

const InvalidDapObjectTypeError = require('../../../lib/errors/InvalidDapObjectTypeError');

describe('DapContract', () => {
  let hashMock;
  let encodeMock;
  let DapContract;
  let dapContractName;
  let dapObjectType;
  let dapObjectSchema;
  let dapObjectsDefinition;
  let dapContract;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    DapContract = rewiremock.proxy('../../../lib/dapContract/DapContract', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    dapContractName = 'LovelyContract';
    dapObjectType = 'niceObject';
    dapObjectSchema = {
      properties: {
        nice: {
          type: 'boolean',
        },
      },
    };
    dapObjectsDefinition = {
      [dapObjectType]: dapObjectSchema,
    };

    dapContract = new DapContract(dapContractName, dapObjectsDefinition);
  });

  describe('constructor', () => {
    it('should create new Dap Contract', () => {
      dapContract = new DapContract(dapContractName, dapObjectsDefinition);
      expect(dapContract.name).to.be.equal(dapContractName);
      expect(dapContract.version).to.be.equal(DapContract.DEFAULTS.VERSION);
      expect(dapContract.schema).to.be.equal(DapContract.DEFAULTS.SCHEMA);
      expect(dapContract.dapObjectsDefinition).to.be.equal(dapObjectsDefinition);
    });
  });

  describe('#getId', () => {
    it('should calculate Dap Contract ID', () => {
      const hash = '123';

      hashMock.returns(hash);

      const result = dapContract.getId();

      expect(result).to.be.equal(hash);
      expect(hashMock).to.be.calledOnce();
    });
  });

  describe('#getJsonSchemaId', () => {
    it('should return JSON Schema $id', () => {
      const result = dapContract.getJsonSchemaId();

      expect(result).to.be.equal('dap-contract');
    });
  });

  describe('#setName', () => {
    it('should set name', () => {
      const result = dapContract.setName(dapContractName);

      expect(result).to.be.equal(dapContract);
      expect(dapContract.name).to.be.equal(dapContractName);
    });
  });

  describe('#getName', () => {
    it('should return name', () => {
      const result = dapContract.getName();

      expect(result).to.be.equal(dapContract.name);
    });
  });

  describe('#setVersion', () => {
    it('should set version', () => {
      const version = 1;

      const result = dapContract.setVersion(version);

      expect(result).to.be.equal(dapContract);
      expect(dapContract.version).to.be.equal(version);
    });
  });

  describe('#getVersion', () => {
    it('should return version', () => {
      const result = dapContract.getVersion();

      expect(result).to.be.equal(dapContract.version);
    });
  });

  describe('#setJsonMetaSchema', () => {
    it('should set meta schema', () => {
      const metaSchema = 'http://test.com/schema';

      const result = dapContract.setJsonMetaSchema(metaSchema);

      expect(result).to.be.equal(dapContract);
      expect(dapContract.schema).to.be.equal(metaSchema);
    });
  });

  describe('#getJsonMetaSchema', () => {
    it('should return meta schema', () => {
      const result = dapContract.getJsonMetaSchema();

      expect(result).to.be.equal(dapContract.schema);
    });
  });

  describe('#setDapObjectsDefinition', () => {
    it('should set Dap Objects definition', () => {
      const anotherDapObjectsDefinition = {
        anotherObject: {
          properties: {
            name: { type: 'string' },
          },
        },
      };

      const result = dapContract.setDapObjectsDefinition(anotherDapObjectsDefinition);

      expect(result).to.be.equal(dapContract);
      expect(dapContract.dapObjectsDefinition).to.be.equal(anotherDapObjectsDefinition);
    });
  });

  describe('#getDapObjectsDefinition', () => {
    it('should return Dap Objects definition', () => {
      const result = dapContract.getDapObjectsDefinition();

      expect(result).to.be.equal(dapContract.dapObjectsDefinition);
    });
  });

  describe('#isDapObjectDefined', () => {
    it('should return true if Dap Object schema is defined', () => {
      const result = dapContract.isDapObjectDefined('niceObject');

      expect(result).to.be.equal(true);
    });

    it('should return false if Dap Object schema is not defined', () => {
      const result = dapContract.isDapObjectDefined('undefinedObject');

      expect(result).to.be.equal(false);
    });
  });

  describe('#setDapObjectSchema', () => {
    it('should set Dap Object schema', () => {
      const anotherType = 'prettyObject';
      const anotherDefinition = {
        properties: {
          name: { type: 'string' },
        },
      };

      const result = dapContract.setDapObjectSchema(anotherType, anotherDefinition);

      expect(result).to.be.equal(dapContract);

      expect(dapContract.dapObjectsDefinition).to.have.property(anotherType);
      expect(dapContract.dapObjectsDefinition[anotherType]).to.be.equal(anotherDefinition);
    });
  });

  describe('#getDapObjectSchema', () => {
    it('should throw error if Dap Object is not defined', () => {
      let error;
      try {
        dapContract.getDapObjectSchema('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
    });

    it('should return Dap Object Schema', () => {
      const result = dapContract.getDapObjectSchema(dapObjectType);

      expect(result).to.be.equal(dapObjectSchema);
    });
  });

  describe('#getDapObjectSchemaRef', () => {
    it('should throw error if Dap Object is not defined', () => {
      let error;
      try {
        dapContract.getDapObjectSchemaRef('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(InvalidDapObjectTypeError);
    });

    it('should return schema with $ref to Dap Object schema', () => {
      const result = dapContract.getDapObjectSchemaRef(dapObjectType);

      expect(result).to.be.deep.equal({
        $ref: 'dap-contract#/dapObjectsDefinition/niceObject',
      });
    });
  });

  describe('#setDefinitions', () => {
    it('should set definitions', () => {
      const definitions = {};

      const result = dapContract.setDefinitions(definitions);

      expect(result).to.be.equal(dapContract);
      expect(dapContract.definitions).to.be.equal(definitions);
    });
  });

  describe('#getDefinitions', () => {
    it('should return definitions', () => {
      const result = dapContract.getDefinitions();

      expect(result).to.be.equal(dapContract.definitions);
    });
  });

  describe('#toJSON', () => {
    it('should return Dap Contract as plain object', () => {
      const result = dapContract.toJSON();

      expect(result).to.be.deep.equal({
        $schema: DapContract.DEFAULTS.SCHEMA,
        name: dapContractName,
        version: DapContract.DEFAULTS.VERSION,
        dapObjectsDefinition,
      });
    });

    it('should return plain object with "definitions" if present', () => {
      const definitions = {
        subSchema: { type: 'object' },
      };

      dapContract.setDefinitions(definitions);

      const result = dapContract.toJSON();

      expect(result).to.be.deep.equal({
        $schema: DapContract.DEFAULTS.SCHEMA,
        name: dapContractName,
        version: DapContract.DEFAULTS.VERSION,
        dapObjectsDefinition,
        definitions,
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized Dap Contract', () => {
      const serializedDapObject = '123';

      encodeMock.returns(serializedDapObject);

      const result = dapContract.serialize();

      expect(result).to.be.equal(serializedDapObject);

      expect(encodeMock).to.be.calledOnceWith(dapContract.toJSON());
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DapContract.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return Dap Contract hash', () => {
      const serializedDapContract = '123';
      const hashedDapObject = '456';

      DapContract.prototype.serialize.returns(serializedDapContract);

      hashMock.returns(hashedDapObject);

      const result = dapContract.hash();

      expect(result).to.be.equal(hashedDapObject);

      expect(DapContract.prototype.serialize).to.be.calledOnce();

      expect(hashMock).to.be.calledOnceWith(serializedDapContract);
    });
  });
});
