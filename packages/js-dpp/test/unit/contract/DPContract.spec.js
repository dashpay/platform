const rewiremock = require('rewiremock/node');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');

describe('DPContract', () => {
  let hashMock;
  let encodeMock;
  let DPContract;
  let dpContractName;
  let documentType;
  let documentSchema;
  let documents;
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
    documentType = 'niceDocument';
    documentSchema = {
      properties: {
        nice: {
          type: 'boolean',
        },
      },
    };
    documents = {
      [documentType]: documentSchema,
    };

    dpContract = new DPContract(dpContractName, documents);
  });

  describe('constructor', () => {
    it('should create new DP Contract', () => {
      dpContract = new DPContract(dpContractName, documents);
      expect(dpContract.name).to.equal(dpContractName);
      expect(dpContract.version).to.equal(DPContract.DEFAULTS.VERSION);
      expect(dpContract.schema).to.equal(DPContract.DEFAULTS.SCHEMA);
      expect(dpContract.documents).to.equal(documents);
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

  describe('#setDocuments', () => {
    it('should set Documents definition', () => {
      const anotherDocuments = {
        anotherDocument: {
          properties: {
            name: { type: 'string' },
          },
        },
      };

      const result = dpContract.setDocuments(anotherDocuments);

      expect(result).to.equal(dpContract);
      expect(dpContract.documents).to.equal(anotherDocuments);
    });
  });

  describe('#getDocuments', () => {
    it('should return Documents definition', () => {
      const result = dpContract.getDocuments();

      expect(result).to.equal(dpContract.documents);
    });
  });

  describe('#isDocumentDefined', () => {
    it('should return true if Document schema is defined', () => {
      const result = dpContract.isDocumentDefined('niceDocument');

      expect(result).to.equal(true);
    });

    it('should return false if Document schema is not defined', () => {
      const result = dpContract.isDocumentDefined('undefinedDocument');

      expect(result).to.equal(false);
    });
  });

  describe('#setDocumentSchema', () => {
    it('should set Document schema', () => {
      const anotherType = 'prettyDocument';
      const anotherDefinition = {
        properties: {
          name: { type: 'string' },
        },
      };

      const result = dpContract.setDocumentSchema(anotherType, anotherDefinition);

      expect(result).to.equal(dpContract);

      expect(dpContract.documents).to.have.property(anotherType);
      expect(dpContract.documents[anotherType]).to.equal(anotherDefinition);
    });
  });

  describe('#getDocumentSchema', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        dpContract.getDocumentSchema('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
    });

    it('should return Document Schema', () => {
      const result = dpContract.getDocumentSchema(documentType);

      expect(result).to.equal(documentSchema);
    });
  });

  describe('#getDocumentSchemaRef', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        dpContract.getDocumentSchemaRef('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
    });

    it('should return schema with $ref to Document schema', () => {
      const result = dpContract.getDocumentSchemaRef(documentType);

      expect(result).to.deep.equal({
        $ref: 'dp-contract#/documents/niceDocument',
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
        documents,
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
        documents,
        definitions,
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized DP Contract', () => {
      const serializedDocument = '123';

      encodeMock.returns(serializedDocument);

      const result = dpContract.serialize();

      expect(result).to.equal(serializedDocument);

      expect(encodeMock).to.have.been.calledOnceWith(dpContract.toJSON());
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DPContract.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return DP Contract hash', () => {
      const serializedDPContract = '123';
      const hashedDocument = '456';

      DPContract.prototype.serialize.returns(serializedDPContract);

      hashMock.returns(hashedDocument);

      const result = dpContract.hash();

      expect(result).to.equal(hashedDocument);

      expect(DPContract.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedDPContract);
    });
  });
});
