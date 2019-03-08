const rewiremock = require('rewiremock/node');

const InvalidDPObjectTypeError = require('../../../lib/errors/InvalidDPObjectTypeError');

describe('DPContract', () => {
  let hashMock;
  let encodeMock;
  let DPContract;
  let dpContractName;
  let dpObjectType;
  let dpObjectSchema;
  let dpObjectsDefinition;
  let dpContract;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    DPContract = rewiremock.proxy('../../../lib/contract/DPContract', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

    dpContractName = 'LovelyContract';
    dpObjectType = 'niceObject';
    dpObjectSchema = {
      properties: {
        nice: {
          type: 'boolean',
        },
      },
    };
    dpObjectsDefinition = {
      [dpObjectType]: dpObjectSchema,
    };

    dpContract = new DPContract(dpContractName, dpObjectsDefinition);
  });

  describe('constructor', () => {
    it('should create new DP Contract', () => {
      dpContract = new DPContract(dpContractName, dpObjectsDefinition);
      expect(dpContract.name).to.equal(dpContractName);
      expect(dpContract.version).to.equal(DPContract.DEFAULTS.VERSION);
      expect(dpContract.schema).to.equal(DPContract.DEFAULTS.SCHEMA);
      expect(dpContract.dpObjectsDefinition).to.equal(dpObjectsDefinition);
    });
  });

  describe('#getId', () => {
    it('should calculate DP Contract ID', () => {
      const hash = '123';

      hashMock.returns(hash);

      const result = dpContract.getId();

      expect(result).to.equal(hash);
      expect(hashMock).to.have.been.calledOnce();
    });
  });

  describe('#getJsonSchemaId', () => {
    it('should return JSON Schema $id', () => {
      const result = dpContract.getJsonSchemaId();

      expect(result).to.equal('dp-contract');
    });
  });

  describe('#setName', () => {
    it('should set name', () => {
      const result = dpContract.setName(dpContractName);

      expect(result).to.equal(dpContract);
      expect(dpContract.name).to.equal(dpContractName);
    });
  });

  describe('#getName', () => {
    it('should return name', () => {
      const result = dpContract.getName();

      expect(result).to.equal(dpContract.name);
    });
  });

  describe('#setVersion', () => {
    it('should set version', () => {
      const version = 1;

      const result = dpContract.setVersion(version);

      expect(result).to.equal(dpContract);
      expect(dpContract.version).to.equal(version);
    });
  });

  describe('#getVersion', () => {
    it('should return version', () => {
      const result = dpContract.getVersion();

      expect(result).to.equal(dpContract.version);
    });
  });

  describe('#setJsonMetaSchema', () => {
    it('should set meta schema', () => {
      const metaSchema = 'http://test.com/schema';

      const result = dpContract.setJsonMetaSchema(metaSchema);

      expect(result).to.equal(dpContract);
      expect(dpContract.schema).to.equal(metaSchema);
    });
  });

  describe('#getJsonMetaSchema', () => {
    it('should return meta schema', () => {
      const result = dpContract.getJsonMetaSchema();

      expect(result).to.equal(dpContract.schema);
    });
  });

  describe('#setDPObjectsDefinition', () => {
    it('should set DPObjects definition', () => {
      const anotherDPObjectsDefinition = {
        anotherObject: {
          properties: {
            name: { type: 'string' },
          },
        },
      };

      const result = dpContract.setDPObjectsDefinition(anotherDPObjectsDefinition);

      expect(result).to.equal(dpContract);
      expect(dpContract.dpObjectsDefinition).to.equal(anotherDPObjectsDefinition);
    });
  });

  describe('#getDPObjectsDefinition', () => {
    it('should return DPObjects definition', () => {
      const result = dpContract.getDPObjectsDefinition();

      expect(result).to.equal(dpContract.dpObjectsDefinition);
    });
  });

  describe('#isDPObjectDefined', () => {
    it('should return true if DPObject schema is defined', () => {
      const result = dpContract.isDPObjectDefined('niceObject');

      expect(result).to.equal(true);
    });

    it('should return false if DPObject schema is not defined', () => {
      const result = dpContract.isDPObjectDefined('undefinedObject');

      expect(result).to.equal(false);
    });
  });

  describe('#setDPObjectSchema', () => {
    it('should set DPObject schema', () => {
      const anotherType = 'prettyObject';
      const anotherDefinition = {
        properties: {
          name: { type: 'string' },
        },
      };

      const result = dpContract.setDPObjectSchema(anotherType, anotherDefinition);

      expect(result).to.equal(dpContract);

      expect(dpContract.dpObjectsDefinition).to.have.property(anotherType);
      expect(dpContract.dpObjectsDefinition[anotherType]).to.equal(anotherDefinition);
    });
  });

  describe('#getDPObjectSchema', () => {
    it('should throw error if DPObject is not defined', () => {
      let error;
      try {
        dpContract.getDPObjectSchema('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDPObjectTypeError);
    });

    it('should return DPObject Schema', () => {
      const result = dpContract.getDPObjectSchema(dpObjectType);

      expect(result).to.equal(dpObjectSchema);
    });
  });

  describe('#getDPObjectSchemaRef', () => {
    it('should throw error if DPObject is not defined', () => {
      let error;
      try {
        dpContract.getDPObjectSchemaRef('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDPObjectTypeError);
    });

    it('should return schema with $ref to DPObject schema', () => {
      const result = dpContract.getDPObjectSchemaRef(dpObjectType);

      expect(result).to.deep.equal({
        $ref: 'dp-contract#/dpObjectsDefinition/niceObject',
      });
    });
  });

  describe('#setDefinitions', () => {
    it('should set definitions', () => {
      const definitions = {};

      const result = dpContract.setDefinitions(definitions);

      expect(result).to.equal(dpContract);
      expect(dpContract.definitions).to.equal(definitions);
    });
  });

  describe('#getDefinitions', () => {
    it('should return definitions', () => {
      const result = dpContract.getDefinitions();

      expect(result).to.equal(dpContract.definitions);
    });
  });

  describe('#toJSON', () => {
    it('should return DP Contract as plain object', () => {
      const result = dpContract.toJSON();

      expect(result).to.deep.equal({
        $schema: DPContract.DEFAULTS.SCHEMA,
        name: dpContractName,
        version: DPContract.DEFAULTS.VERSION,
        dpObjectsDefinition,
      });
    });

    it('should return plain object with "definitions" if present', () => {
      const definitions = {
        subSchema: { type: 'object' },
      };

      dpContract.setDefinitions(definitions);

      const result = dpContract.toJSON();

      expect(result).to.deep.equal({
        $schema: DPContract.DEFAULTS.SCHEMA,
        name: dpContractName,
        version: DPContract.DEFAULTS.VERSION,
        dpObjectsDefinition,
        definitions,
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized DP Contract', () => {
      const serializedDPObject = '123';

      encodeMock.returns(serializedDPObject);

      const result = dpContract.serialize();

      expect(result).to.equal(serializedDPObject);

      expect(encodeMock).to.have.been.calledOnceWith(dpContract.toJSON());
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DPContract.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return DP Contract hash', () => {
      const serializedDPContract = '123';
      const hashedDPObject = '456';

      DPContract.prototype.serialize.returns(serializedDPContract);

      hashMock.returns(hashedDPObject);

      const result = dpContract.hash();

      expect(result).to.equal(hashedDPObject);

      expect(DPContract.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedDPContract);
    });
  });
});
