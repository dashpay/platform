const rewiremock = require('rewiremock/node');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');

const generateRandomId = require('../../../lib/test/utils/generateRandomId');

describe('DataContract', () => {
  let hashMock;
  let encodeMock;
  let DataContract;
  let documentType;
  let documentSchema;
  let documents;
  let dataContract;
  let contractId;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    DataContract = rewiremock.proxy('../../../lib/dataContract/DataContract', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
    });

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

    contractId = generateRandomId();

    dataContract = new DataContract(contractId, documents);
  });

  describe('constructor', () => {
    it('should create new DataContract', () => {
      dataContract = new DataContract(contractId, documents);

      expect(dataContract.version).to.equal(DataContract.DEFAULTS.VERSION);
      expect(dataContract.schema).to.equal(DataContract.DEFAULTS.SCHEMA);
      expect(dataContract.documents).to.equal(documents);
    });
  });

  describe('#getId', () => {
    it('should return base58 encoded DataContract ID', () => {
      const result = dataContract.getId();

      expect(result).to.equal(contractId);
    });
  });

  describe('#getJsonSchemaId', () => {
    it('should return JSON Schema $contractId', () => {
      const result = dataContract.getJsonSchemaId();

      expect(result).to.equal(dataContract.getId());
    });
  });

  describe('#setVersion', () => {
    it('should set version', () => {
      const version = 1;

      const result = dataContract.setVersion(version);

      expect(result).to.equal(dataContract);
      expect(dataContract.version).to.equal(version);
    });
  });

  describe('#getVersion', () => {
    it('should return version', () => {
      const result = dataContract.getVersion();

      expect(result).to.equal(dataContract.version);
    });
  });

  describe('#setJsonMetaSchema', () => {
    it('should set meta schema', () => {
      const metaSchema = 'http://test.com/schema';

      const result = dataContract.setJsonMetaSchema(metaSchema);

      expect(result).to.equal(dataContract);
      expect(dataContract.schema).to.equal(metaSchema);
    });
  });

  describe('#getJsonMetaSchema', () => {
    it('should return meta schema', () => {
      const result = dataContract.getJsonMetaSchema();

      expect(result).to.equal(dataContract.schema);
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

      const result = dataContract.setDocuments(anotherDocuments);

      expect(result).to.equal(dataContract);
      expect(dataContract.documents).to.equal(anotherDocuments);
    });
  });

  describe('#getDocuments', () => {
    it('should return Documents definition', () => {
      const result = dataContract.getDocuments();

      expect(result).to.equal(dataContract.documents);
    });
  });

  describe('#isDocumentDefined', () => {
    it('should return true if Document schema is defined', () => {
      const result = dataContract.isDocumentDefined('niceDocument');

      expect(result).to.equal(true);
    });

    it('should return false if Document schema is not defined', () => {
      const result = dataContract.isDocumentDefined('undefinedDocument');

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

      const result = dataContract.setDocumentSchema(anotherType, anotherDefinition);

      expect(result).to.equal(dataContract);

      expect(dataContract.documents).to.have.property(anotherType);
      expect(dataContract.documents[anotherType]).to.equal(anotherDefinition);
    });
  });

  describe('#getDocumentSchema', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        dataContract.getDocumentSchema('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
    });

    it('should return Document Schema', () => {
      const result = dataContract.getDocumentSchema(documentType);

      expect(result).to.equal(documentSchema);
    });
  });

  describe('#getDocumentSchemaRef', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        dataContract.getDocumentSchemaRef('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
    });

    it('should return schema with $ref to Document schema', () => {
      const result = dataContract.getDocumentSchemaRef(documentType);

      expect(result).to.deep.equal({
        $ref: `${dataContract.getId()}#/documents/niceDocument`,
      });
    });
  });

  describe('#setDefinitions', () => {
    it('should set definitions', () => {
      const definitions = {};

      const result = dataContract.setDefinitions(definitions);

      expect(result).to.equal(dataContract);
      expect(dataContract.definitions).to.equal(definitions);
    });
  });

  describe('#getDefinitions', () => {
    it('should return definitions', () => {
      const result = dataContract.getDefinitions();

      expect(result).to.equal(dataContract.definitions);
    });
  });

  describe('#toJSON', () => {
    it('should return DataContract as plain object', () => {
      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        $schema: DataContract.DEFAULTS.SCHEMA,
        contractId,
        version: DataContract.DEFAULTS.VERSION,
        documents,
      });
    });

    it('should return plain object with "definitions" if present', () => {
      const definitions = {
        subSchema: { type: 'object' },
      };

      dataContract.setDefinitions(definitions);

      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        $schema: DataContract.DEFAULTS.SCHEMA,
        contractId,
        version: DataContract.DEFAULTS.VERSION,
        documents,
        definitions,
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized DataContract', () => {
      const serializedDocument = '123';

      encodeMock.returns(serializedDocument);

      const result = dataContract.serialize();

      expect(result).to.equal(serializedDocument);

      expect(encodeMock).to.have.been.calledOnceWith(dataContract.toJSON());
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DataContract.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return DataContract hash', () => {
      const serializedDataContract = '123';
      const hashedDocument = '456';

      DataContract.prototype.serialize.returns(serializedDataContract);

      hashMock.returns(hashedDocument);

      const result = dataContract.hash();

      expect(result).to.equal(hashedDocument);

      expect(DataContract.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedDataContract);
    });
  });
});
