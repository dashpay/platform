const rewiremock = require('rewiremock/node');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');

describe('Contract', () => {
  let hashMock;
  let encodeMock;
  let Contract;
  let documentType;
  let documentSchema;
  let documents;
  let contract;
  let contractId;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    Contract = rewiremock.proxy('../../../lib/contract/Contract', {
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

    contractId = '6b74011f5d2ad1a8d45b71b9702f54205ce75253593c3cfbba3fdadeca278288';

    contract = new Contract(contractId, documents);
  });

  describe('constructor', () => {
    it('should create new Contract', () => {
      contract = new Contract(contractId, documents);

      expect(contract.version).to.equal(Contract.DEFAULTS.VERSION);
      expect(contract.schema).to.equal(Contract.DEFAULTS.SCHEMA);
      expect(contract.documents).to.equal(documents);
    });
  });

  describe('#getId', () => {
    it('should return base58 encoded Contract ID', () => {
      const result = contract.getId();

      expect(result).to.equal(contractId);
    });
  });

  describe('#getJsonSchemaId', () => {
    it('should return JSON Schema $contractId', () => {
      const result = contract.getJsonSchemaId();

      expect(result).to.equal('contract');
    });
  });

  describe('#setVersion', () => {
    it('should set version', () => {
      const version = 1;

      const result = contract.setVersion(version);

      expect(result).to.equal(contract);
      expect(contract.version).to.equal(version);
    });
  });

  describe('#getVersion', () => {
    it('should return version', () => {
      const result = contract.getVersion();

      expect(result).to.equal(contract.version);
    });
  });

  describe('#setJsonMetaSchema', () => {
    it('should set meta schema', () => {
      const metaSchema = 'http://test.com/schema';

      const result = contract.setJsonMetaSchema(metaSchema);

      expect(result).to.equal(contract);
      expect(contract.schema).to.equal(metaSchema);
    });
  });

  describe('#getJsonMetaSchema', () => {
    it('should return meta schema', () => {
      const result = contract.getJsonMetaSchema();

      expect(result).to.equal(contract.schema);
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

      const result = contract.setDocuments(anotherDocuments);

      expect(result).to.equal(contract);
      expect(contract.documents).to.equal(anotherDocuments);
    });
  });

  describe('#getDocuments', () => {
    it('should return Documents definition', () => {
      const result = contract.getDocuments();

      expect(result).to.equal(contract.documents);
    });
  });

  describe('#isDocumentDefined', () => {
    it('should return true if Document schema is defined', () => {
      const result = contract.isDocumentDefined('niceDocument');

      expect(result).to.equal(true);
    });

    it('should return false if Document schema is not defined', () => {
      const result = contract.isDocumentDefined('undefinedDocument');

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

      const result = contract.setDocumentSchema(anotherType, anotherDefinition);

      expect(result).to.equal(contract);

      expect(contract.documents).to.have.property(anotherType);
      expect(contract.documents[anotherType]).to.equal(anotherDefinition);
    });
  });

  describe('#getDocumentSchema', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        contract.getDocumentSchema('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
    });

    it('should return Document Schema', () => {
      const result = contract.getDocumentSchema(documentType);

      expect(result).to.equal(documentSchema);
    });
  });

  describe('#getDocumentSchemaRef', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        contract.getDocumentSchemaRef('undefinedObject');
      } catch (e) {
        error = e;
      }

      expect(error).to.be.an.instanceOf(InvalidDocumentTypeError);
    });

    it('should return schema with $ref to Document schema', () => {
      const result = contract.getDocumentSchemaRef(documentType);

      expect(result).to.deep.equal({
        $ref: 'contract#/documents/niceDocument',
      });
    });
  });

  describe('#setDefinitions', () => {
    it('should set definitions', () => {
      const definitions = {};

      const result = contract.setDefinitions(definitions);

      expect(result).to.equal(contract);
      expect(contract.definitions).to.equal(definitions);
    });
  });

  describe('#getDefinitions', () => {
    it('should return definitions', () => {
      const result = contract.getDefinitions();

      expect(result).to.equal(contract.definitions);
    });
  });

  describe('#toJSON', () => {
    it('should return Contract as plain object', () => {
      const result = contract.toJSON();

      expect(result).to.deep.equal({
        $schema: Contract.DEFAULTS.SCHEMA,
        contractId,
        version: Contract.DEFAULTS.VERSION,
        documents,
      });
    });

    it('should return plain object with "definitions" if present', () => {
      const definitions = {
        subSchema: { type: 'object' },
      };

      contract.setDefinitions(definitions);

      const result = contract.toJSON();

      expect(result).to.deep.equal({
        $schema: Contract.DEFAULTS.SCHEMA,
        contractId,
        version: Contract.DEFAULTS.VERSION,
        documents,
        definitions,
      });
    });
  });

  describe('#serialize', () => {
    it('should return serialized Contract', () => {
      const serializedDocument = '123';

      encodeMock.returns(serializedDocument);

      const result = contract.serialize();

      expect(result).to.equal(serializedDocument);

      expect(encodeMock).to.have.been.calledOnceWith(contract.toJSON());
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      Contract.prototype.serialize = this.sinonSandbox.stub();
    });

    it('should return Contract hash', () => {
      const serializedContract = '123';
      const hashedDocument = '456';

      Contract.prototype.serialize.returns(serializedContract);

      hashMock.returns(hashedDocument);

      const result = contract.hash();

      expect(result).to.equal(hashedDocument);

      expect(Contract.prototype.serialize).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedContract);
    });
  });
});
