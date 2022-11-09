const bs58 = require('bs58');

//const InvalidDocumentTypeError = require('@dashevo/dpp/lib/errors/InvalidDocumentTypeError');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const Metadata = require('@dashevo/dpp/lib/Metadata');
const JsDataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const hash = require('@dashevo/dpp/lib/util/hash');
const serializer = require('@dashevo/dpp/lib/util/serializer');
const getBinaryPropertiesFromSchema = require('@dashevo/dpp/lib/dataContract/getBinaryPropertiesFromSchema');
const { default: loadWasmDpp } = require('../../../dist');

describe('DataContract', () => {
  let hashMock;
  let encodeMock;
  let documentType;
  let documentSchema;
  let documents;
  let dataContract;
  let ownerId;
  let entropy;
  let contractId;
  let getBinaryPropertiesFromSchemaMock;
  let metadataFixture;

  before(async () => {
    ({
      DataContract, DataContractDefaults, Identifier, InvalidDocumentTypeError
    } = await loadWasmDpp());
  });

  beforeEach(function beforeEach() {
    encodeMock = this.sinonSandbox.stub(serializer, 'encode');
    hashMock = this.sinonSandbox.stub(hash, 'hash');
    getBinaryPropertiesFromSchemaMock = this.sinonSandbox.stub(getBinaryPropertiesFromSchema, 'getBinaryPropertiesFromSchema');

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

    dataContract = new JsDataContract({
      $schema: JsDataContract.DEFAULTS.SCHEMA,
      $id: contractId,
      version: 1,
      ownerId,
      documents,
      $defs: {},
    });

    wasmDataContract = new DataContract({
      $schema: DataContractDefaults.SCHEMA,
      $id: contractId,
      version: 1,
      protocolVersion: 1,
      ownerId,
      documents,
      $defs: {},
    });

    metadataFixture = new Metadata(42, 0);

    dataContract.setMetadata(metadataFixture);
  });

  afterEach(() => {
    encodeMock.restore();
    hashMock.restore();
    getBinaryPropertiesFromSchemaMock.restore();
  });

  describe('constructor', () => {
    it('should create new DataContract', () => {
      const id = generateRandomIdentifier();

      dataContract = new DataContract({
        $schema: DataContractDefaults.SCHEMA,
        $id: id,
        ownerId,
	protocolVersion: 1,
	version: 1,
        documents,
        $defs: {},
      });

      expect(dataContract.getId().toBuffer()).to.deep.equal(id.toBuffer());
      expect(dataContract.getOwnerId().toBuffer()).to.deep.equal(ownerId.toBuffer());
      expect(dataContract.getJsonMetaSchema()).to.deep.equal(DataContractDefaults.SCHEMA);
      expect(dataContract.getDocuments()).to.deep.equal(documents);
      expect(dataContract.getDefinitions()).to.deep.equal({});
    });
  });

  describe('#getId', () => {
    it('should return DataContract Identifier', () => {
      const result = wasmDataContract.getId();

      expect(result.toBuffer()).to.deep.equal(contractId.toBuffer);
      expect(result).to.be.instanceof(Identifier);
    });
  });

  describe('#getJsonSchemaId', () => {
    it('should return JSON Schema ID', () => {
      const result = wasmDataContract.getJsonSchemaId();

      expect(result).to.equal(wasmDataContract.getId().toString());
    });
  });

  describe('#setJsonMetaSchema', () => {
    it('should set meta schema', () => {
      const metaSchema = 'http://test.com/schema';

      wasmDataContract.setJsonMetaSchema(metaSchema);

      expect(wasmDataContract.getJsonMetaSchema()).to.deep.equal(metaSchema);
    });
  });

  describe('#getJsonMetaSchema', () => {
    it('should return meta schema', () => {
      const result = wasmDataContract.getJsonMetaSchema();

      expect(result).to.deep.equal(DataContractDefaults.SCHEMA);
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

      wasmDataContract.setDocuments(anotherDocuments);
      expect(wasmDataContract.getDocuments()).to.deep.equal(anotherDocuments);
    });
  });

  describe('#isDocumentDefined', () => {
    it('should return true if Document schema is defined', () => {
      const result = wasmDataContract.isDocumentDefined('niceDocument');

      expect(result).to.equal(true);
    });

    it('should return false if Document schema is not defined', () => {
      const result = wasmDataContract.isDocumentDefined('undefinedDocument');

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

      wasmDataContract.setDocumentSchema(anotherType, anotherDefinition);

      const documents = wasmDataContract.getDocuments();

      expect(documents).to.have.property(anotherType);
      expect(documents[anotherType]).to.deep.equal(anotherDefinition);
    });
  });

  describe('#getDocumentSchema', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        wasmDataContract.getDocumentSchema('undefinedObject');
      } catch (e) {
        error = e;
      }
      expect(error.getDocType()).to.equal('undefinedObject');
    });

    it('should return Document Schema', () => {
      const result = wasmDataContract.getDocumentSchema(documentType);

      expect(result).to.deep.equal(documentSchema);
    });
  });

  describe('#getDocumentSchemaRef', () => {
    it('should throw error if Document is not defined', () => {
      let error;
      try {
        wasmDataContract.getDocumentSchemaRef('undefinedObject');
      } catch (e) {
        error = e;
      }
      expect(error.getDocType()).to.equal('undefinedObject');
    });

    it('should return schema ref', () => {
      const result = wasmDataContract.getDocumentSchemaRef(documentType);

      expect(result).to.equal(`${wasmDataContract.getJsonSchemaId()}#/documents/niceDocument`);
    });
  });

  describe('#setDefinitions', () => {
    it('should set $defs', () => {
      const $defs = {
        subSchema: { type: 'object' },
      };

      wasmDataContract.setDefinitions($defs);

      expect(wasmDataContract.getDefinitions()).to.deep.equal($defs);
    });
  });

  describe('#getDefinitions', () => {
    it('should return $defs', () => {
      const result = wasmDataContract.getDefinitions();

      expect(result).to.deep.equal({});
    });
  });

  describe('#toJSON', () => {
    it('should return JsDataContract as plain object', () => {
      const result = wasmDataContract.toJSON();

      expect(result).to.deep.equal({
        protocolVersion: wasmDataContract.getProtocolVersion(),
        $id: bs58.encode(contractId),
        $schema: DataContractDefaults.SCHEMA,
        version: 1,
        ownerId: bs58.encode(ownerId),
        documents,
	$defs: {},
      });
    });

    it('should return plain object with "$defs" if present', () => {
      const $defs = {
        subSchema: { type: 'object' },
      };

      wasmDataContract.setDefinitions($defs);

      const result = wasmDataContract.toJSON();

      expect(result).to.deep.equal({
        protocolVersion: wasmDataContract.getProtocolVersion(),
        $schema: DataContractDefaults.SCHEMA,
        $id: bs58.encode(contractId),
        version: 1,
        ownerId: bs58.encode(ownerId),
        documents,
        $defs,
      });
    });
  });

  describe('#toBuffer', () => {
    it('should return JsDataContract as a Buffer', () => {
      const serializedJsDataContract = Buffer.from('123');

      encodeMock.returns(serializedJsDataContract);

      const result = dataContract.toBuffer();

      const dataContractToEncode = dataContract.toObject();
      delete dataContractToEncode.protocolVersion;

      const protocolVersionUInt32 = Buffer.alloc(4);
      protocolVersionUInt32.writeUInt32LE(dataContract.getProtocolVersion(), 0);

      expect(encodeMock).to.have.been.calledOnceWith(dataContractToEncode);
      expect(result).to.deep.equal(Buffer.concat([protocolVersionUInt32, serializedJsDataContract]));
    });
  });

  describe('#hash', () => {
    let toBufferMock;

    beforeEach(function beforeEach() {
      toBufferMock = this.sinonSandbox.stub(JsDataContract.prototype, 'toBuffer');
    });

    afterEach(() => {
      toBufferMock.restore();
    });

    it('should return JsDataContract hash', () => {
      const serializedJsDataContract = '123';
      const hashedDocument = '456';

      toBufferMock.returns(serializedJsDataContract);

      hashMock.returns(hashedDocument);

      const result = dataContract.hash();

      expect(result).to.equal(hashedDocument);

      expect(toBufferMock).to.have.been.calledOnce();

      expect(hashMock).to.have.been.calledOnceWith(serializedJsDataContract);
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
