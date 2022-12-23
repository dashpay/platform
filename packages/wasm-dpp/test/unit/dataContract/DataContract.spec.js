const bs58 = require('bs58');

const generateRandomIdentifier = require('@dashevo/dpp/lib/test/utils/generateRandomIdentifier');
const JsDataContract = require('@dashevo/dpp/lib/dataContract/DataContract');

const { default: loadWasmDpp } = require('../../../dist');

describe('DataContract', () => {
  let documentType;
  let documentSchema;
  let documents;
  let jsDataContract;
  let ownerId;
  let entropy;
  let contractId;
  let dataContract;
  let defs;

  let DataContract;
  let DataContractDefaults;
  let Identifier;
  let Metadata;

  before(async () => {
    ({
      DataContract, DataContractDefaults, Identifier, Metadata,
    } = await loadWasmDpp());
  });

  beforeEach(() => {
    documentType = 'niceDocument';

    documentSchema = {
      properties: {
        nice: {
          type: 'boolean',
        },
        aBinaryProperty: {
          type: 'object',
          byteArray: true,
        },
      },
    };

    documents = {
      [documentType]: documentSchema,
    };

    ownerId = generateRandomIdentifier();
    entropy = Buffer.alloc(32, 420);
    contractId = generateRandomIdentifier();

    defs = { something: { type: 'string' } };

    jsDataContract = new JsDataContract({
      $schema: JsDataContract.DEFAULTS.SCHEMA,
      $id: contractId,
      version: 1,
      protocolVersion: 1,
      ownerId,
      documents,
      $defs: defs,
    });

    dataContract = new DataContract({
      $schema: DataContractDefaults.SCHEMA,
      $id: contractId,
      version: 1,
      protocolVersion: 1,
      ownerId,
      documents,
      $defs: defs,
    });
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
        $defs: defs,
      });

      expect(dataContract.getId().toBuffer()).to.deep.equal(id.toBuffer());
      expect(dataContract.getOwnerId().toBuffer()).to.deep.equal(ownerId.toBuffer());
      expect(dataContract.getJsonMetaSchema()).to.deep.equal(DataContractDefaults.SCHEMA);
      expect(dataContract.getDocuments()).to.deep.equal(documents);
      expect(dataContract.getDefinitions()).to.deep.equal(defs);
    });
  });

  describe('#getId', () => {
    it('should return DataContract Identifier', () => {
      const result = dataContract.getId();

      expect(result.toBuffer()).to.deep.equal(contractId.toBuffer());
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

      dataContract.setJsonMetaSchema(metaSchema);

      expect(dataContract.getJsonMetaSchema()).to.deep.equal(metaSchema);
    });
  });

  describe('#getJsonMetaSchema', () => {
    it('should return meta schema', () => {
      const result = dataContract.getJsonMetaSchema();

      expect(result).to.deep.equal(DataContractDefaults.SCHEMA);
    });
  });

  describe('#getDocuments', () => {
    it('should get Documents definition', () => {
      const anotherDocuments = {
        anotherDocument: {
          properties: {
            name: { type: 'string' },
          },
        },
      };

      dataContract.setDocuments(anotherDocuments);
      expect(dataContract.getDocuments()).to.deep.equal(anotherDocuments);
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

      dataContract.setDocuments(anotherDocuments);
      expect(dataContract.getDocuments()).to.deep.equal(anotherDocuments);
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

      dataContract.setDocumentSchema(anotherType, anotherDefinition);

      const anotherDocuments = dataContract.getDocuments();

      expect(anotherDocuments).to.have.property(anotherType);
      expect(anotherDocuments[anotherType]).to.deep.equal(anotherDefinition);
    });
  });

  describe('#getDocumentSchema', () => {
    it('should throw error if Document is not defined', () => {
      try {
        dataContract.getDocumentSchema('undefinedObject');
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.getDocType()).to.equal('undefinedObject');
      }
    });

    it('should return Document Schema', () => {
      const result = dataContract.getDocumentSchema(documentType);

      expect(result).to.deep.equal(documentSchema);
    });
  });

  describe('#getDocumentSchemaRef', () => {
    it('should throw error if Document is not defined', () => {
      try {
        dataContract.getDocumentSchemaRef('undefinedObject');
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.getDocType()).to.equal('undefinedObject');
      }
    });

    it('should return schema ref', () => {
      const result = dataContract.getDocumentSchemaRef(documentType);

      expect(result).to.equal(`${dataContract.getJsonSchemaId()}#/documents/niceDocument`);
    });
  });

  describe('#setDefinitions', () => {
    it('should set $defs', () => {
      const $defs = {
        subSchema: { type: 'object' },
      };

      dataContract.setDefinitions($defs);

      expect(dataContract.getDefinitions()).to.deep.equal($defs);
    });
  });

  describe('#getDefinitions', () => {
    it('should return $defs', () => {
      const result = dataContract.getDefinitions();

      expect(result).to.deep.equal(defs);
    });
  });

  describe('#toJSON', () => {
    it('should return JataContract as plain object', () => {
      const result = dataContract.toJSON();

      expect(result).to.deep.equal({
        protocolVersion: dataContract.getProtocolVersion(),
        $id: bs58.encode(contractId),
        $schema: DataContractDefaults.SCHEMA,
        version: 1,
        ownerId: bs58.encode(ownerId),
        documents,
        $defs: defs,
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
    it('should return DataContract as a Buffer', () => {
      expect(jsDataContract.getProtocolVersion()).to.deep.equal(dataContract.getProtocolVersion());

      const jsResult = jsDataContract.toBuffer();
      const wasmResult = dataContract.toBuffer();

      expect(wasmResult).to.deep.equal(jsResult);
    });
  });

  describe('#hash', () => {
    it('should return DataContract hash', () => {
      const jsResult = jsDataContract.hash();
      const wasmResult = dataContract.hash();

      expect(wasmResult).to.deep.equal(jsResult);
    });
  });

  describe('#setEntropy', () => {
    it('should set entropy', () => {
      dataContract.setEntropy(entropy);

      expect(dataContract.getEntropy()).to.deep.equal(entropy);
    });
  });

  describe('#getBinaryProperties', () => {
    it('should return flat map of properties with `contentEncoding` keywords', () => {
      const result = dataContract.getBinaryProperties(documentType);
      expect(result).to.deep.equal({
        aBinaryProperty: {
          type: 'object',
          byteArray: true,
        },
      });
    });

    it('should throw an error if document type is not found', () => {
      try {
        dataContract.getBinaryProperties('unknown');
        expect.fail('Error was not thrown');
      } catch (e) {
        expect(e.getDocType()).to.equal('unknown');
      }
    });
  });

  describe('#setMetadata', () => {
    it('should set metadata', () => {
      const otherMetadata = new Metadata(43, 1);
      const otherMetadataToObject = otherMetadata.toObject();

      dataContract.setMetadata(otherMetadata);

      expect(dataContract.getMetadata().toObject()).to.deep.equal(otherMetadataToObject);
    });
  });
});
