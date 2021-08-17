const rewiremock = require('rewiremock/node');
const bs58 = require('bs58');

const Identifier = require('../../../lib/identifier/Identifier');

const InvalidDocumentTypeError = require('../../../lib/errors/InvalidDocumentTypeError');

const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');
const Metadata = require('../../../lib/Metadata');

describe('DataContract', () => {
  let hashMock;
  let encodeMock;
  let DataContract;
  let documentType;
  let documentSchema;
  let documents;
  let dataContract;
  let ownerId;
  let entropy;
  let contractId;
  let getBinaryPropertiesFromSchemaMock;
  let metadataFixture;

  beforeEach(function beforeEach() {
    hashMock = this.sinonSandbox.stub();
    const serializerMock = { encode: this.sinonSandbox.stub() };
    encodeMock = serializerMock.encode;

    getBinaryPropertiesFromSchemaMock = this.sinonSandbox.stub();

    DataContract = rewiremock.proxy('../../../lib/dataContract/DataContract', {
      '../../../lib/util/hash': hashMock,
      '../../../lib/util/serializer': serializerMock,
      '../../../lib/dataContract/getBinaryPropertiesFromSchema': getBinaryPropertiesFromSchemaMock,
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

    getBinaryPropertiesFromSchemaMock.withArgs(documentSchema)
      .returns({
        'firstLevel.secondLevel': {
          type: 'array',
          byteArray: true,
        },
      });

    ownerId = generateRandomIdentifier();
    entropy = Buffer.alloc(32);
    contractId = generateRandomIdentifier();

    dataContract = new DataContract({
      $schema: DataContract.DEFAULTS.SCHEMA,
      $id: contractId,
      ownerId,
      documents,
      $defs: {},
    });

    metadataFixture = new Metadata(42, 0);

    dataContract.setMetadata(metadataFixture);
  });

  describe('constructor', () => {
    it('should create new DataContract', () => {
      const id = generateRandomIdentifier();

      dataContract = new DataContract({
        $schema: DataContract.DEFAULTS.SCHEMA,
        $id: id,
        ownerId,
        documents,
        $defs: {},
      });

      expect(dataContract.id).to.deep.equal(id);
      expect(dataContract.ownerId).to.deep.equal(ownerId);
      expect(dataContract.schema).to.equal(DataContract.DEFAULTS.SCHEMA);
      expect(dataContract.documents).to.equal(documents);
      expect(dataContract.$defs).to.deep.equal({});
    });
  });

  describe('#getId', () => {
    it('should return DataContract Identifier', () => {
      const result = dataContract.getId();

      expect(result).to.deep.equal(contractId);
      expect(result).to.be.instanceof(Identifier);
    });
  });

  describe('#getJsonSchemaId', () => {
    it('should return JSON Schema ID', () => {
      const result = dataContract.getJsonSchemaId();

      expect(result).to.equal(dataContract.getId().toString());
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
      const hashed = Buffer.from(ownerId + entropy);
      hashMock.returns(hashed);

      const result = dataContract.getDocumentSchemaRef(documentType);

      expect(result).to.deep.equal({
        $ref: `${dataContract.getJsonSchemaId()}#/documents/niceDocument`,
      });
    });
  });

  describe('#setDefinitions', () => {
    it('should set $defs', () => {
      const $defs = {};

      const result = dataContract.setDefinitions($defs);

      expect(result).to.equal(dataContract);
      expect(dataContract.$defs).to.equal($defs);
    });
  });

  describe('#getDefinitions', () => {
    it('should return $defs', () => {
      const result = dataContract.getDefinitions();

      expect(result).to.equal(dataContract.$defs);
    });
  });

  describe('#toJSON', () => {
    it('should return DataContract as plain object', () => {
      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        protocolVersion: dataContract.getProtocolVersion(),
        $id: bs58.encode(contractId),
        $schema: DataContract.DEFAULTS.SCHEMA,
        ownerId: bs58.encode(ownerId),
        documents,
      });
    });

    it('should return plain object with "$defs" if present', () => {
      const $defs = {
        subSchema: { type: 'object' },
      };

      dataContract.setDefinitions($defs);

      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        protocolVersion: dataContract.getProtocolVersion(),
        $schema: DataContract.DEFAULTS.SCHEMA,
        $id: bs58.encode(contractId),
        ownerId: bs58.encode(ownerId),
        documents,
        $defs,
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return DataContract as a Buffer', () => {
      const serializedDataContract = Buffer.from('123');

      encodeMock.returns(serializedDataContract);

      const result = dataContract.toBuffer();

      const dataContractToEncode = dataContract.toObject();
      delete dataContractToEncode.protocolVersion;

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(dataContract.getProtocolVersion(), 0);

      expect(encodeMock).to.have.been.calledOnceWith(dataContractToEncode);
      expect(result).to.deep.equal(Buffer.concat([protocolVersionUInt32, serializedDataContract]));
    });
  });

  describe('#hash', () => {
    beforeEach(function beforeEach() {
      DataContract.prototype.toBuffer = this.sinonSandbox.stub();
    });

    it('should return DataContract hash', () => {
      const serializedDataContract = '123';
      const hashedDocument = '456';

      DataContract.prototype.toBuffer.returns(serializedDataContract);

      hashMock.returns(hashedDocument);

      const result = dataContract.hash();

      expect(result).to.equal(hashedDocument);

      expect(DataContract.prototype.toBuffer).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedDataContract);
    });
  });

  describe('#setEntropy', () => {
    it('should set entropy', () => {
      const result = dataContract.setEntropy(entropy);

      expect(result).to.equal(dataContract);
      expect(dataContract.entropy).to.deep.equal(entropy);
    });
  });

  describe('#getEntropy', () => {
    it('should return entropy', () => {
      dataContract.entropy = entropy;

      const result = dataContract.getEntropy();

      expect(result).to.equal(dataContract.entropy);
    });
  });

  describe('#getBinaryProperties', () => {
    it('should return flat map of properties with `contentEncoding` keywords', () => {
      const result = dataContract.getBinaryProperties(documentType);
      expect(result).to.deep.equal({
        'firstLevel.secondLevel': {
          type: 'array',
          byteArray: true,
        },
      });

      expect(getBinaryPropertiesFromSchemaMock).to.have.been.calledOnceWith(documentSchema);
    });

    it('should return cached flat map of properties with `contentEncoding` keywords', () => {
      dataContract.getBinaryProperties(documentType);

      const result = dataContract.getBinaryProperties(documentType);

      expect(result).to.deep.equal({
        'firstLevel.secondLevel': {
          type: 'array',
          byteArray: true,
        },
      });

      expect(getBinaryPropertiesFromSchemaMock).to.have.been.calledOnceWith(documentSchema);
    });

    it('should throw an error if document type is not found', () => {
      try {
        dataContract.getBinaryProperties('unknown');
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e).to.be.an.instanceOf(InvalidDocumentTypeError);
      }
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata', () => {
      const otherMetadata = new Metadata(43, 1);

      dataContract.setMetadata(otherMetadata);

      expect(dataContract.metadata).to.deep.equal(otherMetadata);
    });
  });

  describe('#getMetadata', () => {
    it('should get metadata', () => {
      expect(dataContract.getMetadata()).to.deep.equal(metadataFixture);
    });
  });
});
